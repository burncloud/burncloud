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

    assert!(status != 500 && status != 502, "Router failed to proxy properly");
    
    Ok(())
}

#[tokio::test]
async fn test_header_auth_proxy() -> anyhow::Result<()> {
    // Test Generic Header Injection (e.g. for Azure or AWS Gateway)
    // We will use httpbin.org to verify the header is injected correctly
    
    // 1. Setup Database
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let conn = db.connection()?;

    let id = "httpbin-test";
    let name = "HttpBin Test";
    let base_url = "https://httpbin.org";
    let api_key = "my-secret-azure-key";
    let match_path = "/anything";
    let auth_type = "Header:api-key"; // Simulate Azure style

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            api_key = excluded.api_key,
            base_url = excluded.base_url,
            auth_type = excluded.auth_type
        "#
    )
    .bind(id).bind(name).bind(base_url).bind(api_key).bind(match_path).bind(auth_type)
    .execute(conn.pool())
    .await?;

    // 2. Start Server (Port 3003)
    let port = 3003;
    tokio::spawn(async move {
        if let Err(_e) = burncloud_router::start_server(port).await {
            // Ignore error (port binding) as test might run in parallel
        }
    });
    sleep(Duration::from_secs(2)).await;

    // 3. Send Request
    let client = Client::new();
    let url = format!("http://localhost:{}/anything/test", port);

    let resp = client.post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .body("test body")
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await?;
    
    // 4. Verify Header Injection
    // httpbin returns the headers it received in "headers" field
    let headers = json.get("headers").unwrap();
    println!("Received Headers: {:?}", headers);

    // Verify 'api-key' is present and correct
    // Note: httpbin might capitalize headers differently
    let injected_key = headers.get("Api-Key").or(headers.get("api-key")).unwrap();
    assert_eq!(injected_key.as_str().unwrap(), "my-secret-azure-key");

    // Verify 'Authorization' (Bearer sk-burncloud-demo) is REMOVED
    assert!(headers.get("Authorization").is_none());

    Ok(())
}

#[tokio::test]
async fn test_aws_api_key_proxy() -> anyhow::Result<()> {
    // 1. Check for Environment Variables
    let api_key = env::var("TEST_AWS_API_KEY").unwrap_or_default();
    let endpoint = env::var("TEST_AWS_ENDPOINT").unwrap_or_default();
    
    if api_key.is_empty() || endpoint.is_empty() {
        println!("Skipping AWS API Key test: TEST_AWS_API_KEY or TEST_AWS_ENDPOINT not set.");
        return Ok(());
    }

    println!("Running AWS API Key test against: {}", endpoint);

    // 2. Setup Database
    let db = create_default_database().await?;
    RouterDatabase::init(&db).await?;
    let conn = db.connection()?;

    let id = "aws-api-gateway-test";
    let name = "AWS API Gateway Test";
    let base_url = endpoint; // e.g. https://my-api.execute-api.us-east-1.amazonaws.com
    let match_path = "/aws-test";
    let auth_type = "Header:x-api-key"; // Standard AWS API Gateway Header

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET 
            api_key = excluded.api_key,
            base_url = excluded.base_url,
            auth_type = excluded.auth_type
        "#
    )
    .bind(id).bind(name).bind(base_url).bind(api_key).bind(match_path).bind(auth_type)
    .execute(conn.pool())
    .await?;

    // 3. Start Server (Port 3004)
    let port = 3004; 
    tokio::spawn(async move {
        if let Err(e) = burncloud_router::start_server(port).await {
            eprintln!("Server error: {}", e);
        }
    });

    sleep(Duration::from_secs(2)).await;

    // 4. Send Request
    let client = Client::new();
    // Note: We append /v1/chat/completions or whatever the upstream expects.
    // Since match_path is /aws-test, we request /aws-test/v1/chat/completions
    let url = format!("http://localhost:{}/aws-test/v1/chat/completions", port);
    
    let body = json!({
        "model": "gpt-3.5-turbo", // Or whatever the gateway supports
        "messages": [{"role": "user", "content": "Hello"}]
    });

    let resp = client.post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .json(&body)
        .send()
        .await?;

    println!("Response Status: {}", resp.status());
    let status = resp.status();
    let text = resp.text().await?;
    println!("Response Body Summary: {:.100}...", text); // Truncate log for safety

    // 5. Assertions
    assert!(status != 500 && status != 502, "Router failed to proxy properly. Check logs for details.");
    
    Ok(())
}
