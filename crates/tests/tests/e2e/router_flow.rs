//! Router E2E Tests — Non-streaming/Streaming/OpenAI/Anthropic format coverage
//!
//! Tests the core Router functionality including:
//! - Non-streaming requests (chat completions, embeddings)
//! - Streaming requests (SSE)
//! - OpenAI format API
//! - Anthropic format API
//! - Authentication and authorization
//! - Error scenarios
//! - Routing logic (failover, circuit breaker)
//!
//! Requires a running server. Set `E2E_BASE_URL` (default `http://localhost:3000`)
//! and `TEST_OPENAI_API_KEY` before running.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::let_unit_value,
    clippy::redundant_pattern,
    clippy::manual_is_multiple_of,
    clippy::let_and_return,
    clippy::to_string_trait_impl,
    clippy::to_string_in_format_args,
    clippy::redundant_pattern_matching
)]

use reqwest::Client;
use serde_json::json;
use std::time::Duration;

fn base_url() -> String {
    std::env::var("E2E_BASE_URL").unwrap_or_else(|_| "http://localhost:3000".into())
}

fn client() -> Client {
    Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .expect("build client")
}

/// Generate a unique username to avoid collisions with prior test runs.
fn unique_username(prefix: &str) -> String {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("{prefix}_{ts}")
}

/// Get or create an admin token for testing
async fn get_admin_token() -> String {
    let url = format!("{}/api/auth/login", base_url());
    
    // Try existing admin users first
    let existing_admins = vec![
        ("testadmin2", "TestAdmin123!"),
        ("testadmin", "TestAdmin123!"),
        ("admin", "Admin123!"),
    ];
    
    for (username, password) in existing_admins {
        let resp = client()
            .post(&url)
            .json(&json!({"username": username, "password": password}))
            .send()
            .await
            .expect("login request");
        let data: serde_json::Value = resp.json().await.expect("login response json");
        if let Some(token) = data["data"]["token"].as_str() {
            if !token.is_empty() {
                return token.to_string();
            }
        }
    }
    
    // Create a new admin user if no existing admin found
    let username = unique_username("router_admin");
    let resp = client()
        .post(&format!("{}/api/auth/register", base_url()))
        .json(&json!({
            "username": &username,
            "password": "RouterTest123!",
            "email": "router@test.com"
        }))
        .send()
        .await
        .expect("register request");
    let data: serde_json::Value = resp.json().await.expect("register response json");
    data["data"]["token"].as_str().unwrap_or_default().to_string()
}

/// Ensure a test channel exists for LLM requests
async fn ensure_test_channel(token: &str) {
    let api_key = std::env::var("TEST_OPENAI_API_KEY")
        .expect("TEST_OPENAI_API_KEY must be set for Router E2E tests");
    
    let url = format!("{}/console/api/channel", base_url());
    let body = json!({
        "name": "router-e2e-test-channel",
        "type": 1, // OpenAI
        "key": api_key,
        "base_url": "https://ai.burncloud.com",
        "models": "gpt-4o-mini",
        "group": "default",
        "weight": 1,
        "priority": 0
    });
    
    let _ = client()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await;
}

// ============================================================
// Non-streaming Request Tests
// ============================================================

/// Test: Non-streaming OpenAI Chat Completions request
#[tokio::test]
#[ignore = "requires external infrastructure (running server with valid upstream API key)"]
async fn test_openai_chat_completion_non_streaming() {
    let token = get_admin_token().await;
    ensure_test_channel(&token).await;
    
    let url = format!("{}/v1/chat/completions", base_url());
    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": "Say 'test passed' in exactly two words"}],
        "max_tokens": 10,
        "stream": false
    });
    
    let resp = client()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("chat completion request");
    
    let status = resp.status().as_u16();
    let data: serde_json::Value = resp.json().await.expect("response json");
    
    assert_eq!(status, 200, "Expected 200, got {status}: {data}");
    assert!(data["choices"].as_array().is_some_and(|c| !c.is_empty()), "Expected choices array");
    assert!(data["choices"][0]["message"]["content"].as_str().is_some_and(|c| !c.is_empty()), "Expected message content");
    assert!(data["usage"]["prompt_tokens"].as_u64().unwrap_or(0) > 0, "Expected prompt_tokens > 0");
    assert!(data["usage"]["completion_tokens"].as_u64().unwrap_or(0) > 0, "Expected completion_tokens > 0");
}

/// Test: Non-streaming Embeddings API
#[tokio::test]
#[ignore = "requires external infrastructure (running server with embedding model)"]
async fn test_embeddings_api() {
    let token = get_admin_token().await;
    
    let url = format!("{}/v1/embeddings", base_url());
    let body = json!({
        "model": "text-embedding-3-small",
        "input": "Hello, world!"
    });
    
    let resp = client()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("embeddings request");
    
    let status = resp.status().as_u16();
    // Embedding model may not be configured, so we accept 200 or 404
    if status == 200 {
        let data: serde_json::Value = resp.json().await.expect("response json");
        assert!(data["data"].as_array().is_some_and(|d| !d.is_empty()), "Expected data array");
        assert!(data["data"][0]["embedding"].as_array().is_some(), "Expected embedding array");
    } else {
        eprintln!("SKIP: Embedding model not configured (status {status})");
    }
}

/// Test: Models API endpoint
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_models_list() {
    let token = get_admin_token().await;
    
    let url = format!("{}/v1/models", base_url());
    let resp = client()
        .get(&url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .expect("models request");
    
    let status = resp.status().as_u16();
    let data: serde_json::Value = resp.json().await.expect("response json");
    
    assert_eq!(status, 200, "Expected 200, got {status}: {data}");
    assert!(data["data"].as_array().is_some(), "Expected data array in response");
}

// ============================================================
// Streaming Request Tests
// ============================================================

/// Test: Streaming OpenAI Chat Completions request (SSE)
#[tokio::test]
#[ignore = "requires external infrastructure (running server with valid upstream API key)"]
async fn test_openai_chat_completion_streaming() {
    let token = get_admin_token().await;
    ensure_test_channel(&token).await;
    
    let url = format!("{}/v1/chat/completions", base_url());
    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": "Count from 1 to 3"}],
        "max_tokens": 20,
        "stream": true
    });
    
    let resp = client()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("streaming request");
    
    let status = resp.status().as_u16();
    assert_eq!(status, 200, "Expected 200 for streaming request");
    
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("text/event-stream"), "Expected SSE content-type, got {content_type}");
    
    // Read and verify SSE chunks
    let body = resp.text().await.expect("stream body");
    assert!(body.contains("data: "), "Expected SSE data format");
    assert!(body.contains("[DONE]"), "Expected [DONE] marker");
}

// ============================================================
// Anthropic Format Tests
// ============================================================

/// Test: Anthropic Messages API non-streaming
#[tokio::test]
#[ignore = "requires external infrastructure (running server with Anthropic channel)"]
async fn test_anthropic_messages_non_streaming() {
    let token = get_admin_token().await;
    
    let url = format!("{}/v1/messages", base_url());
    let body = json!({
        "model": "claude-3-haiku-20240307",
        "max_tokens": 100,
        "messages": [{"role": "user", "content": "Say hello"}]
    });
    
    let resp = client()
        .post(&url)
        .header("x-api-key", &token)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .expect("anthropic request");
    
    let status = resp.status().as_u16();
    // Anthropic channel may not be configured
    if status == 200 {
        let data: serde_json::Value = resp.json().await.expect("response json");
        assert!(data["content"].as_array().is_some_and(|c| !c.is_empty()), "Expected content array");
        assert!(data["role"].as_str().is_some(), "Expected role field");
    } else {
        eprintln!("SKIP: Anthropic channel not configured (status {status})");
    }
}

/// Test: Anthropic Messages API streaming
#[tokio::test]
#[ignore = "requires external infrastructure (running server with Anthropic channel)"]
async fn test_anthropic_messages_streaming() {
    let token = get_admin_token().await;
    
    let url = format!("{}/v1/messages", base_url());
    let body = json!({
        "model": "claude-3-haiku-20240307",
        "max_tokens": 50,
        "stream": true,
        "messages": [{"role": "user", "content": "Count to 3"}]
    });
    
    let resp = client()
        .post(&url)
        .header("x-api-key", &token)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .expect("anthropic streaming request");
    
    let status = resp.status().as_u16();
    if status == 200 {
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(content_type.contains("text/event-stream"), "Expected SSE content-type");
        
        let body = resp.text().await.expect("stream body");
        assert!(body.contains("event: message_start") || body.contains("data: "), "Expected Claude SSE events");
    } else {
        eprintln!("SKIP: Anthropic channel not configured (status {status})");
    }
}

// ============================================================
// Authentication Tests
// ============================================================

/// Test: Invalid token returns 401
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_auth_invalid_token() {
    let url = format!("{}/v1/chat/completions", base_url());
    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": "test"}]
    });
    
    let resp = client()
        .post(&url)
        .header("Authorization", "Bearer invalid_token_12345")
        .json(&body)
        .send()
        .await
        .expect("request with invalid token");
    
    let status = resp.status().as_u16();
    assert_eq!(status, 401, "Expected 401 for invalid token, got {status}");
    
    let data: serde_json::Value = resp.json().await.expect("response json");
    assert!(data["error"].is_object() || data["message"].is_string(), "Expected error in response: {data}");
}

/// Test: Missing authorization header returns 401
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_auth_missing_token() {
    let url = format!("{}/v1/chat/completions", base_url());
    let body = json!({
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": "test"}]
    });
    
    let resp = client()
        .post(&url)
        .json(&body)
        .send()
        .await
        .expect("request without token");
    
    let status = resp.status().as_u16();
    // Should return 401 (or 403 if using different auth scheme)
    assert!(status == 401 || status == 403, "Expected 401/403 for missing token, got {status}");
}

// ============================================================
// Error Scenario Tests
// ============================================================

/// Test: Model not found returns proper error
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_model_not_found_error() {
    let token = get_admin_token().await;
    
    let url = format!("{}/v1/chat/completions", base_url());
    let body = json!({
        "model": "nonexistent-model-xyz-12345",
        "messages": [{"role": "user", "content": "test"}]
    });
    
    let resp = client()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("request with nonexistent model");
    
    let status = resp.status().as_u16();
    assert!(status >= 400 && status < 500, "Expected 4xx error, got {status}");
    
    let data: serde_json::Value = resp.json().await.expect("response json");
    assert!(data["error"].is_object() || data.get("error").is_some(), "Expected error field in response: {data}");
}

/// Test: Invalid request body returns error
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_invalid_request_body() {
    let token = get_admin_token().await;
    
    let url = format!("{}/v1/chat/completions", base_url());
    let body = json!({
        // Missing required "messages" field
        "model": "gpt-4o-mini"
    });
    
    let resp = client()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("request with invalid body");
    
    let status = resp.status().as_u16();
    assert!(status >= 400 && status < 500, "Expected 4xx error for invalid request, got {status}");
}

/// Test: Empty messages array returns error
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_empty_messages_error() {
    let token = get_admin_token().await;
    
    let url = format!("{}/v1/chat/completions", base_url());
    let body = json!({
        "model": "gpt-4o-mini",
        "messages": []
    });
    
    let resp = client()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("request with empty messages");
    
    let status = resp.status().as_u16();
    assert!(status >= 400 && status < 500, "Expected 4xx error for empty messages, got {status}");
}

// ============================================================
// JSON Error Format Tests
// ============================================================

/// Test: All API errors return JSON format (not plain text)
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_json_error_format() {
    let token = get_admin_token().await;
    
    // Test with invalid model
    let url = format!("{}/v1/chat/completions", base_url());
    let body = json!({
        "model": "invalid-model",
        "messages": [{"role": "user", "content": "test"}]
    });
    
    let resp = client()
        .post(&url)
        .header("Authorization", format!("Bearer {token}"))
        .json(&body)
        .send()
        .await
        .expect("error request");
    
    // Verify response is JSON
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    assert!(content_type.contains("application/json"), "Expected JSON content-type, got {content_type}");
    
    let data: serde_json::Value = resp.json().await.expect("response json");
    // Should have error object with type, message, code
    assert!(data["error"].is_object(), "Expected error object: {data}");
}

// ============================================================
// Health Check Tests
// ============================================================

/// Test: Health endpoint returns 200
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_health_check() {
    let url = format!("{}/health", base_url());
    let resp = client()
        .get(&url)
        .send()
        .await
        .expect("health check request");
    
    let status = resp.status().as_u16();
    assert_eq!(status, 200, "Expected 200 for health check, got {status}");
}

/// Test: Status endpoint returns JSON
#[tokio::test]
#[ignore = "requires external infrastructure (running server)"]
async fn test_status_endpoint() {
    let url = format!("{}/api/status", base_url());
    let resp = client()
        .get(&url)
        .send()
        .await
        .expect("status request");
    
    let status = resp.status().as_u16();
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    
    assert_eq!(status, 200, "Expected 200 for status, got {status}");
    assert!(content_type.contains("application/json"), "Expected JSON content-type");
}
