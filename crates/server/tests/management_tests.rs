use reqwest::Client;
use std::time::Duration;
use tokio::time::sleep;
use burncloud_database_router::{DbUpstream, DbToken};

#[tokio::test]
async fn test_channel_management_lifecycle() -> anyhow::Result<()> {
    let port = 4005;
    tokio::spawn(async move {
        if let Err(_e) = burncloud_server::start_server(port).await {
            // Ignore
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let base_url = format!("http://localhost:{}/console/api/channels", port);

    // 1. Create Channel
    let new_channel = serde_json::json!({
        "id": "test-chan-1",
        "name": "Test Channel",
        "base_url": "https://api.test.com",
        "api_key": "sk-test",
        "match_path": "/v1/test",
        "auth_type": "Bearer",
        "priority": 10
    });

    let resp_create = client.post(&base_url)
        .json(&new_channel)
        .send().await?;
    assert_eq!(resp_create.status(), 200);

    // 2. List Channels
    let resp_list = client.get(&base_url).send().await?;
    assert_eq!(resp_list.status(), 200);
    let channels: Vec<DbUpstream> = resp_list.json().await?;
    
    let found = channels.iter().find(|c| c.id == "test-chan-1").expect("Channel not found");
    assert_eq!(found.name, "Test Channel");
    assert_eq!(found.priority, 10);

    // 3. Get Specific Channel
    let resp_get = client.get(format!("{}/test-chan-1", base_url)).send().await?;
    assert_eq!(resp_get.status(), 200);
    let channel: DbUpstream = resp_get.json().await?;
    assert_eq!(channel.base_url, "https://api.test.com");

    // 4. Update Channel
    let update_payload = serde_json::json!({
        "id": "test-chan-1",
        "name": "Updated Name",
        "base_url": "https://api.updated.com",
        "api_key": "sk-updated",
        "match_path": "/v1/test",
        "auth_type": "Bearer",
        "priority": 5
    });
    let resp_update = client.put(format!("{}/test-chan-1", base_url))
        .json(&update_payload)
        .send().await?;
    assert_eq!(resp_update.status(), 200);

    // Verify Update
    let resp_get_2 = client.get(format!("{}/test-chan-1", base_url)).send().await?;
    let channel_2: DbUpstream = resp_get_2.json().await?;
    assert_eq!(channel_2.name, "Updated Name");
    assert_eq!(channel_2.base_url, "https://api.updated.com");

    // 5. Delete Channel
    let resp_del = client.delete(format!("{}/test-chan-1", base_url)).send().await?;
    assert_eq!(resp_del.status(), 200);

    // Verify Deletion
    let resp_get_3 = client.get(format!("{}/test-chan-1", base_url)).send().await?;
    let json_3: serde_json::Value = resp_get_3.json().await?;
    assert_eq!(json_3["error"], "Not Found"); // Assuming API returns this on not found wrapper

    Ok(())
}

#[tokio::test]
async fn test_token_management_lifecycle() -> anyhow::Result<()> {
    let port = 4006;
    tokio::spawn(async move {
        if let Err(_e) = burncloud_server::start_server(port).await {
            // Ignore
        }
    });
    sleep(Duration::from_secs(2)).await;

    let client = Client::new();
    let base_url = format!("http://localhost:{}/console/api/tokens", port);

    // 1. Create Token with Quota
    let new_token_req = serde_json::json!({
        "user_id": "quota-user-1",
        "quota_limit": 1000
    });

    let resp_create = client.post(&base_url)
        .json(&new_token_req)
        .send().await?;
    assert_eq!(resp_create.status(), 200);
    let json_create: serde_json::Value = resp_create.json().await?;
    let token_str = json_create["token"].as_str().unwrap().to_string();

    // 2. List Tokens
    let resp_list = client.get(&base_url).send().await?;
    let tokens: Vec<DbToken> = resp_list.json().await?;
    let found = tokens.iter().find(|t| t.token == token_str).expect("Token not found");
    
    assert_eq!(found.user_id, "quota-user-1");
    assert_eq!(found.quota_limit, 1000);
    assert_eq!(found.used_quota, 0);

    // 3. Delete Token
    let resp_del = client.delete(format!("{}/{}", base_url, token_str)).send().await?;
    assert_eq!(resp_del.status(), 200);

    // Verify Deletion
    let resp_list_2 = client.get(&base_url).send().await?;
    let tokens_2: Vec<DbToken> = resp_list_2.json().await?;
    assert!(!tokens_2.iter().any(|t| t.token == token_str));

    Ok(())
}
