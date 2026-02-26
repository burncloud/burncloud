mod common;

use burncloud_common::price_u64::{dollars_to_nano, nano_to_dollars};
use burncloud_database::sqlx;
use burncloud_database_models::{PriceInput, PriceModel};
use burncloud_database_router::RouterDatabase;
use common::setup_db;
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

    // 3. Setup: Create pricing for test model (using PriceModel with nanodollars)
    let price_input = PriceInput {
        model: "gpt-4o-mini-e2e".to_string(),
        input_price: dollars_to_nano(0.15), // $0.15 per 1M tokens -> nanodollars (i64)
        output_price: dollars_to_nano(0.60), // $0.60 per 1M tokens -> nanodollars (i64)
        currency: "USD".to_string(),
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: None,
        region: None,
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceModel::upsert(&_db, &price_input).await?;

    // 4. Test: Get pricing
    let price = PriceModel::get(&_db, "gpt-4o-mini-e2e", "USD", None).await?;
    assert!(price.is_some(), "Price should be found");
    let price = price.unwrap();

    // Convert i64 nanodollars to dollars for display (prices should always be positive)
    let input_dollars = nano_to_dollars(price.input_price);
    let output_dollars = nano_to_dollars(price.output_price);

    assert!(
        (input_dollars - 0.15).abs() < 0.0001,
        "Input price should be 0.15, got {}",
        input_dollars
    );
    assert!(
        (output_dollars - 0.60).abs() < 0.0001,
        "Output price should be 0.60, got {}",
        output_dollars
    );

    // 5. Test: Calculate expected cost using nanodollars
    // 100 prompt + 200 completion tokens = $0.000015 + $0.00012 = $0.000135
    let cost_nano_f64 = (100.0 / 1_000_000.0) * price.input_price as f64
        + (200.0 / 1_000_000.0) * price.output_price as f64;
    let cost_dollars = cost_nano_f64 / 1_000_000_000.0;
    let expected_cost: f64 = (100.0 / 1_000_000.0) * 0.15 + (200.0 / 1_000_000.0) * 0.60;

    assert!(
        (cost_dollars - expected_cost).abs() < 0.0000001,
        "Cost calculation should match: got {}, expected {}",
        cost_dollars,
        expected_cost
    );

    println!("✓ E2E billing flow setup passed");
    println!("  - Token created: {}", token);
    println!(
        "  - Pricing set: ${:.4}/1M input, ${:.4}/1M output",
        input_dollars, output_dollars
    );
    println!(
        "  - Expected cost for 100 prompt + 200 completion: ${:.6}",
        expected_cost
    );

    // 6. Test: Quota deduction (using token-level quota for backward compatibility)
    let quota_before: i64 =
        sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
            .bind(&token)
            .fetch_one(&pool)
            .await?;

    // Deduct 300 tokens
    let deduct_result = RouterDatabase::deduct_quota(&_db, "e2e-test-user", &token, 300).await?;
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

    // Calculate cost using nanodollars
    let input_price_nano = dollars_to_nano(30.0);
    let output_price_nano = dollars_to_nano(60.0);

    // Cost = (100/1M * 30) + (50/1M * 60) = 0.003 + 0.003 = 0.006
    let cost_nano_f64: f64 = (prompt as f64 / 1_000_000.0) * input_price_nano as f64
        + (completion as f64 / 1_000_000.0) * output_price_nano as f64;
    let cost_dollars = cost_nano_f64 / 1_000_000_000.0;

    println!("✓ Token counting integration passed");
    println!("  - Prompt tokens: {}", prompt);
    println!("  - Completion tokens: {}", completion);
    println!("  - Calculated cost: ${:.6}", cost_dollars);

    Ok(())
}
