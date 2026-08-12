//! SQLite migration 0017 — repair missing channel tables and coerce BOOLEAN → INTEGER.
//!
//! Some databases reached 0017 without `channel_protocol_configs` (partial 0010 apply or
//! legacy installs that only had `protocol_configs`). The old SQL migration assumed the
//! table always existed and also attempted an incorrect `channel_abilities` reshape.
//!
//! SQLite table rebuilds must stay on one connection and one transaction. Production uses
//! a multi-connection pool, so issuing CREATE/COPY/DROP/RENAME as independent pool queries
//! can expose intermediate schema states across connections. The rebuild below is atomic:
//! either the canonical table remains untouched or the complete INTEGER-backed table is
//! committed.

use crate::{DatabaseError, Result};
use sqlx::{AnyPool, Row};

pub async fn apply(pool: &AnyPool) -> Result<()> {
    ensure_channel_protocol_configs(pool).await?;
    fix_bool_column(pool, "channel_protocol_configs", "is_default", CHANNEL_PROTOCOL_CONFIGS_DDL)
        .await?;
    recreate_protocol_config_indexes(pool).await?;

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
            .map_err(|e| DatabaseError::Migration(format!("backfill channel_protocol_configs: {e}")))?;
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

async fn fix_bool_column(pool: &AnyPool, table: &str, column: &str, ddl_template: &str) -> Result<()> {
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
    let create_sql = ddl_template.replace("{table}", &temp);

    // Pin the complete SQLite table rebuild to one connection/transaction. This
    // prevents the production AnyPool (10 connections) from observing or acting
    // on the intermediate state between DROP and RENAME.
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DatabaseError::Migration(format!("begin {table} bool rebuild: {e}")))?;

    // A failed pre-transaction implementation could have left a temporary table
    // behind while keeping the canonical table. In that recoverable state the
    // canonical table is the source of truth, so discard only the stale temp.
    sqlx::query(&format!("DROP TABLE IF EXISTS {temp}"))
        .execute(&mut *tx)
        .await
        .map_err(|e| DatabaseError::Migration(format!("drop stale {temp}: {e}")))?;

    sqlx::query(&create_sql)
        .execute(&mut *tx)
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
        .execute(&mut *tx)
        .await
        .map_err(|e| DatabaseError::Migration(format!("copy {table} → {temp}: {e}")))?;

    sqlx::query(&format!("DROP TABLE {table}"))
        .execute(&mut *tx)
        .await
        .map_err(|e| DatabaseError::Migration(format!("drop {table}: {e}")))?;

    sqlx::query(&format!("ALTER TABLE {temp} RENAME TO {table}"))
        .execute(&mut *tx)
        .await
        .map_err(|e| DatabaseError::Migration(format!("rename {temp}: {e}")))?;

    tx.commit()
        .await
        .map_err(|e| DatabaseError::Migration(format!("commit {table} bool rebuild: {e}")))?;

    Ok(())
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
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_channel_abilities_model ON channel_abilities(model)")
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
mod tests {
    use super::*;
    use sqlx::any::AnyPoolOptions;
    use tempfile::TempDir;

    async fn file_pool() -> (AnyPool, TempDir) {
        sqlx::any::install_default_drivers();
        let dir = tempfile::tempdir().expect("temp SQLite directory");
        let db_path = dir.path().join("migration-0017.db");
        let url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy().replace('\\', "/"));
        let pool = AnyPoolOptions::new()
            // Production DatabaseConnection uses a multi-connection pool backed by
            // one SQLite file. A :memory: pool does not model that shared schema.
            .max_connections(10)
            .connect(&url)
            .await
            .expect("file-backed SQLite pool");
        (pool, dir)
    }

    #[tokio::test]
    async fn creates_missing_channel_protocol_configs_from_legacy_table() {
        let (pool, _dir) = file_pool().await;

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

        apply(&pool).await.expect("0017 should repair missing canonical table");

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
    async fn rebuilds_existing_boolean_tables_atomically_with_production_sized_pool() {
        let (pool, _dir) = file_pool().await;

        sqlx::query(
            r#"CREATE TABLE channel_protocol_configs (
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
                updated_at INTEGER,
                UNIQUE(channel_type, api_version)
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO channel_protocol_configs (channel_type, api_version, is_default) VALUES (14, 'v1', 1)",
        )
        .execute(&pool)
        .await
        .unwrap();

        sqlx::query(
            r#"CREATE TABLE channel_abilities (
                "group" VARCHAR(64) NOT NULL,
                model VARCHAR(255) NOT NULL,
                channel_id INTEGER NOT NULL,
                enabled BOOLEAN DEFAULT 1,
                priority INTEGER DEFAULT 0,
                weight INTEGER DEFAULT 0,
                tag TEXT,
                PRIMARY KEY ("group", model, channel_id)
            )"#,
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO channel_abilities (\"group\", model, channel_id, enabled, priority, weight) \
             VALUES ('default', 'staging-model', 1, 1, 0, 100)",
        )
        .execute(&pool)
        .await
        .unwrap();

        apply(&pool)
            .await
            .expect("0017 should atomically rebuild BOOLEAN canonical tables");
        // A retry after success must be a no-op.
        apply(&pool).await.expect("0017 should remain idempotent");

        let protocol_type = column_type(&pool, "channel_protocol_configs", "is_default")
            .await
            .unwrap();
        let abilities_type = column_type(&pool, "channel_abilities", "enabled")
            .await
            .unwrap();
        assert!(!is_bool_type(&protocol_type));
        assert!(!is_bool_type(&abilities_type));

        let protocol_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_protocol_configs")
            .fetch_one(&pool)
            .await
            .unwrap();
        let ability_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM channel_abilities")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(protocol_count, 1);
        assert_eq!(ability_count, 1);
    }
}
