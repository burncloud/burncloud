use burncloud_database::{create_default_database, sqlx, Database};
use burncloud_database_router::RouterDatabase;
use sqlx::AnyPool;
use std::sync::Arc;
use tokio::net::TcpListener;

pub async fn setup_db() -> anyhow::Result<(Database, AnyPool)> {
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let conn = db.get_connection()?;
    let pool = conn.pool().clone();
    Ok((db, pool))
}

pub async fn start_test_server(port: u16) {
    // We create a new DB instance pointing to the same file
    // Note: Tests run sequentially or use different ports/tables?
    // They use the same default DB file. WAL mode handles concurrency.
    let db = create_default_database().await.expect("Failed to open DB");
    let db_arc = Arc::new(db);

    let app = burncloud_router::create_router_app(db_arc)
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

pub async fn start_mock_upstream(listener: TcpListener) {
    let handler = |headers: axum::http::HeaderMap, body: String| async move {
        // Return a structure similar to HttpBin's response for verification
        let mut header_map = serde_json::Map::new();
        for (k, v) in headers {
            if let Some(key) = k {
                 header_map.insert(key.to_string(), serde_json::Value::String(v.to_str().unwrap_or_default().to_string()));
            }
        }
        
        // Construct a response that mimics HttpBin's /anything
        // Specifically, Balancer test checks for `json["url"]`
        // We need to inject the request URL into the response.
        // But the handler doesn't know the full URL requested (host/port).
        // The headers might contain Host.
        // Or we can just mock it based on expected behavior.
        // Balancer test expects `url` to contain `/u1` or `/u2`.
        // We can check the path from the request URI if we had it.
        // Let's change the handler signature to accept URI.
        
        serde_json::json!({
            "headers": header_map,
            "data": body,
            "json": serde_json::from_str::<serde_json::Value>(&body).ok(),
            "url": "http://mocked/u1/group-test" // This needs to be dynamic!
        }).to_string()
    };
    
    // Improved handler to capture URI
    let handler = |method: axum::http::Method, uri: axum::http::Uri, headers: axum::http::HeaderMap, body: String| async move {
         let mut header_map = serde_json::Map::new();
        for (k, v) in headers {
            if let Some(key) = k {
                 header_map.insert(key.to_string(), serde_json::Value::String(v.to_str().unwrap_or_default().to_string()));
            }
        }
        
        serde_json::json!({
            "method": method.to_string(),
            "url": uri.to_string(), // This will be the path only (e.g. /anything/u1/group-test) unless absolute form used
            "headers": header_map,
            "data": body,
            "json": serde_json::from_str::<serde_json::Value>(&body).ok()
        }).to_string()
    };

    axum::serve(
        listener,
        axum::Router::new().fallback(handler),
    )
    .await
    .unwrap();
}
