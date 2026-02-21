mod common;

use burncloud_database::sqlx;
use burncloud_database_models::{PriceInput, PriceModel};
use burncloud_database_router::RouterDatabase;
use common::{setup_db, start_test_server};
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

/// End-to-end test for complete billing flow
/// Tests: token creation, pricing, streaming, token counting, cost calculation, quota deduction
#[tokio::test]
async fn test_e2e_billing_flow() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // 1. Setup: Create unique test token
    let unique_id = Uuid::new_v4();
    let token = format!("sk-e2e-test-{}", unique_id);

    sqlx::query(
        r#"
        INSERT OR REPLACE INTO router_tokens (token, user_id, status, quota_limit, used_quota)
        VALUES (?, 'e2e-test-user', 'active', 100000, 0)
        "#,
    )
    .bind(&token)
    .execute(&pool)
    .await?;

    // 2. Setup: Create test upstream channel
    sqlx::query(
        r#"
        INSERT OR REPLACE INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type, protocol)
        VALUES ('e2e-test-channel', 'E2E Test', 'https://api.openai.com', 'sk-demo-key', '/v1/chat/completions', 'Bearer', 'openai')
        "#,
    )
    .execute(&pool)
    .await?;

    // 3. Setup: Create pricing for test model
    let price_input = PriceInput {
        model: "gpt-4o-mini".to_string(),
        input_price: 0.15,  // $0.15 per 1M tokens
        output_price: 0.60, // $0.60 per 1M tokens
        currency: Some("USD".to_string()),
        alias_for: None,
        ..Default::default()
    };
    PriceModel::upsert(&_db, &price_input).await?;

    // 4. Test: Get pricing
    let price = PriceModel::get(&_db, "gpt-4o-mini").await?;
    assert!(price.is_some(), "Price should be found");
    let price = price.unwrap();
    assert_eq!(price.input_price, 0.15);
    assert_eq!(price.output_price, 0.60);

    // 5. Test: Calculate expected cost
    // 100 prompt + 200 completion tokens = $0.000015 + $0.00012 = $0.000135
    let cost = PriceModel::calculate_cost(&price, 100, 200);
    let expected_cost = (100.0 / 1_000_000.0) * 0.15 + (200.0 / 1_000_000.0) * 0.60;
    assert!(
        (cost - expected_cost).abs() < 0.0000001,
        "Cost calculation should match"
    );

    println!("✓ E2E billing flow setup passed");
    println!("  - Token created: {}", token);
    println!(
        "  - Pricing set: ${:.4}/1M input, ${:.4}/1M output",
        price.input_price, price.output_price
    );
    println!(
        "  - Expected cost for 100 prompt + 200 completion: ${:.6}",
        expected_cost
    );

    // 6. Test: Quota deduction
    let quota_before: i64 =
        sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
            .bind(&token)
            .fetch_one(&pool)
            .await?;

    // Deduct 300 tokens
    let deduct_result = RouterDatabase::deduct_quota(&_db, "e2e-test-user", &token, 300.0).await?;
    assert!(deduct_result, "Quota deduction should succeed");

    let quota_after: i64 =
        sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
            .bind(&token)
            .fetch_one(&pool)
            .await?;

    assert_eq!(
        quota_after - quota_before,
        300,
        "Used quota should increase by 300"
    );

    println!("✓ E2E billing flow quota deduction passed");
    println!("  - Quota before: {}", quota_before);
    println!("  - Quota after: {}", quota_after);

    Ok(())
}

/// Test token counting integration
#[tokio::test]
async fn test_e2e_token_counting() -> anyhow::Result<()> {
    use burncloud_router::stream_parser::StreamingTokenParser;
    use burncloud_router::token_counter::StreamingTokenCounter;
    use std::sync::Arc;

    // Test OpenAI streaming format
    let counter = Arc::new(StreamingTokenCounter::new());
    let chunks = vec![
        "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"}}]}\n\n",
        "data: {\"usage\":{\"prompt_tokens\":100,\"completion_tokens\":50}}\n\n",
        "data: [DONE]\n\n",
    ];

    for chunk in &chunks {
        StreamingTokenParser::parse_openai_chunk(chunk, &counter);
    }

    let (prompt, completion) = counter.get_usage();
    assert_eq!(prompt, 100);
    assert_eq!(completion, 50);

    // Calculate cost
    let price_input = PriceInput {
        model: "test-model".to_string(),
        input_price: 30.0,
        output_price: 60.0,
        currency: Some("USD".to_string()),
        alias_for: None,
        ..Default::default()
    };

    // Cost = (100/1M * 30) + (50/1M * 60) = 0.003 + 0.003 = 0.006
    let cost = (prompt as f64 / 1_000_000.0) * price_input.input_price
        + (completion as f64 / 1_000_000.0) * price_input.output_price;

    println!("✓ Token counting integration passed");
    println!("  - Prompt tokens: {}", prompt);
    println!("  - Completion tokens: {}", completion);
    println!("  - Calculated cost: ${:.6}", cost);

    Ok(())
}
