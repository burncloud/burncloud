use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_api_health() -> anyhow::Result<()> {
    let port = 4000;
    tokio::spawn(async move {
        if let Err(e) = burncloud_server::start_server(port).await {
            eprintln!("Server error: {}", e);
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/console/channels", port);
    
    let resp = client.get(&url).send().await?;
    assert_eq!(resp.status(), 200);
    
    Ok(())
}

#[tokio::test]
async fn test_token_api() -> anyhow::Result<()> {
    let port = 4001;
    tokio::spawn(async move {
        if let Err(_e) = burncloud_server::start_server(port).await {
            // Ignore
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let base_url = format!("http://localhost:{}/console/tokens", port);

    // 1. Create Token
    let resp = client.post(&base_url)
        .json(&serde_json::json!({ "user_id": "test-user" }))
        .send().await?;
    
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
