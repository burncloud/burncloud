#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

use burncloud_database::{create_database_with_url, sqlx, Database};
use burncloud_database_router::RouterDatabase;
use sqlx::AnyPool;
use std::sync::Arc;
use tokio::net::TcpListener;

/// Ensure the `user_accounts`, `user_api_keys`, and `router_tokens` tables
/// exist with the post-migration-0011 shape required by
/// [`RouterDatabase::validate_token_and_get_info`].
///
/// `RouterDatabase::init` (called inside [`setup_db`]) seeds an older
/// `router_tokens` schema (no `order_type` / `price_cap_nanodollars`) and
/// does **not** create the newer `user_accounts` / `user_api_keys` tables,
/// so tests that hit the JOIN path must seed these explicitly. SQLite
/// `CREATE TABLE IF NOT EXISTS` makes this idempotent across calls.
///
/// `user_roles` is also created defensively. Migration 0010 creates
/// `user_role_bindings` with a `FOREIGN KEY(role_id) REFERENCES user_roles`,
/// but `schema/rename.rs` then drops `user_roles` on fresh installs (a known
/// rename-bug, see `crates/database/tests/migration_rename_tests.rs` module
/// docs). With sqlx's default `foreign_keys=ON`, an `INSERT OR REPLACE INTO
/// user_accounts` triggers SQLite's REPLACE-cascade pre-check, which walks the
/// FK schema graph and fails with `no such table: main.user_roles`. Recreating
/// the table here closes that hole.
async fn ensure_l1_classifier_tables(pool: &AnyPool) -> anyhow::Result<()> {
    // Recreate user_roles to satisfy user_role_bindings' FK target — see
    // function-level docs for the rename-migration drop.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_roles (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            description TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_accounts (
            id TEXT PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            status INTEGER DEFAULT 1,
            `group` TEXT DEFAULT 'default'
        )
        "#,
    )
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS user_api_keys (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            key CHAR(48) NOT NULL,
            status INTEGER DEFAULT 1,
            remain_quota INTEGER DEFAULT 0,
            used_quota INTEGER DEFAULT 0
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Mirror the post-migration-0011 router_tokens shape (with order_type +
    // price_cap_nanodollars). RouterDatabase::init's inline DDL predates 0011,
    // so on a fresh test DB the table exists but lacks these columns. The
    // CREATE IF NOT EXISTS below is a no-op when the table is already present;
    // the ALTERs handle column-add for that case. SQLite has no
    // `ADD COLUMN IF NOT EXISTS`, so we ignore "duplicate column name" errors.
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS router_tokens (
            token TEXT PRIMARY KEY,
            user_id TEXT NOT NULL,
            status TEXT NOT NULL,
            quota_limit INTEGER NOT NULL DEFAULT -1,
            used_quota INTEGER NOT NULL DEFAULT 0,
            expired_time INTEGER NOT NULL DEFAULT -1,
            accessed_time INTEGER NOT NULL DEFAULT 0,
            order_type VARCHAR(16) DEFAULT 'value',
            price_cap_nanodollars BIGINT
        )
        "#,
    )
    .execute(pool)
    .await?;
    let _ = sqlx::query(
        "ALTER TABLE router_tokens ADD COLUMN order_type VARCHAR(16) DEFAULT 'value'",
    )
    .execute(pool)
    .await; // ignore duplicate-column error from older test DBs
    let _ =
        sqlx::query("ALTER TABLE router_tokens ADD COLUMN price_cap_nanodollars BIGINT")
            .execute(pool)
            .await; // ignore duplicate-column error from older test DBs

    Ok(())
}

/// Insert a fully-functional token fixture for L1 Classifier (FU-1) tests.
///
/// Always populates `user_accounts` (id=`user_id`) and `user_api_keys`
/// (key=`key`, user_id=`user_id`, status=1) so
/// [`RouterDatabase::validate_token_and_get_info`] returns `Ok(Some(_))`.
///
/// Conditionally populates `router_tokens` (token=`key`, user_id=`user_id`):
/// - If either `order_type` or `price_cap` is `Some`, a row is inserted with
///   the provided values (NULL for the other when `None`). This exercises
///   the LEFT JOIN path in `validate_token_and_get_info`.
/// - If both are `None`, **no** `router_tokens` row is inserted. This models
///   a legacy token that exists only in `user_api_keys`. The LEFT JOIN then
///   yields `(None, None)` for the L1 Classifier inputs, and the caller
///   should fall back to `OrderType::default()`.
///
/// SQLite-only DDL (`INSERT OR REPLACE`); intended for the in-process
/// fixtures used by integration tests that share [`setup_db`].
#[allow(dead_code)]
pub async fn insert_router_token(
    db: &Database,
    key: &str,
    user_id: &str,
    group: &str,
    order_type: Option<&str>,
    price_cap: Option<i64>,
) -> anyhow::Result<()> {
    let conn = db.get_connection()?;
    let pool = conn.pool();

    ensure_l1_classifier_tables(pool).await?;

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO user_accounts (id, username, password_hash, status, `group`)
        VALUES (?, ?, '', 1, ?)
        "#,
    )
    .bind(user_id)
    .bind(format!("user_{user_id}"))
    .bind(group)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT INTO user_api_keys (user_id, key, status)
        VALUES (?, ?, 1)
        "#,
    )
    .bind(user_id)
    .bind(key)
    .execute(pool)
    .await?;

    if order_type.is_some() || price_cap.is_some() {
        sqlx::query(
            r#"
            INSERT INTO router_tokens
                (token, user_id, status, quota_limit, used_quota,
                 expired_time, accessed_time, order_type, price_cap_nanodollars)
            VALUES (?, ?, 'active', -1, 0, -1, 0, ?, ?)
            "#,
        )
        .bind(key)
        .bind(user_id)
        .bind(order_type)
        .bind(price_cap)
        .execute(pool)
        .await?;
    }

    Ok(())
}

/// Ensure the `router_logs` table has the L6 Observability columns
/// (`layer_decision`, `traffic_color`) added by migration 0011.
/// `RouterDatabase::init` creates the base table without these columns,
/// so tests that INSERT/SELECT with layer_decision/traffic_color must
/// call this first. SQLite `ALTER TABLE ADD COLUMN` is idempotent if
/// we swallow "duplicate column name" errors.
#[allow(dead_code)]
pub async fn ensure_l6_observability_columns(pool: &AnyPool) -> anyhow::Result<()> {
    let _ = sqlx::query("ALTER TABLE router_logs ADD COLUMN layer_decision VARCHAR(32)")
        .execute(pool)
        .await; // ignore duplicate-column error
    let _ = sqlx::query("ALTER TABLE router_logs ADD COLUMN traffic_color CHAR(1)")
        .execute(pool)
        .await; // ignore duplicate-column error
    Ok(())
}

pub async fn setup_db() -> anyhow::Result<(Database, AnyPool, String)> {
    // Use a unique temp file per test to avoid SQLite lock contention when tests run in parallel.
    let tmp = tempfile::NamedTempFile::new()?;
    let path = tmp.path().to_string_lossy().to_string();
    // Keep the NamedTempFile alive by leaking it; the OS will clean it up after the process exits.
    std::mem::forget(tmp);
    let url = format!("sqlite:{}", path);
    let db = create_database_with_url(&url).await?;
    RouterDatabase::init(&db).await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();
    Ok((db, pool, url))
}

#[allow(dead_code)]
pub async fn start_test_server(port: u16, db_url: &str) {
    // Ensure MASTER_KEY is set for tests that need encryption (e.g. upstream API keys).
    // Use a fixed 64-hex-char test key; does not affect production.
    if std::env::var("MASTER_KEY").is_err() {
        std::env::set_var(
            "MASTER_KEY",
            "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8",
        );
    }

    // Use the URL directly so concurrent tests don't interfere via a shared env var.
    let db = create_database_with_url(db_url)
        .await
        .unwrap_or_else(|e| panic!("Failed to open DB: {e}"));
    let db_arc = Arc::new(db);

    let (app, internal_app, _force_sync_tx) = burncloud_router::create_router_app(db_arc)
        .await
        .unwrap_or_else(|e| panic!("Failed to create app: {e}"));

    // Merge internal_app before the main app so internal routes
    // (e.g. /console/internal/health) are reachable in tests.
    let app = internal_app.merge(app);

    tokio::spawn(async move {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .unwrap_or_else(|e| panic!("Failed to bind port {port}: {e}"));
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|e| panic!("Server error: {e}"));
    });
    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}

#[allow(dead_code)]
#[allow(clippy::disallowed_types)]
pub async fn start_mock_upstream(listener: TcpListener) {
    let handler = |method: axum::http::Method,
                   uri: axum::http::Uri,
                   headers: axum::http::HeaderMap,
                   body: String| async move {
        let mut header_map = serde_json::Map::new();
        for (k, v) in headers {
            if let Some(key) = k {
                header_map.insert(
                    key.to_string(),
                    serde_json::Value::String(v.to_str().unwrap_or_default().to_string()),
                );
            }
        }

        serde_json::json!({
            "method": method.to_string(),
            "url": uri.to_string(),
            "headers": header_map,
            "data": body,
            "json": serde_json::from_str::<serde_json::Value>(&body).ok()
        })
        .to_string()
    };

    axum::serve(listener, axum::Router::new().fallback(handler))
        .await
        .unwrap_or_else(|e| panic!("Mock upstream server error: {e}"));
}
