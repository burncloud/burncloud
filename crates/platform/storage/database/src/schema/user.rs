//! User, token and protocol-config data migrations and seed data.
//!
//! Handles:
//! - Migrating `user_api_keys.unlimited_quota` from BOOLEAN to INTEGER (SQLite only)
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

async fn sqlite_table_exists(pool: &AnyPool, table: &str) -> Result<bool> {
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
    )
    .bind(table)
    .fetch_one(pool)
    .await?;
    Ok(count > 0)
}

async fn token_table_names(pool: &AnyPool) -> Result<Option<(&'static str, &'static str, &'static str)>> {
    if sqlite_table_exists(pool, "user_api_keys").await? {
        return Ok(Some((
            "user_api_keys",
            "idx_user_api_keys_key",
            "idx_user_api_keys_user_id",
        )));
    }
    if sqlite_table_exists(pool, "tokens").await? {
        return Ok(Some(("tokens", "idx_tokens_key", "idx_tokens_user_id")));
    }
    Ok(None)
}

/// Migrate `user_api_keys.unlimited_quota` from BOOLEAN to INTEGER (SQLite only).
async fn migrate_tokens_unlimited_quota(pool: &AnyPool, kind: &str) -> Result<()> {
    if kind != "sqlite" {
        return Ok(());
    }

    let Some((table_name, index_key, index_user)) = token_table_names(pool).await? else {
        return Ok(());
    };

    let col_type: Option<String> = sqlx::query_scalar(&format!(
        "SELECT type FROM pragma_table_info('{table_name}') WHERE name='unlimited_quota'"
    ))
    .fetch_optional(pool)
    .await?;
    if col_type.as_deref() != Some("boolean") {
        return Ok(());
    }

    tracing::info!("Migrating {table_name}.unlimited_quota from BOOLEAN to INTEGER...");
    rebuild_token_table(pool, table_name).await?;
    create_token_indexes(pool, table_name, index_key, index_user).await?;
    tracing::info!("  Migrated {table_name} table schema");
    Ok(())
}

async fn rebuild_token_table(pool: &AnyPool, table_name: &str) -> Result<()> {
    let tmp = format!("{table_name}_new");
    sqlx::query(&format!(
        r#"
        CREATE TABLE IF NOT EXISTS {tmp} (
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
        "#
    ))
    .execute(pool)
    .await?;

    let copy_result = sqlx::query(&format!(
        r#"
        INSERT INTO {tmp}
        SELECT id, user_id, key, status, name, remain_quota,
               CASE WHEN unlimited_quota = 1 OR unlimited_quota = 'true' THEN 1 ELSE 0 END,
               used_quota, created_time, accessed_time, expired_time
        FROM {table_name}
        "#
    ))
    .execute(pool)
    .await;

    if copy_result.is_err() {
        sqlx::query(&format!("DROP TABLE IF EXISTS {tmp}"))
            .execute(pool)
            .await?;
        sqlx::query(&format!("DROP TABLE IF EXISTS {table_name}"))
            .execute(pool)
            .await?;
        sqlx::query(&format!(
            r#"
            CREATE TABLE IF NOT EXISTS {table_name} (
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
            "#
        ))
        .execute(pool)
        .await?;
    } else {
        sqlx::query(&format!("DROP TABLE IF EXISTS {table_name}"))
            .execute(pool)
            .await?;
        sqlx::query(&format!("ALTER TABLE {tmp} RENAME TO {table_name}"))
            .execute(pool)
            .await?;
    }
    Ok(())
}

async fn create_token_indexes(
    pool: &AnyPool,
    table_name: &str,
    index_key: &str,
    index_user: &str,
) -> Result<()> {
    sqlx::query(&format!(
        "CREATE INDEX IF NOT EXISTS {index_key} ON {table_name}(key)"
    ))
    .execute(pool)
    .await?;
    sqlx::query(&format!(
        "CREATE INDEX IF NOT EXISTS {index_user} ON {table_name}(user_id)"
    ))
    .execute(pool)
    .await?;
    Ok(())
}

/// Migrate existing `quota` column values to `balance_usd`.
async fn migrate_quota_to_balance(pool: &AnyPool) -> Result<()> {
    let canonical = sqlx::query(
        "UPDATE user_accounts SET balance_usd = (quota - used_quota) * 2000 \
         WHERE balance_usd = 0 AND quota > used_quota",
    )
    .execute(pool)
    .await;
    if canonical.is_err() {
        sqlx::query(
            "UPDATE users SET balance_usd = (quota - used_quota) * 2000 \
             WHERE balance_usd = 0 AND quota > used_quota",
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Ensure the demo user exists (required by validate_token_and_get_info JOIN).
async fn seed_demo_user(pool: &AnyPool) -> Result<()> {
    let canonical = sqlx::query(
        "INSERT OR IGNORE INTO user_accounts (id, username, password_hash, status) \
         VALUES ('demo-user', 'demo-user', 'no-login', 1)",
    )
    .execute(pool)
    .await;
    if canonical.is_err() {
        sqlx::query(
            "INSERT OR IGNORE INTO users (id, username, password_hash, status) \
             VALUES ('demo-user', 'demo-user', 'no-login', 1)",
        )
        .execute(pool)
        .await?;
    }
    Ok(())
}

/// Insert the default demo token if it does not yet exist.
async fn seed_demo_token(pool: &AnyPool, kind: &str) -> Result<()> {
    let t_count: i64 = match sqlx::query_scalar(
        "SELECT count(*) FROM user_api_keys WHERE key = 'sk-burncloud-demo'",
    )
    .fetch_one(pool)
    .await
    {
        Ok(n) => n,
        Err(_) => return Ok(()),
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
    tracing::info!("Initialized demo token: sk-burncloud-demo");
    Ok(())
}

/// Insert the four default protocol configs if the table is empty.
async fn seed_protocol_configs(pool: &AnyPool, kind: &str) -> Result<()> {
    let pc_count: i64 = match sqlx::query_scalar("SELECT count(*) FROM channel_protocol_configs")
        .fetch_one(pool)
        .await
    {
        Ok(n) => n,
        Err(_) => return Ok(()),
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

    for (channel_type, api_version, is_default, chat_endpoint, embed_endpoint, models_endpoint) in
        default_protocols
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
    tracing::info!(
        "Initialized default protocol configs for {} channel types",
        default_protocols.len()
    );
    Ok(())
}
