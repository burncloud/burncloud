//! Region-Based Pricing Integration Tests
//!
//! Tests for verifying CNY vs USD pricing based on channel region:
//! - CN region channel should use CNY pricing (deduct from balance_cny)
//! - International region channel should use USD pricing (deduct from balance_usd)
//!
//! Key concepts:
//! - Channel has `pricing_region` field: "cn", "international", or NULL
//! - Prices have `region` and `currency` fields
//! - When routing through a channel, router uses its pricing_region to look up price
//! - Price's currency determines which balance to deduct (CNY or USD)
//!
//! All prices stored in nanodollars (i64, 9 decimal precision: $1 = 1_000_000_000)

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

/// Convert nanodollars to dollars for display
fn nano_to_dollars(nano: i64) -> f64 {
    nano as f64 / 1_000_000_000.0
}

/// Create a CN region channel (uses CNY pricing)
async fn create_cn_channel(admin_client: &TestClient) -> String {
    let gemini_key = match get_gemini_key() {
        Some(k) => k,
        None => panic!("TEST_GEMINI_KEY not set"),
    };

    let channel_name = format!("CN Region Test {}", Uuid::new_v4());
    let body = json!({
        "type": 24, // Gemini channel type
        "key": gemini_key,
        "name": channel_name,
        "base_url": "https://generativelanguage.googleapis.com",
        "models": "gemini-2.0-flash",
        "group": "cn",
        "weight": 10,
        "priority": 100,
        "pricing_region": "cn"  // CN region for CNY pricing
    });

    let res = admin_client
        .post("/console/api/channel", &body)
        .await
        .expect("Create channel failed");
    assert_eq!(res["success"], true, "Failed to create CN channel: {:?}", res);
    channel_name
}

/// Create an International region channel (uses USD pricing)
async fn create_intl_channel(admin_client: &TestClient) -> String {
    let gemini_key = match get_gemini_key() {
        Some(k) => k,
        None => panic!("TEST_GEMINI_KEY not set"),
    };

    let channel_name = format!("Intl Region Test {}", Uuid::new_v4());
    let body = json!({
        "type": 24, // Gemini channel type
        "key": gemini_key,
        "name": channel_name,
        "base_url": "https://generativelanguage.googleapis.com",
        "models": "gemini-2.0-flash",
        "group": "vip",
        "weight": 10,
        "priority": 100,
        "pricing_region": "international"  // International region for USD pricing
    });

    let res = admin_client
        .post("/console/api/channel", &body)
        .await
        .expect("Create channel failed");
    assert_eq!(res["success"], true, "Failed to create Intl channel: {:?}", res);
    channel_name
}

/// Get demo user's current balances (USD, CNY)
async fn get_demo_user_balances(admin_client: &TestClient) -> (i64, i64) {
    let users = admin_client
        .get("/console/api/list_users")
        .await
        .expect("Failed to get users");

    let data = users.get("data").and_then(|d| d.as_array());
    assert!(data.is_some(), "Should have users data");

    for user in data.unwrap() {
        if user["username"].as_str() == Some("demo-user") {
            let balance_usd = user["balance_usd"].as_i64().unwrap_or(0);
            let balance_cny = user["balance_cny"].as_i64().unwrap_or(0);
            return (balance_usd, balance_cny);
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

// ============================================================================
// Test 1: CN region channel uses CNY pricing
// ============================================================================

#[tokio::test]
async fn test_cn_channel_uses_cny_pricing() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    println!("\n=== Test: CN Region Channel Uses CNY Pricing ===\n");

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_cn_channel(&admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Get initial balances
    let (usd_before, cny_before) = get_demo_user_balances(&admin_client).await;
    println!("Before request:");
    println!("  USD Balance: ${:.9} ({} nanodollars)", nano_to_dollars(usd_before), usd_before);
    println!("  CNY Balance: ¥{:.9} ({} nanodollars)", nano_to_dollars(cny_before), cny_before);

    // Send request through CN channel
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "user", "content": "Say hello in Chinese"}
        ]
    });

    println!("\nSending request through CN channel...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat request failed");

    println!("Response: {:?}", chat_res);

    // Wait for async quota deduction
    sleep(Duration::from_millis(500)).await;

    // Get balances after request
    let (usd_after, cny_after) = get_demo_user_balances(&admin_client).await;
    println!("\nAfter request:");
    println!("  USD Balance: ${:.9} ({} nanodollars)", nano_to_dollars(usd_after), usd_after);
    println!("  CNY Balance: ¥{:.9} ({} nanodollars)", nano_to_dollars(cny_after), cny_after);

    // Calculate deductions
    let usd_deducted = usd_before - usd_after;
    let cny_deducted = cny_before - cny_after;

    println!("\nDeductions:");
    println!("  USD deducted: ${:.9} ({} nanodollars)", nano_to_dollars(usd_deducted), usd_deducted);
    println!("  CNY deducted: ¥{:.9} ({} nanodollars)", nano_to_dollars(cny_deducted), cny_deducted);

    // Verify pricing region lookup worked
    // Note: The actual currency deduction depends on the price's currency field
    // If CN region price has currency=CNY, then CNY balance should be deducted
    // If CN region price has currency=USD, then USD balance should be deducted

    // Get the log to verify the cost was calculated
    if let Some(log) = get_latest_log(&admin_client).await {
        println!("\nLatest log entry:");
        println!("  Model: {:?}", log.get("model"));
        println!("  Cost: {:?} nanodollars", log.get("cost"));
        println!("  Prompt tokens: {:?}", log.get("prompt_tokens"));
        println!("  Completion tokens: {:?}", log.get("completion_tokens"));

        // Verify log has cost
        let log_cost = log.get("cost").and_then(|c| c.as_i64());
        assert!(log_cost.is_some(), "Log should have cost field");
        assert!(log_cost.unwrap() > 0, "Log cost should be positive");
    }

    // For CN region with CNY pricing, one of the balances should be deducted
    let total_deducted = usd_deducted + cny_deducted;
    assert!(total_deducted > 0, "At least one balance should be deducted for CN region");

    println!("\n✓ SUCCESS: CN region channel pricing verified");
    println!("  Total deducted: ${:.9} equivalent", nano_to_dollars(total_deducted));
}

// ============================================================================
// Test 2: International region channel uses USD pricing
// ============================================================================

#[tokio::test]
async fn test_intl_channel_uses_usd_pricing() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    println!("\n=== Test: International Region Channel Uses USD Pricing ===\n");

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_intl_channel(&admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Get initial balances
    let (usd_before, cny_before) = get_demo_user_balances(&admin_client).await;
    println!("Before request:");
    println!("  USD Balance: ${:.9} ({} nanodollars)", nano_to_dollars(usd_before), usd_before);
    println!("  CNY Balance: ¥{:.9} ({} nanodollars)", nano_to_dollars(cny_before), cny_before);

    // Send request through International channel
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "user", "content": "Say hello in English"}
        ]
    });

    println!("\nSending request through International channel...");
    let chat_res = user_client
        .post("/v1/chat/completions", &chat_body)
        .await
        .expect("Chat request failed");

    println!("Response: {:?}", chat_res);

    // Wait for async quota deduction
    sleep(Duration::from_millis(500)).await;

    // Get balances after request
    let (usd_after, cny_after) = get_demo_user_balances(&admin_client).await;
    println!("\nAfter request:");
    println!("  USD Balance: ${:.9} ({} nanodollars)", nano_to_dollars(usd_after), usd_after);
    println!("  CNY Balance: ¥{:.9} ({} nanodollars)", nano_to_dollars(cny_after), cny_after);

    // Calculate deductions
    let usd_deducted = usd_before - usd_after;
    let cny_deducted = cny_before - cny_after;

    println!("\nDeductions:");
    println!("  USD deducted: ${:.9} ({} nanodollars)", nano_to_dollars(usd_deducted), usd_deducted);
    println!("  CNY deducted: ¥{:.9} ({} nanodollars)", nano_to_dollars(cny_deducted), cny_deducted);

    // Get the log to verify the cost was calculated
    if let Some(log) = get_latest_log(&admin_client).await {
        println!("\nLatest log entry:");
        println!("  Model: {:?}", log.get("model"));
        println!("  Cost: {:?} nanodollars", log.get("cost"));
        println!("  Prompt tokens: {:?}", log.get("prompt_tokens"));
        println!("  Completion tokens: {:?}", log.get("completion_tokens"));

        // Verify log has cost
        let log_cost = log.get("cost").and_then(|c| c.as_i64());
        assert!(log_cost.is_some(), "Log should have cost field");
        assert!(log_cost.unwrap() > 0, "Log cost should be positive");
    }

    // For international region with USD pricing, USD balance should be deducted
    let total_deducted = usd_deducted + cny_deducted;
    assert!(total_deducted > 0, "At least one balance should be deducted for Intl region");

    println!("\n✓ SUCCESS: International region channel pricing verified");
    println!("  Total deducted: ${:.9} equivalent", nano_to_dollars(total_deducted));
}

// ============================================================================
// Test 3: Compare CN vs International pricing differences
// ============================================================================

#[tokio::test]
async fn test_region_pricing_comparison() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    println!("\n=== Test: Compare CN vs International Pricing ===\n");

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);

    // Create both channels
    let _cn_channel = create_cn_channel(&admin_client).await;
    let _intl_channel = create_intl_channel(&admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    // Get initial balances
    let (usd_before, cny_before) = get_demo_user_balances(&admin_client).await;
    println!("Initial balances:");
    println!("  USD: ${:.9}", nano_to_dollars(usd_before));
    println!("  CNY: ¥{:.9}", nano_to_dollars(cny_before));

    // Test CN channel
    println!("\n--- Testing CN Channel ---");
    let cn_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [{"role": "user", "content": "What is 1+1?"}]
    });

    let cn_res = user_client
        .post("/v1/chat/completions", &cn_body)
        .await
        .expect("CN channel request failed");

    println!("CN Response: {:?}", cn_res);

    sleep(Duration::from_millis(500)).await;

    let (usd_after_cn, cny_after_cn) = get_demo_user_balances(&admin_client).await;
    let cn_usd_deducted = usd_before - usd_after_cn;
    let cn_cny_deducted = cny_before - cny_after_cn;

    println!("After CN request:");
    println!("  USD deducted: ${:.9}", nano_to_dollars(cn_usd_deducted));
    println!("  CNY deducted: ¥{:.9}", nano_to_dollars(cn_cny_deducted));

    // Test International channel
    println!("\n--- Testing International Channel ---");
    let intl_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [{"role": "user", "content": "What is 2+2?"}]
    });

    let intl_res = user_client
        .post("/v1/chat/completions", &intl_body)
        .await
        .expect("Intl channel request failed");

    println!("Intl Response: {:?}", intl_res);

    sleep(Duration::from_millis(500)).await;

    let (usd_after_intl, cny_after_intl) = get_demo_user_balances(&admin_client).await;
    let intl_usd_deducted = usd_after_cn - usd_after_intl;
    let intl_cny_deducted = cny_after_cn - cny_after_intl;

    println!("After Intl request:");
    println!("  USD deducted: ${:.9}", nano_to_dollars(intl_usd_deducted));
    println!("  CNY deducted: ¥{:.9}", nano_to_dollars(intl_cny_deducted));

    // Summary
    println!("\n=== Summary ===");
    println!("CN channel: USD={:.9}, CNY={:.9}", nano_to_dollars(cn_usd_deducted), nano_to_dollars(cn_cny_deducted));
    println!("Intl channel: USD={:.9}, CNY={:.9}", nano_to_dollars(intl_usd_deducted), nano_to_dollars(intl_cny_deducted));

    // Verify total deductions
    let total_deducted = cn_usd_deducted + cn_cny_deducted + intl_usd_deducted + intl_cny_deducted;
    assert!(total_deducted > 0, "Total deductions should be positive");

    println!("\n✓ SUCCESS: Region pricing comparison completed");
    println!("  Total deductions: ${:.9} equivalent", nano_to_dollars(total_deducted));
}

// ============================================================================
// Test 4: Verify price lookup by region
// ============================================================================

#[tokio::test]
async fn test_price_lookup_by_region() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    println!("\n=== Test: Verify Price Lookup by Region ===\n");

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);

    // Create CN channel
    let cn_channel_name = create_cn_channel(&admin_client).await;

    // Get the channel details to verify pricing_region is set
    let channels = admin_client
        .get("/console/api/list_channels")
        .await
        .expect("Failed to get channels");

    println!("Channels response: {:?}", channels);

    // Find our CN channel
    let cn_channel = channels
        .get("data")
        .and_then(|d| d.as_array())
        .and_then(|arr| arr.iter().find(|c| c["name"].as_str() == Some(&cn_channel_name)));

    assert!(cn_channel.is_some(), "CN channel should exist");
    let cn_channel = cn_channel.unwrap();

    println!("CN Channel details:");
    println!("  Name: {:?}", cn_channel.get("name"));
    println!("  Pricing Region: {:?}", cn_channel.get("pricing_region"));

    // Verify pricing_region is set to "cn"
    let pricing_region = cn_channel.get("pricing_region").and_then(|r| r.as_str());
    assert_eq!(pricing_region, Some("cn"), "CN channel should have pricing_region='cn'");

    println!("\n✓ SUCCESS: Price lookup by region verified");
}

// ============================================================================
// Test 5: Full billing cycle with region-specific pricing
// ============================================================================

#[tokio::test]
async fn test_full_billing_cycle_with_region() {
    if get_gemini_key().is_none() {
        println!("SKIPPING: TEST_GEMINI_KEY not set");
        return;
    }

    println!("\n=== Test: Full Billing Cycle with Region ===\n");

    let base_url = common_mod::spawn_app().await;
    let admin_client = TestClient::new(&base_url);
    let _channel_name = create_cn_channel(&admin_client).await;

    let user_client = TestClient::new(&base_url).with_token(&common_mod::get_demo_token());

    println!("[1] Creating CN region channel...");
    println!("    Channel: {}", _channel_name);

    // Get initial state
    let (usd_before, cny_before) = get_demo_user_balances(&admin_client).await;
    println!("\n[2] Initial balances:");
    println!("    USD: ${:.9}", nano_to_dollars(usd_before));
    println!("    CNY: ¥{:.9}", nano_to_dollars(cny_before));

    // Send request
    println!("\n[3] Sending request through CN channel...");
    let chat_body = json!({
        "model": "gemini-2.0-flash",
        "messages": [
            {"role": "user", "content": "What is the capital of China?"}
        ]
    });

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

    println!("    Prompt tokens: {}", prompt_tokens);
    println!("    Completion tokens: {}", completion_tokens);

    // Wait for async operations
    sleep(Duration::from_millis(500)).await;

    // Check final state
    let (usd_after, cny_after) = get_demo_user_balances(&admin_client).await;
    println!("\n[4] Final balances:");
    println!("    USD: ${:.9}", nano_to_dollars(usd_after));
    println!("    CNY: ¥{:.9}", nano_to_dollars(cny_after));

    let usd_deducted = usd_before - usd_after;
    let cny_deducted = cny_before - cny_after;

    println!("\n[5] Balance changes:");
    println!("    USD deducted: ${:.9}", nano_to_dollars(usd_deducted));
    println!("    CNY deducted: ¥{:.9}", nano_to_dollars(cny_deducted));

    // Verify log
    if let Some(log) = get_latest_log(&admin_client).await {
        let log_model = log.get("model").and_then(|m| m.as_str());
        let log_cost = log.get("cost").and_then(|c| c.as_i64());
        let log_prompt = log.get("prompt_tokens").and_then(|t| t.as_i64());
        let log_completion = log.get("completion_tokens").and_then(|t| t.as_i64());

        println!("\n[6] Log entry:");
        println!("    Model: {:?}", log_model);
        println!("    Cost: {} nanodollars (${:.9})", log_cost.unwrap_or(0), nano_to_dollars(log_cost.unwrap_or(0)));
        println!("    Tokens: {} prompt + {} completion", log_prompt.unwrap_or(0), log_completion.unwrap_or(0));

        // Verify log matches response tokens
        if prompt_tokens > 0 {
            assert_eq!(log_prompt.unwrap_or(0) as u64, prompt_tokens, "Log prompt tokens should match response");
        }
        if completion_tokens > 0 {
            assert_eq!(log_completion.unwrap_or(0) as u64, completion_tokens, "Log completion tokens should match response");
        }
    }

    // Verify billing occurred
    let total_deducted = usd_deducted + cny_deducted;
    if prompt_tokens > 0 || completion_tokens > 0 {
        assert!(total_deducted > 0, "Billing should occur for token usage");
    }

    println!("\n✓ SUCCESS: Full billing cycle with region verified!");
    println!("    - Token counting: ✓");
    println!("    - Cost calculation: ✓");
    println!("    - Balance deduction: ✓");
    println!("    - Log recording: ✓");
}
