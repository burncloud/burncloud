#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::let_and_return,
    clippy::disallowed_types
)]

use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

fn ensure_master_key() {
    if std::env::var("MASTER_KEY").is_err() {
        std::env::set_var(
            "MASTER_KEY",
            "a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8a1b2c3d4e5f6a7b8",
        );
    }
}

#[tokio::test]
async fn test_api_health() -> anyhow::Result<()> {
    ensure_master_key();
    let port = 4000;
    tokio::spawn(async move {
        if let Err(e) = burncloud_server::start_server("127.0.0.1", port, false).await {
            eprintln!("Server error: {}", e);
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/console/api/channel", port);

    let resp = client.get(&url).send().await?;
    assert_eq!(resp.status(), 200);

    Ok(())
}

#[tokio::test]
async fn test_token_api() -> anyhow::Result<()> {
    ensure_master_key();
    let port = 4001;
    tokio::spawn(async move {
        if let Err(_e) = burncloud_server::start_server("127.0.0.1", port, false).await {
            // Ignore
        }
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
