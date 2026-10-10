//! Database operations for user_ domain (accounts, roles, bindings, recharges, API keys).
//!
//! The spec-aligned entity layout is split across per-entity files:
//! - `user_account.rs`: `UserAccount`, `UserAccountInput`
//! - `user_recharge.rs`: `UserRecharge`
//! - `user_api_key.rs`: `UserApiKey`, `UserApiKeyModel`, `UserApiKeyInput`, `UserApiKeyUpdateInput`
//!
//! `UserDatabase` is the crate-level controller (initialises sub-tables, seeds default roles,
//! and contains operation-style helpers). `UserAccountModel` is exposed as a spec-aligned alias
//! so callers can reference it under the canonical `{Domain}{Entity}Model` naming.

mod common;
mod password_reset;
mod user_account;
mod user_api_key;
mod user_recharge;

pub use password_reset::{PasswordResetDatabase, PasswordResetToken};
pub use user_account::{UserAccount, UserAccountInput};
pub use user_api_key::{UserApiKey, UserApiKeyInput, UserApiKeyModel, UserApiKeyUpdateInput};
pub use user_recharge::UserRecharge;

use burncloud_database::{Database, Result};
use sqlx::Row;

pub struct UserDatabase;

/// Spec-aligned alias: operation type for the `user_accounts` table.
pub type UserAccountModel = UserDatabase;

/// Run a migration statement whose failure must not abort initialization.
///
/// `ALTER TABLE ... ADD COLUMN` fails with "duplicate column name" once the
/// column exists, and the SQLite-only `INSERT OR IGNORE` seed is a no-op on
/// other dialects, so both are expected results on a re-run and are logged at
/// debug level instead of being propagated.
async fn best_effort_execute(pool: &sqlx::AnyPool, sql: &str) {
    if let Err(e) = sqlx::query(sql).execute(pool).await {
        tracing::debug!(
            "UserDatabase: best-effort statement skipped: {} ({})",
            e,
            sql
        );
    }
}

impl UserDatabase {
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        let kind = db.kind();

        Self::create_tables(conn.pool(), &kind).await?;

        // Migrations (add columns if missing)
        if kind == "sqlite" {
            Self::migrate_sqlite_columns(conn.pool()).await;
        }
        if kind == "postgres" {
            Self::migrate_postgres_columns(conn.pool()).await;
        }

        Self::seed_default_roles(conn.pool()).await?;

        // Migration: assign roles to users that were created before the role
        // system existed (they have no entries in user_role_bindings).
        Self::assign_orphan_roles(db, conn.pool()).await?;

        tracing::info!("UserDatabase: init complete.");
        Ok(())
    }

    /// Seed the Identity-owned demo account and demo API key (#842).
    ///
    /// `user_accounts` and `user_api_keys` are Identity's Data Truth, so the default records
    /// written into them belong here rather than in `platform/storage/database`. Platform
    /// previously issued both INSERTs on every database initialization; the SQL is moved
    /// verbatim, including the PostgreSQL `ON CONFLICT DO NOTHING` / SQLite `INSERT OR IGNORE`
    /// split and the historical fallback from `user_accounts` to the legacy `users` table, so
    /// the seeded rows are unchanged.
    ///
    /// Both statements are idempotent: re-running on an existing database inserts nothing and
    /// never overwrites a row that has since been edited. The demo account is written before
    /// the demo key because the key references `user_id = 'demo-user'`.
    ///
    /// Ordering relative to schema creation is the caller's responsibility: the application
    /// bootstrap runs this after infrastructure initialization, which is what creates the
    /// tables.
    pub async fn seed_demo_defaults(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        let pool = conn.pool();
        let kind = db.kind();

        Self::seed_demo_user(pool, &kind).await?;
        Self::seed_demo_token(pool, &kind).await?;

        Ok(())
    }

    /// Ensure the demo account exists.
    ///
    /// Required by the `validate_token_and_get_info` JOIN, which resolves the demo API key
    /// against `user_accounts`.
    async fn seed_demo_user(pool: &sqlx::AnyPool, kind: &str) -> Result<()> {
        // PostgreSQL does not implement SQLite's INSERT OR IGNORE syntax. Preserve the
        // existing fallback from the canonical to the legacy user table.
        let insert = if kind == "postgres" {
            "INSERT INTO user_accounts (id, username, password_hash, status) \
             VALUES ('demo-user', 'demo-user', 'no-login', 1) ON CONFLICT DO NOTHING"
        } else {
            "INSERT OR IGNORE INTO user_accounts (id, username, password_hash, status) \
             VALUES ('demo-user', 'demo-user', 'no-login', 1)"
        };
        if sqlx::query(insert).execute(pool).await.is_err() {
            let fallback = if kind == "postgres" {
                "INSERT INTO users (id, username, password_hash, status) \
                 VALUES ('demo-user', 'demo-user', 'no-login', 1) ON CONFLICT DO NOTHING"
            } else {
                "INSERT OR IGNORE INTO users (id, username, password_hash, status) \
                 VALUES ('demo-user', 'demo-user', 'no-login', 1)"
            };
            sqlx::query(fallback).execute(pool).await?;
        }
        Ok(())
    }

    /// Insert the default demo token if it does not yet exist.
    async fn seed_demo_token(pool: &sqlx::AnyPool, kind: &str) -> Result<()> {
        let t_count: i64 = match sqlx::query_scalar(
            "SELECT count(*) FROM user_api_keys WHERE key = 'sk-burncloud-demo'",
        )
        .fetch_one(pool)
        .await
        {
            Ok(n) => n,
            // A missing table means there is nothing to seed yet; this matches the previous
            // tolerance during early bootstrap.
            Err(_) => return Ok(()),
        };

        if t_count != 0 {
            return Ok(());
        }

        let now = common::current_timestamp();
        let insert_sql = match kind {
            "sqlite" => {
                "INSERT INTO user_api_keys \
                 (user_id, key, status, name, remain_quota, unlimited_quota, \
                  used_quota, created_time, accessed_time, expired_time) \
                 VALUES ('demo-user', 'sk-burncloud-demo', 1, 'Demo Token', \
                         -1, 1, 0, ?, ?, -1)"
            }
            "postgres" => {
                "INSERT INTO user_api_keys \
                 (user_id, key, status, name, remain_quota, unlimited_quota, \
                  used_quota, created_time, accessed_time, expired_time) \
                 VALUES ('demo-user', 'sk-burncloud-demo', 1, 'Demo Token', \
                         -1, 1, 0, $1, $2, -1)"
            }
            _ => return Ok(()),
        };

        sqlx::query(insert_sql)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
        tracing::info!("Initialized demo token: sk-burncloud-demo");
        Ok(())
    }

    /// Create the `user_*` tables owned by this crate.
    ///
    /// Note: balance_usd and balance_cny use BIGINT nanodollars (9 decimal precision).
    /// Note: user_accounts CREATE TABLE is authoritative in schema.rs; omitted here.
    async fn create_tables(pool: &sqlx::AnyPool, kind: &str) -> Result<()> {
        let (user_roles_sql, user_role_bindings_sql, user_recharges_sql) = match kind {
            "sqlite" => (
                r#"
                CREATE TABLE IF NOT EXISTS user_roles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS user_role_bindings (
                    user_id TEXT NOT NULL,
                    role_id TEXT NOT NULL,
                    PRIMARY KEY (user_id, role_id),
                    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE,
                    FOREIGN KEY(role_id) REFERENCES user_roles(id) ON DELETE CASCADE
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS user_recharges (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id TEXT NOT NULL,
                    amount BIGINT NOT NULL,
                    currency VARCHAR(10) DEFAULT 'USD',
                    description TEXT,
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE
                );
                "#,
            ),
            "postgres" => (
                r#"
                CREATE TABLE IF NOT EXISTS user_roles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS user_role_bindings (
                    user_id TEXT NOT NULL,
                    role_id TEXT NOT NULL,
                    PRIMARY KEY (user_id, role_id),
                    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE,
                    FOREIGN KEY(role_id) REFERENCES user_roles(id) ON DELETE CASCADE
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS user_recharges (
                    id SERIAL PRIMARY KEY,
                    user_id TEXT NOT NULL,
                    amount BIGINT NOT NULL,
                    currency VARCHAR(10) DEFAULT 'USD',
                    description TEXT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(user_id) REFERENCES user_accounts(id) ON DELETE CASCADE
                );
                "#,
            ),
            _ => unreachable!("Unsupported database kind"),
        };

        sqlx::query(user_roles_sql).execute(pool).await?;
        tracing::info!("UserDatabase: tables created/verified.");

        sqlx::query(user_role_bindings_sql).execute(pool).await?;
        sqlx::query(user_recharges_sql).execute(pool).await?;

        Ok(())
    }

    /// SQLite migrations (add columns if missing).
    ///
    /// Best-effort: "duplicate column name" is the expected error if the column
    /// already exists.
    async fn migrate_sqlite_columns(pool: &sqlx::AnyPool) {
        best_effort_execute(
            pool,
            "ALTER TABLE user_accounts ADD COLUMN password_hash TEXT",
        )
        .await;
        best_effort_execute(pool, "ALTER TABLE user_accounts ADD COLUMN github_id TEXT").await;
        best_effort_execute(
            pool,
            "ALTER TABLE user_accounts ADD COLUMN status INTEGER DEFAULT 1",
        )
        .await;
        best_effort_execute(
            pool,
            "ALTER TABLE user_accounts ADD COLUMN balance_usd BIGINT DEFAULT 0",
        )
        .await;
        best_effort_execute(
            pool,
            "ALTER TABLE user_accounts ADD COLUMN balance_cny BIGINT DEFAULT 0",
        )
        .await;
        best_effort_execute(
            pool,
            "ALTER TABLE user_accounts ADD COLUMN preferred_currency VARCHAR(10) DEFAULT 'USD'",
        )
        .await;
        best_effort_execute(
            pool,
            "ALTER TABLE user_recharges ADD COLUMN currency VARCHAR(10) DEFAULT 'USD'",
        )
        .await;
    }

    /// PostgreSQL migrations (add columns if missing). Best-effort, see
    /// [`best_effort_execute`].
    async fn migrate_postgres_columns(pool: &sqlx::AnyPool) {
        best_effort_execute(
            pool,
            "ALTER TABLE user_accounts ADD COLUMN IF NOT EXISTS balance_usd BIGINT DEFAULT 0",
        )
        .await;
        best_effort_execute(
            pool,
            "ALTER TABLE user_accounts ADD COLUMN IF NOT EXISTS balance_cny BIGINT DEFAULT 0",
        )
        .await;
        best_effort_execute(
            pool,
            "ALTER TABLE user_accounts ADD COLUMN IF NOT EXISTS preferred_currency VARCHAR(10) DEFAULT 'USD'",
        )
        .await;
        best_effort_execute(
            pool,
            "ALTER TABLE user_recharges ADD COLUMN IF NOT EXISTS currency VARCHAR(10) DEFAULT 'USD'",
        )
        .await;
    }

    /// Insert the default `admin` / `user` roles when the table is still empty.
    async fn seed_default_roles(pool: &sqlx::AnyPool) -> Result<()> {
        let role_count: i64 = sqlx::query("SELECT COUNT(*) FROM user_roles")
            .fetch_one(pool)
            .await?
            .get(0);

        if role_count == 0 {
            tracing::info!("UserDatabase: inserting default roles...");
            best_effort_execute(pool, "INSERT OR IGNORE INTO user_roles (id, name, description) VALUES ('role-admin', 'admin', 'Administrator'), ('role-user', 'user', 'Standard User')").await;
        }

        Ok(())
    }

    /// Assign roles to users created before the role system existed (they have
    /// no entries in user_role_bindings).
    ///
    /// The demo-user seed (password_hash = 'no-login) is always assigned "user"
    /// — it cannot log in and should never be admin. The first real (non-seed)
    /// orphan becomes admin when no other orphan was promoted yet.
    async fn assign_orphan_roles(db: &Database, pool: &sqlx::AnyPool) -> Result<()> {
        let orphan_sql = if db.kind() == "postgres" {
            "SELECT u.id, u.username, u.password_hash FROM user_accounts u WHERE NOT EXISTS (SELECT 1 FROM user_role_bindings urb WHERE urb.user_id = u.id) ORDER BY u.id"
        } else {
            "SELECT u.id, u.username, u.password_hash FROM user_accounts u WHERE NOT EXISTS (SELECT 1 FROM user_role_bindings urb WHERE urb.user_id = u.id) ORDER BY u.rowid"
        };
        let orphan_rows = sqlx::query(orphan_sql).fetch_all(pool).await?;

        if orphan_rows.is_empty() {
            return Ok(());
        }

        tracing::info!(
            "UserDatabase: assigning roles to {} orphan user(s)",
            orphan_rows.len()
        );
        // First real orphan (non-seed) gets admin; all others get user.
        let mut first_real = true;
        for row in orphan_rows.iter() {
            Self::assign_orphan_role(db, row, &mut first_real).await;
        }

        Ok(())
    }

    /// Give one orphan user a role: the first real (non-seed) orphan is promoted
    /// to `admin`, every other orphan becomes `user`.
    async fn assign_orphan_role(db: &Database, row: &sqlx::any::AnyRow, first_real: &mut bool) {
        let user_id: String = row.get(0);
        let username: String = row.get(1);
        let pw_hash: Option<String> = row.get(2);
        let is_seed = username == "demo-user" || pw_hash.as_deref() == Some("no-login");
        let role = if !is_seed && *first_real {
            *first_real = false;
            "admin"
        } else {
            "user"
        };
        if let Err(e) = Self::assign_role(db, &user_id, role).await {
            tracing::warn!(
                "Failed to assign {} role to orphan user {}: {}",
                role,
                user_id,
                e
            );
        }
    }

    pub async fn create_user(db: &Database, user: &UserAccount) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "INSERT INTO user_accounts (id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        } else {
            "INSERT INTO user_accounts (id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        };
        sqlx::query(sql)
            .bind(&user.id)
            .bind(&user.username)
            .bind(&user.email)
            .bind(&user.password_hash)
            .bind(&user.github_id)
            .bind(user.status)
            .bind(user.balance_usd)
            .bind(user.balance_cny)
            .bind(&user.preferred_currency)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn get_user_by_username(
        db: &Database,
        username: &str,
    ) -> Result<Option<UserAccount>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency FROM user_accounts WHERE username = $1"
        } else {
            "SELECT id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency FROM user_accounts WHERE username = ?"
        };
        let user = sqlx::query_as::<_, UserAccount>(sql)
            .bind(username)
            .fetch_optional(conn.pool())
            .await?;
        Ok(user)
    }

    pub async fn get_user_roles(db: &Database, user_id: &str) -> Result<Vec<String>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT r.name FROM user_roles r JOIN user_role_bindings ur ON r.id = ur.role_id WHERE ur.user_id = $1"
        } else {
            "SELECT r.name FROM user_roles r JOIN user_role_bindings ur ON r.id = ur.role_id WHERE ur.user_id = ?"
        };
        let rows = sqlx::query(sql)
            .bind(user_id)
            .fetch_all(conn.pool())
            .await?;

        let roles = rows.iter().map(|r| r.get(0)).collect();
        Ok(roles)
    }

    pub async fn assign_role(db: &Database, user_id: &str, role_name: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let (select_sql, insert_sql) = if db.kind() == "postgres" {
            (
                "SELECT id FROM user_roles WHERE name = $1",
                "INSERT INTO user_role_bindings (user_id, role_id) VALUES ($1, $2)",
            )
        } else {
            (
                "SELECT id FROM user_roles WHERE name = ?",
                "INSERT INTO user_role_bindings (user_id, role_id) VALUES (?, ?)",
            )
        };
        let role_id: Option<String> = sqlx::query(select_sql)
            .bind(role_name)
            .fetch_optional(conn.pool())
            .await?
            .map(|r| r.get(0));

        if let Some(rid) = role_id {
            let res = sqlx::query(insert_sql)
                .bind(user_id)
                .bind(rid)
                .execute(conn.pool())
                .await;

            if let Err(e) = res {
                tracing::warn!("Role assignment skipped (maybe already exists): {}", e);
            }
        }
        Ok(())
    }

    pub async fn list_users(db: &Database) -> Result<Vec<UserAccount>> {
        let conn = db.get_connection()?;
        let users = sqlx::query_as::<_, UserAccount>(
            "SELECT id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency FROM user_accounts"
        )
            .fetch_all(conn.pool())
            .await?;
        Ok(users)
    }

    pub async fn update_password_hash(
        db: &Database,
        user_id: &str,
        password_hash: &str,
    ) -> Result<()> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "UPDATE user_accounts SET password_hash = $1 WHERE id = $2"
        } else {
            "UPDATE user_accounts SET password_hash = ? WHERE id = ?"
        };
        sqlx::query(sql)
            .bind(password_hash)
            .bind(user_id)
            .execute(conn.pool())
            .await?;
        Ok(())
    }

    pub async fn get_user_by_id(db: &Database, user_id: &str) -> Result<Option<UserAccount>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency FROM user_accounts WHERE id = $1"
        } else {
            "SELECT id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency FROM user_accounts WHERE id = ?"
        };
        let user = sqlx::query_as::<_, UserAccount>(sql)
            .bind(user_id)
            .fetch_optional(conn.pool())
            .await?;
        Ok(user)
    }

    pub async fn get_user_by_email(db: &Database, email: &str) -> Result<Option<UserAccount>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency FROM user_accounts WHERE email = $1"
        } else {
            "SELECT id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency FROM user_accounts WHERE email = ?"
        };
        let user = sqlx::query_as::<_, UserAccount>(sql)
            .bind(email)
            .fetch_optional(conn.pool())
            .await?;
        Ok(user)
    }

    /// Count real users (excludes seed/demo accounts). Used by
    /// first-user-is-admin check to avoid counting the demo-user seed.
    /// Filters by username (not id) because the demo-user seed row has
    /// a generated `usr_` prefixed id, not the literal 'demo-user'.
    pub async fn count_users(db: &Database) -> Result<i64> {
        let conn = db.get_connection()?;
        let sql = "SELECT COUNT(*) FROM user_accounts WHERE username != 'demo-user'";
        let count: i64 = sqlx::query(sql).fetch_one(conn.pool()).await?.get(0);
        Ok(count)
    }

    /// Check whether any real user (excluding seed/demo accounts) has the
    /// admin role. Used by first-user-is-admin logic: if no admin exists,
    /// the next registrant is promoted so the system always has at least
    /// one admin.
    ///
    /// Excludes: (1) username = 'demo-user' (seed row), (2) password_hash =
    /// 'no-login' (seed placeholder — cannot log in, so not a real admin).
    pub async fn has_admin_user(db: &Database) -> Result<bool> {
        let conn = db.get_connection()?;
        let sql = "SELECT COUNT(*) FROM user_role_bindings urb JOIN user_roles r ON urb.role_id = r.id JOIN user_accounts u ON urb.user_id = u.id WHERE r.name = 'admin' AND u.username != 'demo-user' AND (u.password_hash IS NULL OR u.password_hash != 'no-login')";
        let count: i64 = sqlx::query(sql).fetch_one(conn.pool()).await?.get(0);
        Ok(count > 0)
    }

    /// Update USD balance by delta (in nanodollars)
    pub async fn update_balance_usd(db: &Database, user_id: &str, delta: i64) -> Result<i64> {
        let conn = db.get_connection()?;
        let (update_sql, select_sql) = if db.kind() == "postgres" {
            (
                "UPDATE user_accounts SET balance_usd = balance_usd + $1 WHERE id = $2",
                "SELECT balance_usd FROM user_accounts WHERE id = $1",
            )
        } else {
            (
                "UPDATE user_accounts SET balance_usd = balance_usd + ? WHERE id = ?",
                "SELECT balance_usd FROM user_accounts WHERE id = ?",
            )
        };
        sqlx::query(update_sql)
            .bind(delta)
            .bind(user_id)
            .execute(conn.pool())
            .await?;

        let new_balance: i64 = sqlx::query(select_sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await?
            .get(0);

        Ok(new_balance)
    }

    /// Update CNY balance by delta (in nanodollars)
    pub async fn update_balance_cny(db: &Database, user_id: &str, delta: i64) -> Result<i64> {
        let conn = db.get_connection()?;
        let (update_sql, select_sql) = if db.kind() == "postgres" {
            (
                "UPDATE user_accounts SET balance_cny = balance_cny + $1 WHERE id = $2",
                "SELECT balance_cny FROM user_accounts WHERE id = $1",
            )
        } else {
            (
                "UPDATE user_accounts SET balance_cny = balance_cny + ? WHERE id = ?",
                "SELECT balance_cny FROM user_accounts WHERE id = ?",
            )
        };
        sqlx::query(update_sql)
            .bind(delta)
            .bind(user_id)
            .execute(conn.pool())
            .await?;

        let new_balance: i64 = sqlx::query(select_sql)
            .bind(user_id)
            .fetch_one(conn.pool())
            .await?
            .get(0);

        Ok(new_balance)
    }

    /// Update balance by delta in nanodollars (generic currency-aware method)
    /// Defaults to USD if currency is not specified
    pub async fn update_balance(
        db: &Database,
        user_id: &str,
        delta_nano: i64,
        currency: Option<&str>,
    ) -> Result<i64> {
        match currency {
            Some("CNY") => Self::update_balance_cny(db, user_id, delta_nano).await,
            _ => Self::update_balance_usd(db, user_id, delta_nano).await,
        }
    }

    pub async fn create_recharge(db: &Database, recharge: &UserRecharge) -> Result<i32> {
        let conn = db.get_connection()?;
        let currency = recharge.currency.as_deref().unwrap_or("USD");
        let id: i32 = match db.kind().as_str() {
            "sqlite" => {
                sqlx::query("INSERT INTO user_recharges (user_id, amount, currency, description) VALUES (?, ?, ?, ?)")
                    .bind(&recharge.user_id)
                    .bind(recharge.amount)
                    .bind(currency)
                    .bind(&recharge.description)
                    .execute(conn.pool())
                    .await?;
                let id: i32 = sqlx::query_scalar("SELECT last_insert_rowid()")
                    .fetch_one(conn.pool())
                    .await?;
                id
            }
            "postgres" => {
                sqlx::query("INSERT INTO user_recharges (user_id, amount, currency, description) VALUES ($1, $2, $3, $4) RETURNING id")
                    .bind(&recharge.user_id)
                    .bind(recharge.amount)
                    .bind(currency)
                    .bind(&recharge.description)
                    .fetch_one(conn.pool())
                    .await?
                    .get(0)
            }
            _ => unreachable!(),
        };

        // Also update user balance based on currency
        if currency == "CNY" {
            Self::update_balance_cny(db, &recharge.user_id, recharge.amount).await?;
        } else {
            Self::update_balance_usd(db, &recharge.user_id, recharge.amount).await?;
        }

        Ok(id)
    }

    pub async fn list_recharges(db: &Database, user_id: &str) -> Result<Vec<UserRecharge>> {
        let conn = db.get_connection()?;
        let sql = if db.kind() == "postgres" {
            "SELECT * FROM user_recharges WHERE user_id = $1 ORDER BY created_at DESC"
        } else {
            "SELECT * FROM user_recharges WHERE user_id = ? ORDER BY created_at DESC"
        };
        let recharges = sqlx::query_as::<_, UserRecharge>(sql)
            .bind(user_id)
            .fetch_all(conn.pool())
            .await?;
        Ok(recharges)
    }
}
