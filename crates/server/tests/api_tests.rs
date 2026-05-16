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
        axum::serve(listener, app)
            .await
            .expect("Server error");
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
async fn test_token_api() -> anyhow::Result<()> {
    let db_arc = test_utils::make_isolated_db().await;
    let app = create_app(db_arc, false).await?;

    let port = 4001_u16;
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}"))
            .await
            .expect("Failed to bind test port");
        axum::serve(listener, app)
            .await
            .expect("Server error");
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let base_url = format!("http://localhost:{}/console/api/tokens", port);

    // 1. Create Token
    let resp = client
        .post(&base_url)
        .json(&serde_json::json!({ "user_id": "test-user" }))
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await?;
    let token = json["token"].as_str().unwrap();
    assert!(token.starts_with("sk-burncloud-"));

    // 2. List Tokens
    let resp_list = client.get(&base_url).send().await?;
    assert_eq!(resp_list.status(), 200);
    let list: serde_json::Value = resp_list.json().await?;
    let arr = list.as_array().unwrap();

    // Should find the created token
    let found = arr.iter().any(|t| t["token"] == token);
    assert!(found, "Created token not found in list");

    Ok(())
}