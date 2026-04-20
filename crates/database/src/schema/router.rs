//! Router-related data migrations.
//!
//! Handles schema fixups for the `router_logs` table that cannot be expressed
//! as plain idempotent DDL (e.g. SQLite column-type changes that require a
//! table recreation).

use crate::Result;
use sqlx::AnyPool;

/// Fix router_logs table schema for SQLite.
///
/// The old schema used DATETIME and REAL types which are incompatible with the
/// sqlx Any driver.  SQLite does not support ALTER COLUMN, so the table is
/// recreated when the old column type is detected.
pub(super) async fn migrate_router_logs(pool: &AnyPool, kind: &str) -> Result<()> {
    if kind != "sqlite" {
        return Ok(());
    }

    let table_exists: bool = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='router_logs'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(0)
        > 0;

    let needs_migration = if table_exists {
        let col_type: Option<String> = sqlx::query_scalar(
            "SELECT type FROM pragma_table_info('router_logs') WHERE name='created_at'",
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        col_type.as_ref().map(|t| t == "DATETIME").unwrap_or(false)
    } else {
        false
    };

    if !needs_migration {
        return Ok(());
    }

    tracing::info!("[Migration] Migrating router_logs table: DATETIME -> TEXT, REAL -> INTEGER");

    let _ = sqlx::query("DROP TABLE IF EXISTS router_logs_new")
        .execute(pool)
        .await;

    let _ = sqlx::query(
        r#"
        CREATE TABLE router_logs_new (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            request_id TEXT NOT NULL,
            user_id TEXT,
            path TEXT NOT NULL,
            upstream_id TEXT,
            status_code INTEGER NOT NULL,
            latency_ms INTEGER NOT NULL,
            prompt_tokens INTEGER DEFAULT 0,
            completion_tokens INTEGER DEFAULT 0,
            cost INTEGER DEFAULT 0,
            created_at TEXT DEFAULT CURRENT_TIMESTAMP
        )
        "#,
    )
    .execute(pool)
    .await;

    let _ = sqlx::query(
        "INSERT INTO router_logs_new \
         SELECT id, request_id, user_id, path, upstream_id, status_code, \
                latency_ms, prompt_tokens, completion_tokens, \
                CAST(COALESCE(cost, 0) AS INTEGER), created_at \
         FROM router_logs",
    )
    .execute(pool)
    .await;

    let _ = sqlx::query("DROP TABLE router_logs").execute(pool).await;
    let _ = sqlx::query("ALTER TABLE router_logs_new RENAME TO router_logs")
        .execute(pool)
        .await;
    let _ =
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_router_logs_user_id ON router_logs(user_id)")
            .execute(pool)
            .await;
    let _ = sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_router_logs_created_at ON router_logs(created_at)",
    )
    .execute(pool)
    .await;

    tracing::info!("[Migration] router_logs table migration completed successfully");
    Ok(())
}
