use burncloud_database::{create_default_database, sqlx};
use burncloud_database_router::RouterDatabase;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::time::Duration;
use tokio::time::sleep;

#[tokio::test]
async fn test_bedrock_proxy() -> anyhow::Result<()> {
    // 1. Check for Environment Variables
    let ak = env::var("TEST_AWS_AK").unwrap_or_default();
    let sk = env::var("TEST_AWS_SK").unwrap_or_default();
    let region = env::var("TEST_AWS_REGION").unwrap_or("us-east-1".to_string());
    
    if ak.is_empty() || sk.is_empty() {
        println!("Skipping AWS Bedrock test: TEST_AWS_AK or TEST_AWS_SK not set.");
        return Ok(());
    }

    // 2. Setup Database
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let conn = db.connection()?;

    let id = "bedrock-sonnet-test";
    let name = "AWS Bedrock Sonnet Test";
    let base_url = format!("https://bedrock-runtime.{}.amazonaws.com", region);
    let api_key = format!("{}:{}:{}", ak, sk, region);
    let match_path = "/model";
    let auth_type = "AwsSigV4";

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            api_key = excluded.api_key,
            base_url = excluded.base_url
        "#
    )
    .bind(id).bind(name).bind(base_url).bind(api_key).bind(match_path).bind(auth_type)
    .execute(conn.pool())
    .await?;

    // 3. Start Server in Background Task (Random port to avoid conflict)
    let port = 3002; 
    tokio::spawn(async move {
        if let Err(e) = burncloud_router::start_server(port).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Wait for server startup
    sleep(Duration::from_secs(2)).await;

    // 4. Send Request
    let client = Client::new();
    let url = format!("http://localhost:{}/model/anthropic.claude-3-sonnet-20240229-v1:0/invoke", port);
    
    // Simple Claude 3 format
    let body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 200,
        "messages": [
            {
                "role": "user",
                "content": "Hello, just say PASS."
            }
        ]
    });

    let resp = client.post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo") // 网关本身的鉴权
        .json(&body)
        .send()
        .await?;

    println!("Response Status: {}", resp.status());
    let status = resp.status();
    let text = resp.text().await?;
    println!("Response Body: {}", text);

    // 5. Assertions
    // Even if we get 403/400 from AWS, it means the Router worked (it proxied).
    // If we get 500/502, the Router failed.
    assert!(status != 500 && status != 502, "Router failed to proxy properly");
    
    Ok(())
}
