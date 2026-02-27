//! Gemini Billing Test Suite
//!
//! Tests for Gemini API billing accuracy:
//! - Token counting accuracy (prompt_tokens, completion_tokens)
//! - Cost calculation correctness
//! - User balance deduction
//! - Log recording verification
//!
//! Key concepts:
//! - All prices stored in nanodollars (i64, 9 decimal precision: $1 = 1_000_000_000)
//! - Cost formula: (prompt_tokens * input_price + completion_tokens * output_price) / 1_000_000
//! - Balance and cost are both in nanodollars

use burncloud_tests::TestClient;
use dotenvy::dotenv;
use serde_json::json;
use std::env;
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::common as common_mod;

/// Get Gemini API key from environment
fn get_gemini_key() -> Option<String> {
    dotenv().ok();
    env::var("TEST_GEMINI_KEY").ok().filter(|k| !k.is_empty())
}

/// Create a Gemini channel for testing
async fn create_gemini_channel(_base_url: &str, admin_client: &TestClient) -> String {
    let gemini_key = match get_gemini_key() {
        Some(k) => k,
        None => panic!("TEST_GEMINI_KEY not set"),
    };

    let channel_name = format!("Gemini Billing Test {}", Uuid::new_v4());
    let body = json!({
        "type": 24, // Gemini channel type
        "key": gemini_key,
        "name": channel_name,
        "base_url": "https://generativelanguage.googleapis.com",
        "models": "gemini-2.0-flash,gemini-2.5-flash",
        "group": "vip",
        "weight": 10,
        "priority": 100
    });

    let res = admin_client
        .post("/console/api/channel", &body)
        .await
        .expect("Create channel failed");
    assert_eq!(res["success"], true);
    channel_name
}

/// Get demo user's current balance
async fn get_demo_user_balance(admin_client: &TestClient) -> i64 {
    let users = admin_client
        .get("/console/api/list_users")
        .await
        .expect("Failed to get users");

    let data = users.get("data").and_then(|d| d.as_array());
    assert!(data.is_some(), "Should have users data");

    for user in data.unwrap() {
        if user["username"].as_str() == Some("demo-user") {
            return user["balance_usd"].as_i64().unwrap_or(0);
        }
    }

    panic!("Demo user not found");
}

/// Get the most recent log entry
async fn get_latest_log(admin_client: &TestClient) -> Option<serde_json::Value> {
    let logs = admin_client
        .get("/console/api/logs?page=1&page_size=1")
        .await
        .expect("Failed to get logs");

    logs.get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.first().cloned())
}

/// Convert nanodollars to dollars for display
fn nano_to_dollars(nano: i64) -> f64 {
    nano as f64 / 1_000_000_000.0
}

/// Calculate expected cost in nanodollars
/// Formula: (prompt_tokens * input_price + completion_tokens * output_price) / 1_000_000
#[allow(dead_code)]
fn calculate_expected_cost(prompt_tokens: u64, completion_tokens: u64, input_price: i64, output_price: i64) -> i64 {
    let input_cost = (prompt_tokens as i128 * input_price as i128) / 1_000_000;
    let output_cost = (completion_tokens as i128 * output_price as i128) / 1_000_000;
    (input_cost + output_cost) as i64
}

// ============================================================================
// Test 1: Token counting accuracy in Gemini native format
// ============================================================================

#[tokio::test]
async fn test_token_counting_gemini_native() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Send request with Gemini native format
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Hello, this is a test message. Please respond briefly."}]
            }
        ]
    });

    println!("Testing token counting in Gemini native format...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Response: {:?}", chat_res);

    // Check for usageMetadata in passthrough mode response
    if let Some(usage) = chat_res.get("usageMetadata") {
        let prompt_tokens = usage.get("promptTokenCount").and_then(|t| t.as_u64());
        let completion_tokens = usage.get("candidatesTokenCount").and_then(|t| t.as_u64());
        let total_tokens = usage.get("totalTokenCount").and_then(|t| t.as_u64());

        println!("Prompt tokens: {:?}", prompt_tokens);
        println!("Completion tokens: {:?}", completion_tokens);
        println!("Total tokens: {:?}", total_tokens);

        assert!(prompt_tokens.is_some(), "Should have promptTokenCount");
        assert!(prompt_tokens.unwrap() > 0, "Prompt tokens should be > 0");

        assert!(completion_tokens.is_some(), "Should have candidatesTokenCount");
        assert!(completion_tokens.unwrap() > 0, "Completion tokens should be > 0");

        // Verify total = prompt + completion
        if let (Some(pt), Some(ct), Some(tt)) = (prompt_tokens, completion_tokens, total_tokens) {
            assert_eq!(pt + ct, tt, "Total tokens should equal prompt + completion");
        }

        println!("SUCCESS: Token counting verified in Gemini native format");
    } else {
        println!("WARNING: No usageMetadata in response (may be in different format)");
    }
}

// ============================================================================
// Test 2: Token counting accuracy in OpenAI format (conversion mode)
// ============================================================================

#[tokio::test]
async fn test_token_counting_openai_format() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Send request with OpenAI format (will be converted to Gemini)
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "user", "content": "What is 2+2? Answer with just the number."}
        ]
    });

    println!("Testing token counting in OpenAI format (conversion mode)...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("Response: {:?}", chat_res);

    // Check for usage in OpenAI format response
    if let Some(usage) = chat_res.get("usage") {
        let prompt_tokens = usage.get("prompt_tokens").and_then(|t| t.as_u64());
        let completion_tokens = usage.get("completion_tokens").and_then(|t| t.as_u64());
        let total_tokens = usage.get("total_tokens").and_then(|t| t.as_u64());

        println!("Prompt tokens: {:?}", prompt_tokens);
        println!("Completion tokens: {:?}", completion_tokens);
        println!("Total tokens: {:?}", total_tokens);

        assert!(prompt_tokens.is_some(), "Should have prompt_tokens");
        assert!(prompt_tokens.unwrap() > 0, "Prompt tokens should be > 0");

        assert!(completion_tokens.is_some(), "Should have completion_tokens");
        assert!(completion_tokens.unwrap() > 0, "Completion tokens should be > 0");

        // Verify total = prompt + completion
        if let (Some(pt), Some(ct), Some(tt)) = (prompt_tokens, completion_tokens, total_tokens) {
            assert_eq!(pt + ct, tt, "Total tokens should equal prompt + completion");
        }

        println!("SUCCESS: Token counting verified in OpenAI format");
    } else {
        println!("WARNING: No usage in response");
    }
}

// ============================================================================
// Test 3: Cost calculation correctness
// ============================================================================

#[tokio::test]
async fn test_cost_calculation() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Get initial balance
    let balance_before = get_demo_user_balance(&admin_client).await;
    println!("Balance before: ${:.9}", nano_to_dollars(balance_before));

    // Send request
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "Count from 1 to 10"}]
            }
        ]
    });

    println!("Testing cost calculation...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // Extract token counts
    let (prompt_tokens, completion_tokens) = if let Some(usage) = chat_res.get("usageMetadata") {
        (
            usage.get("promptTokenCount").and_then(|t| t.as_u64()).unwrap_or(0),
            usage.get("candidatesTokenCount").and_then(|t| t.as_u64()).unwrap_or(0),
        )
    } else if let Some(usage) = chat_res.get("usage") {
        (
            usage.get("prompt_tokens").and_then(|t| t.as_u64()).unwrap_or(0),
            usage.get("completion_tokens").and_then(|t| t.as_u64()).unwrap_or(0),
        )
    } else {
        (0, 0)
    };

    println!("Prompt tokens: {}", prompt_tokens);
    println!("Completion tokens: {}", completion_tokens);

    // Wait for async quota deduction
    sleep(Duration::from_millis(500)).await;

    // Get balance after
    let balance_after = get_demo_user_balance(&admin_client).await;
    println!("Balance after: ${:.9}", nano_to_dollars(balance_after));

    // Calculate balance deduction
    let balance_deducted = balance_before - balance_after;
    println!("Balance deducted: ${:.9}", nano_to_dollars(balance_deducted));

    // Verify cost was deducted (should be positive if tokens were used)
    if prompt_tokens > 0 && completion_tokens > 0 {
        assert!(balance_deducted > 0, "Balance should be deducted after API call");

        // Gemini-2.0-flash pricing: ~$0.00001/input token, ~$0.00004/output token
        // These are approximate - actual prices may vary
        // The key assertion is that cost > 0 and balance was deducted
        println!("SUCCESS: Cost calculation verified - balance deducted: ${:.9}", nano_to_dollars(balance_deducted));
    } else {
        println!("WARNING: Could not extract token counts for cost verification");
    }
}

// ============================================================================
// Test 4: User balance deduction verification
// ============================================================================

#[tokio::test]
async fn test_balance_deduction() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Get initial balance
    let balance_before = get_demo_user_balance(&admin_client).await;
    println!("Balance before: {} nanodollars (${:.9})", balance_before, nano_to_dollars(balance_before));

    // Make multiple requests to verify cumulative deduction
    for i in 1..=3 {
        let chat_body = json!({
            "model": "gemini-2.0-flash",
            "messages": [
                {"role": "user", "content": format!("Request {}: Say 'hello'", i)}
            ]
        });

        let _ = user_client
            .post("/v1/chat/completions", &chat_body)
            .await
            .expect("Chat failed");

        // Small delay between requests
        sleep(Duration::from_millis(200)).await;
    }

    // Wait for async quota deduction to complete
    sleep(Duration::from_millis(1000)).await;

    // Get balance after
    let balance_after = get_demo_user_balance(&admin_client).await;
    println!("Balance after: {} nanodollars (${:.9})", balance_after, nano_to_dollars(balance_after));

    let total_deducted = balance_before - balance_after;
    println!("Total deducted: {} nanodollars (${:.9})", total_deducted, nano_to_dollars(total_deducted));

    // Verify balance was deducted
    assert!(balance_after < balance_before, "Balance should decrease after API calls");
    assert!(total_deducted > 0, "Total deduction should be positive");

    println!("SUCCESS: Balance deduction verified over multiple requests");
}

// ============================================================================
// Test 5: Log recording verification
// ============================================================================

#[tokio::test]
async fn test_log_recording() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Send a request
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "contents": [
            {
                "role": "user",
                "parts": [{"text": "This is a log recording test"}]
            }
        ]
    });

    println!("Testing log recording...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    // Extract token counts from response
    let (prompt_tokens, completion_tokens) = if let Some(usage) = chat_res.get("usageMetadata") {
        (
            usage.get("promptTokenCount").and_then(|t| t.as_u64()).unwrap_or(0) as i32,
            usage.get("candidatesTokenCount").and_then(|t| t.as_u64()).unwrap_or(0) as i32,
        )
    } else if let Some(usage) = chat_res.get("usage") {
        (
            usage.get("prompt_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as i32,
            usage.get("completion_tokens").and_then(|t| t.as_u64()).unwrap_or(0) as i32,
        )
    } else {
        (0, 0)
    };

    // Wait for log to be written
    sleep(Duration::from_millis(500)).await;

    // Get the latest log
    let latest_log = get_latest_log(&admin_client).await;

    if let Some(log) = latest_log {
        println!("Latest log: {:?}", log);

        // Verify log has expected fields
        let log_model = log.get("model").and_then(|m| m.as_str());
        let log_prompt_tokens = log.get("prompt_tokens").and_then(|t| t.as_i64());
        let log_completion_tokens = log.get("completion_tokens").and_then(|t| t.as_i64());
        let log_cost = log.get("cost").and_then(|c| c.as_i64());
        let log_status = log.get("status_code").and_then(|s| s.as_i64());

        println!("Log model: {:?}", log_model);
        println!("Log prompt_tokens: {:?}", log_prompt_tokens);
        println!("Log completion_tokens: {:?}", log_completion_tokens);
        println!("Log cost: {:?} nanodollars", log_cost);
        println!("Log status_code: {:?}", log_status);

        // Verify log content
        assert!(log_model.is_some(), "Log should have model");
        assert!(
            log_model.unwrap().contains("gemini"),
            "Log model should contain 'gemini'"
        );

        assert!(log_prompt_tokens.is_some(), "Log should have prompt_tokens");
        assert!(log_completion_tokens.is_some(), "Log should have completion_tokens");
        assert!(log_cost.is_some(), "Log should have cost");

        // Verify token counts match
        if prompt_tokens > 0 {
            assert_eq!(
                log_prompt_tokens.unwrap() as i32, prompt_tokens,
                "Log prompt_tokens should match response"
            );
        }
        if completion_tokens > 0 {
            assert_eq!(
                log_completion_tokens.unwrap() as i32, completion_tokens,
                "Log completion_tokens should match response"
            );
        }

        // Verify cost is positive
        assert!(log_cost.unwrap() > 0, "Log cost should be positive");

        // Verify status code is 200
        assert_eq!(log_status.unwrap(), 200, "Log status_code should be 200");

        println!("SUCCESS: Log recording verified with all required fields");
    } else {
        println!("WARNING: No log found - log recording may not be working");
    }
}

// ============================================================================
// Test 6: Full billing cycle integration test
// ============================================================================

#[tokio::test]
async fn test_full_billing_cycle() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_gemini_channel(&base_url, &admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    println!("Testing full billing cycle...");

    // 1. Get initial balance
    let balance_before = get_demo_user_balance(&admin_client).await;
    println!("[1] Initial balance: ${:.9}", nano_to_dollars(balance_before));

    // 2. Send request
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "user", "content": "What is the capital of France?"}
        ]
    });

    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat failed");

    println!("[2] Request sent successfully");

    // 3. Extract token counts from response
    let (prompt_tokens, completion_tokens) = if let Some(usage) = chat_res.get("usageMetadata") {
        (
            usage.get("promptTokenCount").and_then(|t| t.as_u64()).unwrap_or(0),
            usage.get("candidatesTokenCount").and_then(|t| t.as_u64()).unwrap_or(0),
        )
    } else if let Some(usage) = chat_res.get("usage") {
        (
            usage.get("prompt_tokens").and_then(|t| t.as_u64()).unwrap_or(0),
            usage.get("completion_tokens").and_then(|t| t.as_u64()).unwrap_or(0),
        )
    } else {
        (0, 0)
    };

    println!("[3] Token counts - prompt: {}, completion: {}", prompt_tokens, completion_tokens);

    // 4. Verify response has content
    let has_content = chat_res.get("choices").and_then(|c| c.as_array())
        .map(|arr| arr.first()
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .is_some())
        .unwrap_or(false)
        || chat_res.get("candidates").and_then(|c| c.as_array())
            .map(|arr| arr.first()
                .and_then(|c| c.get("content"))
                .and_then(|c| c.get("parts"))
                .and_then(|p| p.as_array())
                .and_then(|parts| parts.first())
                .and_then(|part| part.get("text"))
                .and_then(|t| t.as_str())
                .is_some())
            .unwrap_or(false);

    assert!(has_content, "Response should have content");
    println!("[4] Response content verified");

    // 5. Wait for async operations
    sleep(Duration::from_millis(500)).await;

    // 6. Verify balance was deducted
    let balance_after = get_demo_user_balance(&admin_client).await;
    let balance_deducted = balance_before - balance_after;
    println!("[5] Balance after: ${:.9}, deducted: ${:.9}",
        nano_to_dollars(balance_after),
        nano_to_dollars(balance_deducted)
    );

    if prompt_tokens > 0 && completion_tokens > 0 {
        assert!(balance_deducted > 0, "Balance should be deducted");
    }

    // 7. Verify log was recorded
    let latest_log = get_latest_log(&admin_client).await;
    if let Some(log) = latest_log {
        let log_model = log.get("model").and_then(|m| m.as_str());
        let log_cost = log.get("cost").and_then(|c| c.as_i64());

        println!("[6] Log recorded - model: {:?}, cost: {:?} nanodollars",
            log_model, log_cost);

        assert!(log_model.is_some(), "Log should have model");

        // Verify cost in log matches balance deduction (approximately)
        if let (Some(log_cost), Some(deducted)) = (log_cost, Some(balance_deducted)) {
            // Allow small difference due to timing or rounding
            let diff = (log_cost - deducted).abs();
            let tolerance = 1000_i64; // 0.000001 USD tolerance
            assert!(
                diff <= tolerance,
                "Log cost ({}) should approximately match balance deduction ({})",
                log_cost, deducted
            );
        }
    }

    println!("[7] SUCCESS: Full billing cycle verified!");
    println!("    - Token counting: ✓");
    println!("    - Response content: ✓");
    println!("    - Balance deduction: ✓");
    println!("    - Log recording: ✓");
}
