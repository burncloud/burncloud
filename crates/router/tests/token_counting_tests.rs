mod common;

use burncloud_database::sqlx;
use common::{setup_db, start_test_server};
use reqwest::Client;
use serde_json::json;
use std::env;

/// Test streaming token counting with OpenAI API
/// Requires TEST_OPENAI_API_KEY environment variable
#[tokio::test]
async fn test_openai_streaming_token_count() -> anyhow::Result<()> {
    let api_key = env::var("TEST_OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        println!("Skipping OpenAI streaming token count test: TEST_OPENAI_API_KEY not set.");
        return Ok(());
    }

    let (_db, pool) = setup_db().await?;

    // Insert test upstream
    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, protocol)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            api_key = excluded.api_key,
            base_url = excluded.base_url
        "#,
    )
    .bind("test-openai-stream")
    .bind("openai-stream")
    .bind("https://api.openai.com")
    .bind(&api_key)
    .bind("/v1/chat/completions")
    .bind("Bearer")
    .bind("openai")
    .execute(&pool)
    .await?;

    // Insert test token
    sqlx::query(
        r#"
        INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota)
        VALUES (?, ?, 'active', -1, 0)
        ON CONFLICT(token) DO UPDATE SET status = 'active'
        "#,
    )
    .bind("sk-test-openai-stream")
    .bind("test-user-stream")
    .execute(&pool)
    .await?;

    let port = 3020;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    // Enable streaming with usage stats
    let request_body = json!({
        "model": "gpt-4o-mini",
        "messages": [
            { "role": "user", "content": "Say hello in 5 words" }
        ],
        "max_tokens": 50,
        "stream": true,
        "stream_options": { "include_usage": true }
    });

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer sk-test-openai-stream")
        .json(&request_body)
        .send()
        .await?;

    assert_eq!(resp.status(), 200);

    // Consume the stream
    let body = resp.text().await?;

    // Check that we received streaming data
    assert!(body.contains("data:"));
    println!("Streaming response length: {} bytes", body.len());

    // Wait a moment for logging to complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Check router_logs for token counts
    let logs: (i32, i32) = sqlx::query_as(
        "SELECT prompt_tokens, completion_tokens FROM router_logs WHERE upstream_id = 'test-openai-stream' ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(&pool)
    .await?;

    println!(
        "Token counts from logs: prompt={}, completion={}",
        logs.0, logs.1
    );

    // Prompt tokens should be > 0 (estimated or from usage)
    // Note: For OpenAI streaming with stream_options.include_usage, we should get actual counts
    assert!(logs.0 > 0, "Prompt tokens should be greater than 0");

    Ok(())
}

/// Test streaming token counting with Anthropic API
/// Requires TEST_ANTHROPIC_API_KEY environment variable
#[tokio::test]
async fn test_anthropic_streaming_token_count() -> anyhow::Result<()> {
    let api_key = env::var("TEST_ANTHROPIC_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        println!("Skipping Anthropic streaming token count test: TEST_ANTHROPIC_API_KEY not set.");
        return Ok(());
    }

    let (_db, pool) = setup_db().await?;

    // Insert test upstream
    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, protocol)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            api_key = excluded.api_key,
            base_url = excluded.base_url
        "#,
    )
    .bind("test-claude-stream")
    .bind("claude-stream")
    .bind("https://api.anthropic.com")
    .bind(&api_key)
    .bind("/v1/messages")
    .bind("XApiKey")
    .bind("claude")
    .execute(&pool)
    .await?;

    // Insert test token
    sqlx::query(
        r#"
        INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota)
        VALUES (?, ?, 'active', -1, 0)
        ON CONFLICT(token) DO UPDATE SET status = 'active'
        "#,
    )
    .bind("sk-test-claude-stream")
    .bind("test-user-claude")
    .execute(&pool)
    .await?;

    let port = 3021;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/messages", port);

    let request_body = json!({
        "model": "claude-3-haiku-20240307",
        "messages": [
            { "role": "user", "content": "Say hello in 5 words" }
        ],
        "max_tokens": 50,
        "stream": true
    });

    let resp = client
        .post(&url)
        .header("Authorization", "Bearer sk-test-claude-stream")
        .json(&request_body)
        .send()
        .await?;

    assert_eq!(resp.status(), 200);

    // Consume the stream
    let body = resp.text().await?;

    // Check that we received streaming data
    assert!(body.contains("event:"));
    println!("Streaming response length: {} bytes", body.len());

    // Wait a moment for logging to complete
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Check router_logs for token counts
    let logs: (i32, i32) = sqlx::query_as(
        "SELECT prompt_tokens, completion_tokens FROM router_logs WHERE upstream_id = 'test-claude-stream' ORDER BY created_at DESC LIMIT 1"
    )
    .fetch_one(&pool)
    .await?;

    println!(
        "Token counts from logs: prompt={}, completion={}",
        logs.0, logs.1
    );

    // Prompt tokens should be > 0
    assert!(logs.0 > 0, "Prompt tokens should be greater than 0");

    Ok(())
}

/// Test that token counting works with unit test (no API key required)
/// Uses mocked streaming data to verify parser and counter integration
#[tokio::test]
async fn test_token_counting_unit() -> anyhow::Result<()> {
    use burncloud_router::stream_parser::StreamingTokenParser;
    use burncloud_router::token_counter::StreamingTokenCounter;
    use std::sync::Arc;

    let counter = Arc::new(StreamingTokenCounter::new());

    // Simulate OpenAI streaming response
    let openai_chunks = [
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
        "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":15,\"completion_tokens\":5}}\n\n",
        "data: [DONE]\n\n",
    ];

    for chunk in &openai_chunks {
        StreamingTokenParser::parse_openai_chunk(chunk, &counter);
    }

    let (prompt, completion) = counter.get_usage();
    assert_eq!(prompt, 15, "OpenAI prompt tokens should be 15");
    assert_eq!(completion, 5, "OpenAI completion tokens should be 5");

    // Test Anthropic streaming response
    let counter2 = Arc::new(StreamingTokenCounter::new());
    let anthropic_chunks = [
        "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":25}}}\n\n",
        "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":10}}\n\n",
    ];

    for chunk in &anthropic_chunks {
        StreamingTokenParser::parse_anthropic_chunk(chunk, &counter2);
    }

    let (prompt2, completion2) = counter2.get_usage();
    assert_eq!(prompt2, 25, "Anthropic prompt tokens should be 25");
    assert_eq!(completion2, 10, "Anthropic completion tokens should be 10");

    // Test Gemini streaming response
    let counter3 = Arc::new(StreamingTokenCounter::new());
    let gemini_chunk =
        r#"{"candidates":[],"usageMetadata":{"promptTokenCount":30,"candidatesTokenCount":20}}"#;

    StreamingTokenParser::parse_gemini_chunk(gemini_chunk, &counter3);

    let (prompt3, completion3) = counter3.get_usage();
    assert_eq!(prompt3, 30, "Gemini prompt tokens should be 30");
    assert_eq!(completion3, 20, "Gemini completion tokens should be 20");

    println!("All unit tests passed!");
    Ok(())
}
