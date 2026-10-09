#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    dead_code,
    reason = "Shared test helper. Each file in `tests/` is its own crate, so this one is also built as a \
              standalone test binary in which none of these helpers is used; the binaries that do use them \
              are the `*_tests.rs` files beside it. The helpers stay `pub` within the crate for that reason, \
              which is also why some are reported as unreachable from outside it."
)]

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
/// rename-bug, see `crates/platform/storage/database/tests/migration_rename_tests.rs` module
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
    sqlx::query("ALTER TABLE router_tokens ADD COLUMN order_type VARCHAR(16) DEFAULT 'value'")
        .execute(pool)
        .await
        .ok(); // ignore duplicate-column error from older test DBs
    sqlx::query("ALTER TABLE router_tokens ADD COLUMN price_cap_nanodollars BIGINT")
        .execute(pool)
        .await
        .ok(); // ignore duplicate-column error from older test DBs

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
#[allow(
    dead_code,
    reason = "shared test helper: each file in `tests/` is its own crate, so a helper used only by a sibling test binary is dead code in this one"
)]
pub(crate) async fn insert_router_token(
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
#[allow(
    dead_code,
    reason = "shared test helper: each file in `tests/` is its own crate, so a helper used only by a sibling test binary is dead code in this one"
)]
pub(crate) async fn ensure_l6_observability_columns(pool: &AnyPool) -> anyhow::Result<()> {
    sqlx::query("ALTER TABLE router_logs ADD COLUMN layer_decision VARCHAR(32)")
        .execute(pool)
        .await
        .ok(); // ignore duplicate-column error
    sqlx::query("ALTER TABLE router_logs ADD COLUMN traffic_color CHAR(1)")
        .execute(pool)
        .await
        .ok(); // ignore duplicate-column error
    Ok(())
}

/// Ensure the `router_logs` table has the `cost_status` column added by
/// migration 0013. `RouterDatabase::init` creates the base table without
/// this column, so tests that INSERT/SELECT with cost_status must call this
/// first. SQLite `ALTER TABLE ADD COLUMN` is idempotent if we swallow
/// "duplicate column name" errors.
#[allow(
    dead_code,
    reason = "shared test helper: each file in `tests/` is its own crate, so a helper used only by a sibling test binary is dead code in this one"
)]
pub(crate) async fn ensure_cost_status_column(pool: &AnyPool) -> anyhow::Result<()> {
    sqlx::query("ALTER TABLE router_logs ADD COLUMN cost_status TEXT DEFAULT NULL")
        .execute(pool)
        .await
        .ok(); // ignore duplicate-column error
    Ok(())
}

/// Ensure the `router_logs` table has the `error_type` column added by
/// migration 0014. Same pattern as `ensure_cost_status_column`.
#[allow(
    dead_code,
    reason = "shared test helper: each file in `tests/` is its own crate, so a helper used only by a sibling test binary is dead code in this one"
)]
pub(crate) async fn ensure_error_type_column(pool: &AnyPool) -> anyhow::Result<()> {
    sqlx::query("ALTER TABLE router_logs ADD COLUMN error_type TEXT DEFAULT NULL")
        .execute(pool)
        .await
        .ok(); // ignore duplicate-column error
    Ok(())
}

/// Ensure the `channel_providers` and `channel_abilities` tables exist.
/// These tables supersede the deprecated `router_upstreams`, `router_groups`,
/// and `router_group_members` tables.
#[allow(
    dead_code,
    reason = "shared test helper: each file in `tests/` is its own crate, so a helper used only by a sibling test binary is dead code in this one"
)]
pub(crate) async fn ensure_channel_tables(pool: &AnyPool) -> anyhow::Result<()> {
    // Create channel_providers if not exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS channel_providers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            type INTEGER DEFAULT 0,
            key TEXT NOT NULL,
            status INTEGER DEFAULT 1,
            name TEXT,
            weight INTEGER DEFAULT 0,
            base_url TEXT DEFAULT '',
            models TEXT,
            `group` TEXT DEFAULT 'default',
            used_quota BIGINT DEFAULT 0,
            priority BIGINT DEFAULT 0,
            auto_ban INTEGER DEFAULT 1,
            rpm_cap INTEGER,
            tpm_cap BIGINT,
            param_override TEXT,
            header_override TEXT,
            api_version TEXT,
            pricing_region TEXT
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create channel_abilities if not exists
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS channel_abilities (
            `group` TEXT NOT NULL,
            model TEXT NOT NULL,
            channel_id INTEGER NOT NULL,
            enabled INTEGER DEFAULT 1,
            priority INTEGER DEFAULT 0,
            weight INTEGER DEFAULT 0,
            PRIMARY KEY (`group`, model, channel_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Insert a channel for testing.
/// Creates entries in both `channel_providers` and `channel_abilities`.
#[allow(
    dead_code,
    clippy::too_many_arguments,
    reason = "shared test helper: each file in `tests/` is its own crate, so a helper used only by a sibling test binary is dead code in this one; the channel fixture's seven fields are all required by the INSERT"
)]
pub(crate) async fn insert_test_channel(
    pool: &AnyPool,
    channel_id: i32,
    name: &str,
    base_url: &str,
    api_key: &str,
    model: &str,
    group: &str,
) -> anyhow::Result<()> {
    ensure_channel_tables(pool).await?;

    // `type = 1` is `ChannelType::OpenAI`, and it is not cosmetic: the router filters candidates by path
    // format at `lib.rs:2194-2202`, so a `/v1/chat/completions` request is only routed to a channel whose
    // type is OpenAI or Zai. This fixture used to write `type = 0` (Unknown), which meant every test that
    // drove an OpenAI-format request through this channel was answered `404 no_available_channel` with no
    // error and no visible log -- the channel was silently skipped after passing every other filter.
    // Measured, not inferred: see the bisection in #660.
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO channel_providers
        (id, type, key, status, name, weight, base_url, models, `group`, priority)
        VALUES (?, 1, ?, 1, ?, 1, ?, ?, ?, 0)
        "#,
    )
    .bind(channel_id)
    .bind(api_key)
    .bind(name)
    .bind(base_url)
    .bind(model)
    .bind(group)
    .execute(pool)
    .await?;

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO channel_abilities (`group`, model, channel_id, enabled, priority, weight)
        VALUES (?, ?, ?, 1, 0, 1)
        "#,
    )
    .bind(group)
    .bind(model)
    .bind(channel_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Build a SQLite connection URL for a filesystem path.
///
/// **The three-slash rule.** `sqlite:///C:/data/db.sqlite` is the absolute form; `sqlite://C:/data/db.sqlite`
/// has only two slashes after the scheme, so SQLite reads `C:` as a **host** and refuses the connection with
/// `(code: 14) unable to open database file`. A relative path keeps two slashes, because with three the leading
/// `/` would become part of the path and turn `data/db.sqlite` into `/data/db.sqlite`, which is not what the
/// caller asked for.
///
/// Measured on Windows, the only platform this was run on: the three-slash form opens, the two-slash form is
/// refused with `(code: 14)`, and `sqlite:////C:/...` -- four slashes, the shape a naive
/// `format!("sqlite:///{absolute}")` produces for an absolute Windows path -- is **also** refused. The
/// relative-path branch below was measured to open on Windows; no caller here exercises it, because `tempfile`
/// always returns an absolute path, and it has not been run on Unix.
///
/// This is a local copy of `burncloud_database::sqlite_url`, which lives in an open pull request rather than on
/// `main`. Duplicating eight lines across the three test files that need it is the price of a PR that does not
/// depend on another one landing first; once that function is on `main`, these copies should be replaced by it.
fn sqlite_url(path: &str) -> String {
    let normalised = path.replace('\\', "/");
    if normalised.starts_with('/') || has_drive_letter(&normalised) {
        format!("sqlite:///{}?mode=rwc", normalised.trim_start_matches('/'))
    } else {
        format!("sqlite://{}?mode=rwc", normalised)
    }
}

/// Whether the path begins with a Windows drive letter followed by a separator (`C:/`).
fn has_drive_letter(path: &str) -> bool {
    let bytes: Vec<char> = path.chars().take(3).collect();
    bytes.len() == 3 && bytes[0].is_ascii_alphabetic() && bytes[1] == ':' && bytes[2] == '/'
}
pub(crate) async fn setup_db() -> anyhow::Result<(Database, AnyPool, String)> {
    // Use a unique temp file per test to avoid SQLite lock contention when tests run in parallel.
    let tmp = tempfile::NamedTempFile::new()?;
    let path = tmp.path().to_string_lossy().to_string();
    // Keep the NamedTempFile alive by leaking it; the OS will clean it up after the process exits.
    std::mem::forget(tmp);
    // **`sqlite_url`, not `format!("sqlite://{}?mode=rwc", path)`.** The two-slash form is correct only
    // for a *relative* path; `tempfile` returns an absolute one, and SQLite reads `sqlite://C:\...` as a host
    // named `C:` and refuses it with `(code: 14) unable to open database file`. Every `burncloud-router`
    // integration test builds its database through this one helper, so the malformed URL failed seventeen test
    // targets at once rather than one.
    //
    // `main` fixed the same defect inline with `#[cfg(windows)]`/`#[cfg(not(windows))]`. On Windows the two
    // produce the identical URL, compared directly, so this helper is kept: one definition shared by the four
    // call sites instead of four copies.
    let url = sqlite_url(&path);
    let db = create_database_with_url(&url).await?;
    RouterDatabase::init(&db).await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();
    Ok((db, pool, url))
}

#[allow(
    dead_code,
    reason = "shared test helper: each file in `tests/` is its own crate, so a helper used only by a sibling test binary is dead code in this one"
)]
pub(crate) async fn start_test_server(port: u16, db_url: &str) {
    let listener = TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap_or_else(|e| panic!("Failed to bind port {port}: {e}"));
    start_test_server_on(listener, db_url).await;
}

/// Start a test Router using a pre-bound listener, permitting the OS to
/// allocate an unused port before creating mock channel URLs. Tests that
/// launch concurrent integration servers should prefer this over fixed ports.
#[allow(
    dead_code,
    reason = "shared test helper: each integration test file is compiled independently"
)]
pub(crate) async fn start_test_server_on(listener: TcpListener, db_url: &str) {
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

    let jwt_secret = burncloud_service_user::JwtSecret::new("burncloud-router-test-jwt-secret")
        .unwrap_or_else(|e| panic!("test JWT secret must be valid: {e}"));
    let (app, internal_app, _force_sync_tx) =
        burncloud_router::create_router_app(db_arc, jwt_secret)
            .await
            .unwrap_or_else(|e| panic!("Failed to create app: {e}"));

    // Merge internal_app before the main app so internal routes
    // (e.g. /console/internal/health) are reachable in tests.
    let app = internal_app.merge(app);

    tokio::spawn(async move {
        axum::serve(listener, app)
            .await
            .unwrap_or_else(|e| panic!("Server error: {e}"));
    });
    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}

#[allow(
    dead_code,
    reason = "shared test helper: each file in `tests/` is its own crate, so a helper used only by a sibling test binary is dead code in this one"
)]
#[allow(
    clippy::disallowed_types,
    reason = "mock upstream: the handler echoes arbitrary client JSON back, so serde_json::Value is the correct representation of a protocol boundary of unknown shape"
)]
pub(crate) async fn start_mock_upstream(listener: TcpListener) {
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
            "json": serde_json::from_str::<serde_json::Value>(&body).ok(),
            // Model-routed HTTP tests must return a valid OpenAI response. The
            // original echo fields above remain available for URL/auth checks.
            // Without a completion/usage, Router's response-quality gate
            // correctly rejects the mock as malformed (HTTP 502).
            "id": "chatcmpl-test-upstream",
            "object": "chat.completion",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "Mock upstream response"},
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 2,
                "completion_tokens": 3,
                "total_tokens": 5
            }
        })
        .to_string()
    };

    axum::serve(listener, axum::Router::new().fallback(handler))
        .await
        .unwrap_or_else(|e| panic!("Mock upstream server error: {e}"));
}
