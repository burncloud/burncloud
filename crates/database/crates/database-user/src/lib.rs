use burncloud_database::{Database, Result};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, Row};

/// User with dual-currency wallet for regional pricing support.
/// Balance fields use i64 nanodollars (9 decimal precision) for PostgreSQL BIGINT compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbUser {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub password_hash: Option<String>, // Nullable for OIDC users
    pub github_id: Option<String>,
    #[sqlx(default)]
    pub status: i32, // 1: Active, 0: Disabled
    /// USD balance in nanodollars (9 decimal precision)
    #[sqlx(default)]
    pub balance_usd: i64,
    /// CNY balance in nanodollars (9 decimal precision)
    #[sqlx(default)]
    pub balance_cny: i64,
    /// User's preferred currency for display
    #[sqlx(default)]
    pub preferred_currency: Option<String>,
    // created_at handled by DB
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbRole {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbUserRole {
    pub user_id: String,
    pub role_id: String,
}

/// Recharge record with dual-currency support.
/// Amount is stored as i64 nanodollars (9 decimal precision).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct DbRecharge {
    pub id: i32,
    pub user_id: String,
    /// Amount in nanodollars (9 decimal precision)
    pub amount: i64,
    /// Currency of the recharge (USD, CNY)
    #[sqlx(default)]
    pub currency: Option<String>,
    pub description: Option<String>,
    pub created_at: Option<String>, // SQL datetime string
}

pub struct UserDatabase;

impl UserDatabase {
    pub async fn init(db: &Database) -> Result<()> {
        let conn = db.get_connection()?;
        let kind = db.kind();

        // Table definitions
        // Note: balance_usd and balance_cny use BIGINT nanodollars (9 decimal precision)
        let (users_sql, roles_sql, user_roles_sql, recharges_sql) = match kind.as_str() {
            "sqlite" => (
                r#"
                CREATE TABLE IF NOT EXISTS users (
                    id TEXT PRIMARY KEY,
                    username TEXT NOT NULL UNIQUE,
                    email TEXT UNIQUE,
                    password_hash TEXT,
                    github_id TEXT,
                    status INTEGER DEFAULT 1,
                    balance_usd BIGINT DEFAULT 0,
                    balance_cny BIGINT DEFAULT 0,
                    preferred_currency VARCHAR(10) DEFAULT 'USD',
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS roles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS user_roles (
                    user_id TEXT NOT NULL,
                    role_id TEXT NOT NULL,
                    PRIMARY KEY (user_id, role_id),
                    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                    FOREIGN KEY(role_id) REFERENCES roles(id) ON DELETE CASCADE
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS recharges (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    user_id TEXT NOT NULL,
                    amount BIGINT NOT NULL,
                    currency VARCHAR(10) DEFAULT 'USD',
                    description TEXT,
                    created_at TEXT DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
                );
                "#,
            ),
            "postgres" => (
                r#"
                CREATE TABLE IF NOT EXISTS users (
                    id TEXT PRIMARY KEY,
                    username TEXT NOT NULL UNIQUE,
                    email TEXT UNIQUE,
                    password_hash TEXT,
                    github_id TEXT,
                    status INTEGER DEFAULT 1,
                    balance_usd BIGINT DEFAULT 0,
                    balance_cny BIGINT DEFAULT 0,
                    preferred_currency VARCHAR(10) DEFAULT 'USD',
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS roles (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL UNIQUE,
                    description TEXT
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS user_roles (
                    user_id TEXT NOT NULL,
                    role_id TEXT NOT NULL,
                    PRIMARY KEY (user_id, role_id),
                    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE,
                    FOREIGN KEY(role_id) REFERENCES roles(id) ON DELETE CASCADE
                );
                "#,
                r#"
                CREATE TABLE IF NOT EXISTS recharges (
                    id SERIAL PRIMARY KEY,
                    user_id TEXT NOT NULL,
                    amount BIGINT NOT NULL,
                    currency VARCHAR(10) DEFAULT 'USD',
                    description TEXT,
                    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(user_id) REFERENCES users(id) ON DELETE CASCADE
                );
                "#,
            ),
            _ => unreachable!("Unsupported database kind"),
        };

        sqlx::query(users_sql).execute(conn.pool()).await?;
        sqlx::query(roles_sql).execute(conn.pool()).await?;
        println!("UserDatabase: tables created/verified.");

        sqlx::query(user_roles_sql).execute(conn.pool()).await?;
        sqlx::query(recharges_sql).execute(conn.pool()).await?;

        // Migrations for SQLite (Add columns if missing)
        if kind == "sqlite" {
            // Ignoring errors as "duplicate column name" is the expected error if it exists
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN password_hash TEXT")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN github_id TEXT")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN status INTEGER DEFAULT 1")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN balance_usd BIGINT DEFAULT 0")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN balance_cny BIGINT DEFAULT 0")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN preferred_currency VARCHAR(10) DEFAULT 'USD'")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE recharges ADD COLUMN currency VARCHAR(10) DEFAULT 'USD'")
                .execute(conn.pool())
                .await;
        }

        // Migrations for PostgreSQL (Add columns if missing)
        if kind == "postgres" {
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS balance_usd BIGINT DEFAULT 0")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS balance_cny BIGINT DEFAULT 0")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE users ADD COLUMN IF NOT EXISTS preferred_currency VARCHAR(10) DEFAULT 'USD'")
                .execute(conn.pool())
                .await;
            let _ = sqlx::query("ALTER TABLE recharges ADD COLUMN IF NOT EXISTS currency VARCHAR(10) DEFAULT 'USD'")
                .execute(conn.pool())
                .await;
        }

        // Initialize default roles
        let role_count: i64 = sqlx::query("SELECT COUNT(*) FROM roles")
            .fetch_one(conn.pool())
            .await?
            .get(0);

        if role_count == 0 {
            println!("UserDatabase: inserting default roles...");
            sqlx::query("INSERT INTO roles (id, name, description) VALUES ('role-admin', 'admin', 'Administrator'), ('role-user', 'user', 'Standard User')")
                .execute(conn.pool())
                .await?;
        }

        // Ensure demo user exists
        let user_count: i64 =
            sqlx::query("SELECT COUNT(*) FROM users WHERE username = 'demo-user'")
                .fetch_one(conn.pool())
                .await?
                .get(0);

        if user_count == 0 {
            println!("UserDatabase: inserting demo user...");
            // Password: "123456"
            let dummy_hash = "$2a$10$N9qo8uLOickgx2ZMRZoMyeIjZAgcfl7p92ldGxad68LJZdL17lhWy";
            // Give demo user 100 USD in nanodollars (100 * 10^9 = 100_000_000_000)
            sqlx::query("INSERT INTO users (id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency) VALUES ('demo-user', 'demo-user', NULL, ?, NULL, 1, 100000000000, 0, 'USD')")
                .bind(dummy_hash)
                .execute(conn.pool())
                .await?;
            // Assign admin role
            println!("UserDatabase: assigning admin role...");
            sqlx::query(
                "INSERT INTO user_roles (user_id, role_id) VALUES ('demo-user', 'role-admin')",
            )
            .execute(conn.pool())
            .await?;
        }

        println!("UserDatabase: init complete.");
        Ok(())
    }

    pub async fn create_user(db: &Database, user: &DbUser) -> Result<()> {
        let conn = db.get_connection()?;
        sqlx::query("INSERT INTO users (id, username, email, password_hash, github_id, status, balance_usd, balance_cny, preferred_currency) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
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

    pub async fn get_user_by_username(db: &Database, username: &str) -> Result<Option<DbUser>> {
        let conn = db.get_connection()?;
        let user = sqlx::query_as::<_, DbUser>("SELECT * FROM users WHERE username = ?")
            .bind(username)
            .fetch_optional(conn.pool())
            .await?;
        Ok(user)
    }

    pub async fn get_user_roles(db: &Database, user_id: &str) -> Result<Vec<String>> {
        let conn = db.get_connection()?;
        let rows = sqlx::query("SELECT r.name FROM roles r JOIN user_roles ur ON r.id = ur.role_id WHERE ur.user_id = ?")
            .bind(user_id)
            .fetch_all(conn.pool())
            .await?;

        let roles = rows.iter().map(|r| r.get(0)).collect();
        Ok(roles)
    }

    pub async fn assign_role(db: &Database, user_id: &str, role_name: &str) -> Result<()> {
        let conn = db.get_connection()?;
        let role_id: Option<String> = sqlx::query("SELECT id FROM roles WHERE name = ?")
            .bind(role_name)
            .fetch_optional(conn.pool())
            .await?
            .map(|r| r.get(0));

        if let Some(rid) = role_id {
            let res = sqlx::query("INSERT INTO user_roles (user_id, role_id) VALUES (?, ?)")
                .bind(user_id)
                .bind(rid)
                .execute(conn.pool())
                .await;

            if let Err(e) = res {
                println!("Role assignment skipped (maybe already exists): {}", e);
            }
        }
        Ok(())
    }

    pub async fn list_users(db: &Database) -> Result<Vec<DbUser>> {
        let conn = db.get_connection()?;
        let users = sqlx::query_as::<_, DbUser>("SELECT * FROM users")
            .fetch_all(conn.pool())
            .await?;
        Ok(users)
    }

    /// Update USD balance by delta (in nanodollars)
    pub async fn update_balance_usd(db: &Database, user_id: &str, delta: i64) -> Result<i64> {
        let conn = db.get_connection()?;
        sqlx::query("UPDATE users SET balance_usd = balance_usd + ? WHERE id = ?")
            .bind(delta)
            .bind(user_id)
            .execute(conn.pool())
            .await?;

        let new_balance: i64 = sqlx::query("SELECT balance_usd FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one(conn.pool())
            .await?
            .get(0);

        Ok(new_balance)
    }

    /// Update CNY balance by delta (in nanodollars)
    pub async fn update_balance_cny(db: &Database, user_id: &str, delta: i64) -> Result<i64> {
        let conn = db.get_connection()?;
        sqlx::query("UPDATE users SET balance_cny = balance_cny + ? WHERE id = ?")
            .bind(delta)
            .bind(user_id)
            .execute(conn.pool())
            .await?;

        let new_balance: i64 = sqlx::query("SELECT balance_cny FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one(conn.pool())
            .await?
            .get(0);

        Ok(new_balance)
    }

    /// Legacy method for backward compatibility - updates USD balance
    pub async fn update_balance(db: &Database, user_id: &str, delta: f64) -> Result<f64> {
        // Convert f64 dollars to i64 nanodollars
        let delta_nano = (delta * 1_000_000_000.0) as i64;
        let new_balance = Self::update_balance_usd(db, user_id, delta_nano).await?;
        Ok(new_balance as f64 / 1_000_000_000.0)
    }

    pub async fn create_recharge(db: &Database, recharge: &DbRecharge) -> Result<i32> {
        let conn = db.get_connection()?;
        let currency = recharge.currency.as_deref().unwrap_or("USD");
        let id: i32 = match db.kind().as_str() {
            "sqlite" => {
                sqlx::query("INSERT INTO recharges (user_id, amount, currency, description) VALUES (?, ?, ?, ?)")
                    .bind(&recharge.user_id)
                    .bind(recharge.amount)
                    .bind(currency)
                    .bind(&recharge.description)
                    .execute(conn.pool())
                    .await?
                    .last_insert_id()
                    .unwrap_or(0) as i32
            }
            "postgres" => {
                sqlx::query("INSERT INTO recharges (user_id, amount, currency, description) VALUES ($1, $2, $3, $4) RETURNING id")
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

    pub async fn list_recharges(db: &Database, user_id: &str) -> Result<Vec<DbRecharge>> {
        let conn = db.get_connection()?;
        let recharges = sqlx::query_as::<_, DbRecharge>(
            "SELECT * FROM recharges WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(conn.pool())
        .await?;
        Ok(recharges)
    }
}
