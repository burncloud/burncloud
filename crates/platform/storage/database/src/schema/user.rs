//! User and token data migrations.
//!
//! Handles:
//! - Migrating `user_api_keys.unlimited_quota` from BOOLEAN to INTEGER (SQLite only)
//! - Migrating legacy `quota` column values to `balance_usd`
//!
//! Default business records are **not** created here. Since #842 the demo account and demo
//! API key are seeded by `identity/user` (`UserDatabase::seed_demo_defaults`) and the default
//! protocol configs by `supply/channel`
//! (`ChannelProtocolConfigModel::seed_default_protocol_configs`). The application bootstrap
//! sequences those owner calls; Platform writes no domain rows itself, which is what keeps
//! the dependency direction one-way.

use crate::Result;
use sqlx::AnyPool;

/// Historical compatibility fixups; never create new business records.
pub(super) async fn migrate_users(pool: &AnyPool, kind: &str) -> Result<()> {
    migrate_tokens_unlimited_quota(pool, kind).await?;
    migrate_quota_to_balance(pool).await?;
    Ok(())
}

async fn sqlite_table_exists(pool: &AnyPool, table: &str) -> Result<bool> {
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?")
            .bind(table)
            .fetch_one(pool)
            .await?;
    Ok(count > 0)
}

async fn token_table_names(
    pool: &AnyPool,
) -> Result<Option<(&'static str, &'static str, &'static str)>> {
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
