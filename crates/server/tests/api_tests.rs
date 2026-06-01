#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::let_and_return,
    clippy::disallowed_types
)]

use burncloud_server::create_app;
use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

mod test_utils;

#[tokio::test]
async fn test_api_health() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(db_arc, false).await?;

    let port = 4000_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/console/api/channel", port);

    // Channel endpoints require authentication - expect 401 without token
    let resp = client.get(&url).send().await?;
    assert_eq!(resp.status(), 401);

    Ok(())
}

#[tokio::test]
async fn test_token_api_requires_auth() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(db_arc, false).await?;

    let port = 4001_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let base_url = format!("http://localhost:{}/console/api/tokens", port);

    // Token API now requires authentication - expect 401 without token
    let resp = client.get(&base_url).send().await?;
    assert_eq!(resp.status(), 401, "Token list should require authentication");

    // POST should also require authentication
    let resp = client
        .post(&base_url)
        .json(&serde_json::json!({ "user_id": "test-user" }))
        .send()
        .await?;
    assert_eq!(resp.status(), 401, "Token create should require authentication");

    Ok(())
}

#[tokio::test]
async fn test_log_api_requires_auth() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(db_arc, false).await?;

    let port = 4002_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/console/api/logs", port);

    // Log API now requires authentication - expect 401 without token
    let resp = client.get(&url).send().await?;
    assert_eq!(resp.status(), 401, "Log API should require authentication");

    Ok(())
}

#[tokio::test]
async fn test_monitor_api_requires_auth() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(db_arc, false).await?;

    let port = 4003_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/console/api/monitor", port);

    // Monitor API now requires authentication - expect 401 without token
    let resp = client.get(&url).send().await?;
    assert_eq!(resp.status(), 401, "Monitor API should require authentication");

    Ok(())
}

#[tokio::test]
async fn test_cache_api_requires_auth() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(db_arc, false).await?;

    let port = 4004_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app).await.expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();

    // Cache stats endpoint should require authentication
    let stats_url = format!("http://localhost:{}/console/api/cache/stats", port);
    let resp = client.get(&stats_url).send().await?;
    assert_eq!(resp.status(), 401, "Cache stats API should require authentication");

    // Cache clear endpoint should require authentication
    let clear_url = format!("http://localhost:{}/console/api/cache/clear", port);
    let resp = client.post(&clear_url).send().await?;
    assert_eq!(resp.status(), 401, "Cache clear API should require authentication");

    Ok(())
}
