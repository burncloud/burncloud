#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

use burncloud_database::{create_database_with_url, sqlx, Database};
use burncloud_database_router::RouterDatabase;
use sqlx::AnyPool;
use std::sync::Arc;
use tokio::net::TcpListener;

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
        .expect("Failed to open DB");
    let db_arc = Arc::new(db);

    let (app, _force_sync_tx) = burncloud_router::create_router_app(db_arc)
        .await
        .expect("Failed to create app");

    tokio::spawn(async move {
        let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
            .await
            .unwrap();
        axum::serve(listener, app).await.unwrap();
    });
    // Give server a moment to start
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
}

#[allow(dead_code)]
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
        .unwrap();
}
