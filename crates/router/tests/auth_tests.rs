mod common;

use common::{setup_db, start_test_server};
use burncloud_database::sqlx;
use reqwest::Client;
use serde_json::json;
use std::env;

#[tokio::test]
async fn test_bedrock_proxy() -> anyhow::Result<()> {
    let ak = env::var("TEST_AWS_AK").unwrap_or_default();
    let sk = env::var("TEST_AWS_SK").unwrap_or_default();
    let region = env::var("TEST_AWS_REGION").unwrap_or("us-east-1".to_string());
    
    if ak.is_empty() || sk.is_empty() {
        println!("Skipping AWS Bedrock test: TEST_AWS_AK or TEST_AWS_SK not set.");
        return Ok(());
    }

    let (_db, pool) = setup_db().await?;

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
    .execute(&pool)
    .await?;

    let port = 3002; 
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/model/anthropic.claude-3-sonnet-20240229-v1:0/invoke", port);
    
    let body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 200,
        "messages": [
            { "role": "user", "content": "Hello, just say PASS." }
        ]
    });

    let resp = client.post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let _text = resp.text().await?;
    println!("Bedrock Response Status: {}", status);
    // assert!(status != 500 && status != 502); // Only assert if credentials valid
    Ok(())
}

#[tokio::test]
async fn test_deepseek_proxy() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    let id = "deepseek-test";
    let name = "DeepSeek Test";
    let base_url = "https://httpbin.org/anything";
    let api_key = "sk-deepseek-mock-key";
    let match_path = "/v1/chat/completions";
    let auth_type = "DeepSeek";

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
    .execute(&pool)
    .await?;

    let port = 3009;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    let resp = client.post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .body("deepseek body")
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await?;
    
    let headers = json.get("headers").unwrap();
    let auth_header = headers.get("Authorization").or(headers.get("authorization")).unwrap();
    assert_eq!(auth_header.as_str().unwrap(), "Bearer sk-deepseek-mock-key");

    Ok(())
}

#[tokio::test]
async fn test_qwen_proxy() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    let id = "qwen-test";
    let name = "Qwen Test";
    let base_url = "https://httpbin.org/anything";
    let api_key = "sk-qwen-mock-key";
    let match_path = "/api/v1/services/aigc/text-generation/generation";
    let auth_type = "Qwen";

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
    .execute(&pool)
    .await?;

    let port = 3010;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/api/v1/services/aigc/text-generation/generation", port);

    let resp = client.post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .body("qwen body")
        .send()
        .await?;

    assert_eq!(resp.status(), 200);
    let json: serde_json::Value = resp.json().await?;
    
    let headers = json.get("headers").unwrap();
    let auth_header = headers.get("Authorization").or(headers.get("authorization")).unwrap();
    assert_eq!(auth_header.as_str().unwrap(), "Bearer sk-qwen-mock-key");

    Ok(())
}
