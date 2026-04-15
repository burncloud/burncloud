//! User, token and protocol-config data migrations and seed data.
//!
//! Handles:
//! - Migrating `tokens.unlimited_quota` from BOOLEAN to INTEGER (SQLite only)
//! - Migrating legacy `quota` column values to `balance_usd`
//! - Seeding the demo user, demo token and default protocol configs

use crate::Result;
use sqlx::AnyPool;

/// Run all user/token data migrations then seed initial records.
pub(super) async fn migrate_users_and_seed(pool: &AnyPool, kind: &str) -> Result<()> {
    migrate_tokens_unlimited_quota(pool, kind).await?;
    migrate_quota_to_balance(pool).await?;
    seed_demo_user(pool).await?;
    seed_demo_token(pool, kind).await?;
    seed_protocol_configs(pool, kind).await?;
    Ok(())
}

/// Migrate `tokens.unlimited_quota` from BOOLEAN to INTEGER (SQLite only).
///
/// The sqlx Any driver expects INTEGER for this column; BOOLEAN causes binding
/// failures.  SQLite does not support ALTER COLUMN, so the table is recreated.
async fn migrate_tokens_unlimited_quota(pool: &AnyPool, kind: &str) -> Result<()> {
    if kind != "sqlite" {
        return Ok(());
    }

    let table_exists: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='tokens'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0)
        > 0;

    let needs_migration = if table_exists {
        let col_type: Option<String> = sqlx::query_scalar(
            "SELECT type FROM pragma_table_info('tokens') WHERE name='unlimited_quota'",
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        col_type.as_ref().map(|t| t == "boolean").unwrap_or(false)
    } else {
        false
    };

    if !needs_migration {
        return Ok(());
    }

    println!("Migrating tokens.unlimited_quota from BOOLEAN to INTEGER...");

    let _ = sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tokens_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            key CHAR(48) NOT NULL,
            status INTEGER DEFAULT 1,
            name VARCHAR(255),
            remain_quota INTEGER DEFAULT 0,
            unlimited_quota INTEGER DEFAULT 0,
            used_quota INTEGER DEFAULT 0,
            created_time INTEGER,
            accessed_time INTEGER,
            expired_time INTEGER DEFAULT -1
        )
        "#,
    )
    .execute(pool)
    .await;

    let copy_result = sqlx::query(
        r#"
        INSERT INTO tokens_new
        SELECT id, user_id, key, status, name, remain_quota,
               CASE WHEN unlimited_quota = 1 OR unlimited_quota = 'true' THEN 1 ELSE 0 END,
               used_quota, created_time, accessed_time, expired_time
        FROM tokens
        "#,
    )
    .execute(pool)
    .await;

    if copy_result.is_err() {
        // Fallback: drop and recreate with correct schema
        let _ = sqlx::query("DROP TABLE IF EXISTS tokens_new")
            .execute(pool)
            .await;
        let _ = sqlx::query("DROP TABLE IF EXISTS tokens")
            .execute(pool)
            .await;
        let _ = sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tokens (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id TEXT NOT NULL,
                key CHAR(48) NOT NULL,
                status INTEGER DEFAULT 1,
                name VARCHAR(255),
                remain_quota INTEGER DEFAULT 0,
                unlimited_quota INTEGER DEFAULT 0,
                used_quota INTEGER DEFAULT 0,
                created_time INTEGER,
                accessed_time INTEGER,
                expired_time INTEGER DEFAULT -1
            )
            "#,
        )
        .execute(pool)
        .await;
    } else {
        let _ = sqlx::query("DROP TABLE IF EXISTS tokens")
            .execute(pool)
            .await;
        let _ = sqlx::query("ALTER TABLE tokens_new RENAME TO tokens")
            .execute(pool)
            .await;
    }

    let _ =
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tokens_key ON tokens(key)")
            .execute(pool)
            .await;
    let _ =
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_tokens_user_id ON tokens(user_id)")
            .execute(pool)
            .await;

    println!("  Migrated tokens table schema");
    Ok(())
}

/// Migrate existing `quota` column values to `balance_usd`.
///
/// Conversion ratio: 500 000 quota = $1 = 1_000_000_000 nanodollars
/// → balance_usd = (quota - used_quota) * 2000
///
/// Only runs for users whose `balance_usd` is still 0.
async fn migrate_quota_to_balance(pool: &AnyPool) -> Result<()> {
    let _ = sqlx::query(
        "UPDATE users SET balance_usd = (quota - used_quota) * 2000 \
         WHERE balance_usd = 0 AND quota > used_quota",
    )
    .execute(pool)
    .await;
    Ok(())
}

/// Ensure the demo user exists (required by validate_token_and_get_info JOIN).
async fn seed_demo_user(pool: &AnyPool) -> Result<()> {
    // Try user_accounts (canonical name) first, fall back to users (legacy) if absent.
    let inserted = sqlx::query(
        "INSERT OR IGNORE INTO user_accounts (id, username, password_hash, status) \
         VALUES ('demo-user', 'demo-user', 'no-login', 1)",
    )
    .execute(pool)
    .await;
    if inserted.is_err() {
        // Fallback for legacy installs where table hasn't been renamed yet.
        let _ = sqlx::query(
            "INSERT OR IGNORE INTO users (id, username, password_hash, status) \
             VALUES ('demo-user', 'demo-user', 'no-login', 1)",
        )
        .execute(pool)
        .await;
    }
    Ok(())
}

/// Insert the default demo token if it does not yet exist.
async fn seed_demo_token(pool: &AnyPool, kind: &str) -> Result<()> {
    // If the user_api_keys table doesn't exist yet (fresh database before migrations run),
    // skip seeding and return Ok — the table will be created by MigrationRunner later.
    let t_count: i64 =
        match sqlx::query_scalar("SELECT count(*) FROM user_api_keys WHERE key = 'sk-burncloud-demo'")
            .fetch_one(pool)
            .await
        {
            Ok(n) => n,
            Err(_) => return Ok(()), // table absent — skip seeding
        };

    if t_count != 0 {
        return Ok(());
    }

    let now = crate::schema::current_timestamp();
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
    println!("Initialized demo token: sk-burncloud-demo");
    Ok(())
}

/// Insert the four default protocol configs if the table is empty.
async fn seed_protocol_configs(pool: &AnyPool, kind: &str) -> Result<()> {
    // Skip seeding if the table doesn't exist yet (fresh database before migrations run).
    let pc_count: i64 = match sqlx::query_scalar("SELECT count(*) FROM channel_protocol_configs")
        .fetch_one(pool)
        .await
    {
        Ok(n) => n,
        Err(_) => return Ok(()), // table absent — skip seeding
    };

    if pc_count != 0 {
        return Ok(());
    }

    let now = crate::schema::current_timestamp();

    type ProtocolConfig<'a> = (
        i32,
        &'a str,
        bool,
        Option<&'a str>,
        Option<&'a str>,
        Option<&'a str>,
    );

    let default_protocols: [ProtocolConfig; 4] = [
        (
            1,
            "default",
            true,
            Some("/v1/chat/completions"),
            Some("/v1/embeddings"),
            Some("/v1/models"),
        ),
        (2, "2023-06-01", true, Some("/v1/messages"), None, None),
        (
            3,
            "2024-02-01",
            true,
            Some("/deployments/{deployment_id}/chat/completions"),
            Some("/deployments/{deployment_id}/embeddings"),
            Some("/deployments?api-version=2024-02-01"),
        ),
        (
            4,
            "v1",
            true,
            Some("/v1/models/{model}:generateContent"),
            Some("/v1/models/{model}:embedContent"),
            Some("/v1/models"),
        ),
    ];

    let insert_sql = match kind {
        "sqlite" => {
            "INSERT INTO channel_protocol_configs \
             (channel_type, api_version, is_default, chat_endpoint, \
              embed_endpoint, models_endpoint, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
        }
        "postgres" => {
            "INSERT INTO channel_protocol_configs \
             (channel_type, api_version, is_default, chat_endpoint, \
              embed_endpoint, models_endpoint, created_at, updated_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"
        }
        _ => return Ok(()),
    };

    for (
        channel_type,
        api_version,
        is_default,
        chat_endpoint,
        embed_endpoint,
        models_endpoint,
    ) in default_protocols
    {
        sqlx::query(insert_sql)
            .bind(channel_type)
            .bind(api_version)
            .bind(is_default)
            .bind(chat_endpoint)
            .bind(embed_endpoint)
            .bind(models_endpoint)
            .bind(now)
            .bind(now)
            .execute(pool)
            .await?;
    }
    println!(
        "Initialized default protocol configs for {} channel types",
        default_protocols.len()
    );
    Ok(())
}
