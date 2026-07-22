//! SQLite migration 0017 — repair missing channel tables and coerce BOOLEAN → INTEGER.
//!
//! Some databases reached 0017 without `channel_protocol_configs` (partial 0010 apply or
//! legacy installs that only had `protocol_configs`).  The old SQL migration assumed the
//! table always existed and also attempted an incorrect `channel_abilities` reshape.

use crate::{DatabaseError, Result};
use sqlx::{AnyPool, Row};

pub async fn apply(pool: &AnyPool) -> Result<()> {
    recover_interrupted_bool_fix(
        pool,
        "channel_protocol_configs",
        CHANNEL_PROTOCOL_CONFIGS_DDL,
    )
    .await?;
    ensure_channel_protocol_configs(pool).await?;
    fix_bool_column(
        pool,
        "channel_protocol_configs",
        "is_default",
        CHANNEL_PROTOCOL_CONFIGS_DDL,
    )
    .await?;
    recreate_protocol_config_indexes(pool).await?;

    recover_interrupted_bool_fix(pool, "channel_abilities", CHANNEL_ABILITIES_DDL).await?;
    ensure_channel_abilities(pool).await?;
    fix_bool_column(pool, "channel_abilities", "enabled", CHANNEL_ABILITIES_DDL).await?;
    recreate_channel_abilities_indexes(pool).await?;

    Ok(())
}

const CHANNEL_PROTOCOL_CONFIGS_DDL: &str = r#"
CREATE TABLE {table} (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_type INTEGER NOT NULL,
    api_version TEXT NOT NULL,
    is_default INTEGER NOT NULL DEFAULT 0,
    chat_endpoint TEXT,
    embed_endpoint TEXT,
    models_endpoint TEXT,
    request_mapping TEXT,
    response_mapping TEXT,
    detection_rules TEXT,
    created_at INTEGER,
    updated_at INTEGER,
    UNIQUE(channel_type, api_version)
)"#;

const CHANNEL_ABILITIES_DDL: &str = r#"
CREATE TABLE {table} (
    "group" VARCHAR(64) NOT NULL,
    model VARCHAR(255) NOT NULL,
    channel_id INTEGER NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    priority INTEGER NOT NULL DEFAULT 0,
    weight INTEGER NOT NULL DEFAULT 0,
    tag TEXT,
    PRIMARY KEY ("group", model, channel_id)
)"#;

async fn ensure_channel_protocol_configs(pool: &AnyPool) -> Result<()> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS channel_protocol_configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            channel_type INTEGER NOT NULL,
            api_version TEXT NOT NULL,
            is_default INTEGER NOT NULL DEFAULT 0,
            chat_endpoint TEXT,
            embed_endpoint TEXT,
            models_endpoint TEXT,
            request_mapping TEXT,
            response_mapping TEXT,
            detection_rules TEXT,
            created_at INTEGER,
            updated_at INTEGER,
            UNIQUE(channel_type, api_version)
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|e| DatabaseError::Migration(format!("ensure channel_protocol_configs: {e}")))?;

    if table_exists(pool, "protocol_configs").await {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_protocol_configs")
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        if count == 0 {
            sqlx::query(
                "INSERT INTO channel_protocol_configs (\
                    channel_type, api_version, is_default, chat_endpoint, embed_endpoint, \
                    models_endpoint, request_mapping, response_mapping, detection_rules, \
                    created_at, updated_at\
                ) SELECT \
                    channel_type, api_version, is_default, chat_endpoint, embed_endpoint, \
                    models_endpoint, request_mapping, response_mapping, detection_rules, \
                    created_at, updated_at \
                FROM protocol_configs",
            )
            .execute(pool)
            .await
            .map_err(|e| {
                DatabaseError::Migration(format!("backfill channel_protocol_configs: {e}"))
            })?;
        }
    }

    Ok(())
}

async fn ensure_channel_abilities(pool: &AnyPool) -> Result<()> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS channel_abilities (
            "group" VARCHAR(64) NOT NULL,
            model VARCHAR(255) NOT NULL,
            channel_id INTEGER NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            priority INTEGER NOT NULL DEFAULT 0,
            weight INTEGER NOT NULL DEFAULT 0,
            tag TEXT,
            PRIMARY KEY ("group", model, channel_id)
        )"#,
    )
    .execute(pool)
    .await
    .map_err(|e| DatabaseError::Migration(format!("ensure channel_abilities: {e}")))?;

    if table_exists(pool, "abilities").await {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_abilities")
            .fetch_one(pool)
            .await
            .unwrap_or(0);
        if count == 0 {
            sqlx::query(
                "INSERT INTO channel_abilities (\"group\", model, channel_id, enabled, priority, weight, tag) \
                 SELECT \"group\", model, channel_id, enabled, priority, weight, tag FROM abilities",
            )
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Migration(format!("backfill channel_abilities: {e}")))?;
        }
    }

    Ok(())
}

async fn fix_bool_column(
    pool: &AnyPool,
    table: &str,
    column: &str,
    ddl_template: &str,
) -> Result<()> {
    if !table_exists(pool, table).await {
        return Ok(());
    }

    let Some(col_type) = column_type(pool, table, column).await else {
        return Ok(());
    };

    if !is_bool_type(&col_type) {
        return Ok(());
    }

    let temp = format!("{table}_boolfix");
    let mut transaction = pool
        .begin()
        .await
        .map_err(|e| DatabaseError::Migration(format!("start {table} rebuild transaction: {e}")))?;
    let create_sql = ddl_template.replace("{table}", &temp);
    sqlx::query(&create_sql)
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("create {temp}: {e}")))?;

    let copy_sql = match table {
        "channel_protocol_configs" => format!(
            "INSERT INTO {temp} \
             SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint, \
                    models_endpoint, request_mapping, response_mapping, detection_rules, \
                    created_at, updated_at \
             FROM {table}"
        ),
        "channel_abilities" => format!(
            "INSERT INTO {temp} (\"group\", model, channel_id, enabled, priority, weight, tag) \
             SELECT \"group\", model, channel_id, enabled, priority, weight, tag FROM {table}"
        ),
        _ => unreachable!(),
    };

    sqlx::query(&copy_sql)
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("copy {table} → {temp}: {e}")))?;

    sqlx::query(&format!("DROP TABLE {table}"))
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("drop {table}: {e}")))?;

    let create_final_sql = ddl_template.replace("{table}", table);
    sqlx::query(&create_final_sql)
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("recreate {table}: {e}")))?;

    let restore_sql = restore_sql(table, &temp);
    sqlx::query(&restore_sql)
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("restore {table} from {temp}: {e}")))?;

    sqlx::query(&format!("DROP TABLE {temp}"))
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("drop {temp}: {e}")))?;

    transaction.commit().await.map_err(|e| {
        DatabaseError::Migration(format!("commit {table} rebuild transaction: {e}"))
    })?;

    Ok(())
}

/// Restore a table left behind by an interrupted pre-transaction migration.
///
/// Earlier versions could leave `<table>_boolfix` behind after dropping the
/// canonical table. Prefer the canonical table when it has data; otherwise,
/// the temporary table is the only complete copy and must be restored.
async fn recover_interrupted_bool_fix(
    pool: &AnyPool,
    table: &str,
    ddl_template: &str,
) -> Result<()> {
    let temp = format!("{table}_boolfix");
    if !table_exists(pool, &temp).await {
        return Ok(());
    }

    let table_exists = table_exists(pool, table).await;
    let table_count = if table_exists {
        row_count(pool, table).await
    } else {
        0
    };
    let temp_count = row_count(pool, &temp).await;

    if table_exists && (table_count > 0 || temp_count == 0) {
        sqlx::query(&format!("DROP TABLE {temp}"))
            .execute(pool)
            .await
            .map_err(|e| DatabaseError::Migration(format!("discard stale {temp}: {e}")))?;
        return Ok(());
    }

    let mut transaction = pool.begin().await.map_err(|e| {
        DatabaseError::Migration(format!("start {table} recovery transaction: {e}"))
    })?;

    if table_exists {
        sqlx::query(&format!("DROP TABLE {table}"))
            .execute(&mut *transaction)
            .await
            .map_err(|e| DatabaseError::Migration(format!("drop incomplete {table}: {e}")))?;
    }

    sqlx::query(&ddl_template.replace("{table}", table))
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("recreate recovered {table}: {e}")))?;
    sqlx::query(&restore_sql(table, &temp))
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("restore recovered {table}: {e}")))?;
    sqlx::query(&format!("DROP TABLE {temp}"))
        .execute(&mut *transaction)
        .await
        .map_err(|e| DatabaseError::Migration(format!("drop recovered {temp}: {e}")))?;
    transaction.commit().await.map_err(|e| {
        DatabaseError::Migration(format!("commit {table} recovery transaction: {e}"))
    })?;

    Ok(())
}

fn restore_sql(table: &str, source: &str) -> String {
    match table {
        "channel_protocol_configs" => format!(
            "INSERT INTO {table} \
             SELECT id, channel_type, api_version, is_default, chat_endpoint, embed_endpoint, \
                    models_endpoint, request_mapping, response_mapping, detection_rules, \
                    created_at, updated_at \
             FROM {source}"
        ),
        "channel_abilities" => format!(
            "INSERT INTO {table} (\"group\", model, channel_id, enabled, priority, weight, tag) \
             SELECT \"group\", model, channel_id, enabled, priority, weight, tag FROM {source}"
        ),
        _ => unreachable!(),
    }
}

async fn recreate_protocol_config_indexes(pool: &AnyPool) -> Result<()> {
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_channel_protocol_configs_type ON channel_protocol_configs(channel_type)",
    )
    .execute(pool)
    .await
    .map_err(|e| DatabaseError::Migration(format!("index channel_protocol_configs.type: {e}")))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_channel_protocol_configs_version ON channel_protocol_configs(api_version)",
    )
    .execute(pool)
    .await
    .map_err(|e| DatabaseError::Migration(format!("index channel_protocol_configs.version: {e}")))?;
    Ok(())
}

async fn recreate_channel_abilities_indexes(pool: &AnyPool) -> Result<()> {
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_channel_abilities_model ON channel_abilities(model)",
    )
    .execute(pool)
    .await
    .map_err(|e| DatabaseError::Migration(format!("index channel_abilities.model: {e}")))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_channel_abilities_channel_id ON channel_abilities(channel_id)",
    )
    .execute(pool)
    .await
    .map_err(|e| DatabaseError::Migration(format!("index channel_abilities.channel_id: {e}")))?;
    Ok(())
}

async fn table_exists(pool: &AnyPool, table_name: &str) -> bool {
    sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?")
        .bind(table_name)
        .fetch_one(pool)
        .await
        .unwrap_or(0)
        > 0
}

async fn row_count(pool: &AnyPool, table: &str) -> i64 {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

async fn column_type(pool: &AnyPool, table: &str, column: &str) -> Option<String> {
    let safe = table.replace(['"', '\'', '`', ';'], "");
    let rows = sqlx::query(&format!("PRAGMA table_info({safe})"))
        .fetch_all(pool)
        .await
        .ok()?;
    for row in rows {
        if row
            .try_get::<String, _>("name")
            .ok()
            .is_some_and(|n| n.eq_ignore_ascii_case(column))
        {
            return row.try_get::<String, _>("type").ok();
        }
    }
    None
}

fn is_bool_type(type_str: &str) -> bool {
    type_str.to_uppercase().contains("BOOL")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use sqlx::any::AnyPoolOptions;

    async fn memory_pool() -> AnyPool {
        sqlx::any::install_default_drivers();
        AnyPoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:?cache=shared")
            .await
            .expect("memory pool")
    }

    #[tokio::test]
    async fn creates_missing_channel_protocol_configs_from_legacy_table() {
        let pool = memory_pool().await;

        sqlx::query(
            r#"CREATE TABLE protocol_configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_type INTEGER NOT NULL,
                api_version TEXT NOT NULL,
                is_default BOOLEAN DEFAULT 0,
                chat_endpoint TEXT,
                embed_endpoint TEXT,
                models_endpoint TEXT,
                request_mapping TEXT,
                response_mapping TEXT,
                detection_rules TEXT,
                created_at INTEGER,
                updated_at INTEGER
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            "INSERT INTO protocol_configs (channel_type, api_version, is_default, chat_endpoint) \
             VALUES (1, 'v1', 1, '/v1/chat')",
        )
        .execute(&pool)
        .await
        .unwrap();

        apply(&pool)
            .await
            .expect("0017 should repair missing canonical table");

        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_protocol_configs")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);

        let col_type = column_type(&pool, "channel_protocol_configs", "is_default")
            .await
            .unwrap();
        assert!(!is_bool_type(&col_type));
    }

    #[tokio::test]
    async fn converts_canonical_boolean_column_without_legacy_table() {
        let pool = memory_pool().await;
        sqlx::query(
            r#"CREATE TABLE channel_protocol_configs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                channel_type INTEGER NOT NULL,
                api_version TEXT NOT NULL,
                is_default BOOLEAN NOT NULL DEFAULT 0,
                chat_endpoint TEXT,
                embed_endpoint TEXT,
                models_endpoint TEXT,
                request_mapping TEXT,
                response_mapping TEXT,
                detection_rules TEXT,
                created_at INTEGER,
                updated_at INTEGER,
                UNIQUE(channel_type, api_version)
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO channel_protocol_configs (channel_type, api_version, is_default) \
             VALUES (7, 'v1', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();

        apply(&pool)
            .await
            .expect("0017 should repair the canonical table");

        let col_type = column_type(&pool, "channel_protocol_configs", "is_default")
            .await
            .unwrap();
        assert!(!is_bool_type(&col_type));
        let value: i64 = sqlx::query_scalar(
            "SELECT is_default FROM channel_protocol_configs WHERE channel_type = 7",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(value, 1);
    }

    #[tokio::test]
    async fn restores_data_from_interrupted_bool_fix() {
        let pool = memory_pool().await;
        let temp = "channel_protocol_configs_boolfix";
        sqlx::query(&CHANNEL_PROTOCOL_CONFIGS_DDL.replace("{table}", temp))
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO channel_protocol_configs_boolfix \
             (channel_type, api_version, is_default, chat_endpoint) \
             VALUES (9, 'v1', 1, '/v1/chat')",
        )
        .execute(&pool)
        .await
        .unwrap();

        apply(&pool)
            .await
            .expect("0017 should recover the interrupted rebuild");

        assert!(!table_exists(&pool, temp).await);
        let endpoint: String = sqlx::query_scalar(
            "SELECT chat_endpoint FROM channel_protocol_configs WHERE channel_type = 9",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(endpoint, "/v1/chat");
    }
}
