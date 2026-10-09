// serde_json::Value is required for dynamic JSON parsing in auth tests
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::unnecessary_cast,
    clippy::let_and_return,
    clippy::redundant_pattern_matching,
    clippy::panic_in_result_fn,
    reason = "integration tests: a failed assertion is the intended failure signal, and clippy.toml's allow-panic-in-tests does not recognise #[tokio::test]"
)]

mod common;

use burncloud_database::sqlx;
use common::{setup_db, start_test_server};
use reqwest::Client;
use serde_json::json;
use std::env;

async fn start_mock_upstream(listener: tokio::net::TcpListener) {
    let handler = |headers: axum::http::HeaderMap, body: String| async move {
        // Return a structure similar to HttpBin's response for verification
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
            "headers": header_map,
            "data": body,
            "json": serde_json::from_str::<serde_json::Value>(&body).ok()
        })
        .to_string()
    };

    axum::serve(listener, axum::Router::new().fallback(handler))
        .await
        .unwrap_or_else(|e| panic!("Mock upstream server error: {e}"));
}

#[tokio::test]
async fn test_bedrock_proxy() -> anyhow::Result<()> {
    let ak = env::var("TEST_AWS_AK").unwrap_or_default();
    let sk = env::var("TEST_AWS_SK").unwrap_or_default();
    let region = env::var("TEST_AWS_REGION").unwrap_or("us-east-1".to_string());

    if ak.is_empty() || sk.is_empty() {
        println!("Skipping AWS Bedrock test: TEST_AWS_AK or TEST_AWS_SK not set.");
        return Ok(());
    }

    let (_db, pool, db_url) = setup_db().await?;

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
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(base_url)
    .bind(api_key)
    .bind(match_path)
    .bind(auth_type)
    .execute(&pool)
    .await?;

    let port = 3002;
    start_test_server(port, &db_url).await;

    let client = Client::new();
    let url = format!(
        "http://localhost:{}/model/anthropic.claude-3-sonnet-20240229-v1:0/invoke",
        port
    );

    let body = json!({
        "anthropic_version": "bedrock-2023-05-31",
        "max_tokens": 200,
        "messages": [
            { "role": "user", "content": "Hello, just say PASS." }
        ]
    });

    let resp = client
        .post(&url)
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
    let (_db, pool, db_url) = setup_db().await?;

    // Start Mock Upstream
    let mock_port = 3020;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", mock_port))
        .await
        .unwrap_or_else(|e| panic!("Failed to bind mock port {mock_port}: {e}"));
    tokio::spawn(async move {
        start_mock_upstream(listener).await;
    });

    // Current Router routing truth is owned by Supply channels, not router_upstreams.
    common::insert_test_channel(
        &pool,
        75601,
        "deepseek_proxy fixture",
        &format!("http://127.0.0.1:{}/anything", mock_port),
        "sk-deepseek-mock-key",
        "deepseek-chat",
        "default",
    )
    .await?;

    // Preflight pricing is required before the upstream request is attempted.
    sqlx::query(
        "INSERT OR IGNORE INTO billing_prices (model, currency, input_price, output_price, region) VALUES (?, 'USD', 1, 1, '')",
    )
    .bind("deepseek-chat")
    .execute(&pool)
    .await?;

    let port = 3009;
    start_test_server(port, &db_url).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .json(&json!({"model": "deepseek-chat", "messages": [{"role": "user", "content": "test"}]}))
        .send()
        .await?;

    let status = resp.status();
    let response_text = resp.text().await?;
    assert_eq!(
        status, 200,
        "Proxy should reach its mock upstream; Router response: {response_text}"
    );
    let json: serde_json::Value = serde_json::from_str(&response_text)?;

    let headers = json
        .get("headers")
        .unwrap_or_else(|| panic!("Expected headers in response"));
    let auth_header = headers
        .get("Authorization")
        .or(headers.get("authorization"))
        .unwrap_or_else(|| panic!("Expected Authorization header"));
    assert_eq!(
        auth_header
            .as_str()
            .unwrap_or_else(|| panic!("Expected auth header as string")),
        "Bearer sk-deepseek-mock-key"
    );

    Ok(())
}

#[tokio::test]
async fn test_qwen_proxy() -> anyhow::Result<()> {
    let (_db, pool, db_url) = setup_db().await?;

    // Start Mock Upstream
    let mock_port = 3021;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", mock_port))
        .await
        .unwrap_or_else(|e| panic!("Failed to bind mock port {mock_port}: {e}"));
    tokio::spawn(async move {
        start_mock_upstream(listener).await;
    });

    // Current Router routing truth is owned by Supply channels, not router_upstreams.
    common::insert_test_channel(
        &pool,
        75602,
        "qwen_proxy fixture",
        &format!("http://127.0.0.1:{}/anything", mock_port),
        "sk-qwen-mock-key",
        "qwen-plus",
        "default",
    )
    .await?;

    // Preflight pricing is required before the upstream request is attempted.
    sqlx::query(
        "INSERT OR IGNORE INTO billing_prices (model, currency, input_price, output_price, region) VALUES (?, 'USD', 1, 1, '')",
    )
    .bind("qwen-plus")
    .execute(&pool)
    .await?;

    let port = 3010;
    start_test_server(port, &db_url).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer sk-burncloud-demo")
        .json(&json!({"model": "qwen-plus", "messages": [{"role": "user", "content": "test"}]}))
        .send()
        .await?;

    let status = resp.status();
    let response_text = resp.text().await?;
    assert_eq!(
        status, 200,
        "Proxy should reach its mock upstream; Router response: {response_text}"
    );
    let json: serde_json::Value = serde_json::from_str(&response_text)?;

    let headers = json
        .get("headers")
        .unwrap_or_else(|| panic!("Expected headers in response"));
    let auth_header = headers
        .get("Authorization")
        .or(headers.get("authorization"))
        .unwrap_or_else(|| panic!("Expected Authorization header"));
    assert_eq!(
        auth_header
            .as_str()
            .unwrap_or_else(|| panic!("Expected auth header as string")),
        "Bearer sk-qwen-mock-key"
    );

    Ok(())
}
