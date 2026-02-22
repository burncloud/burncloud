mod common;

use burncloud_common::dollars_to_nano;
use burncloud_database_models::{PriceV2Input, PriceV2Model};
use common::setup_db;

/// Helper to convert dollars to nanodollars as i64
fn to_nano(price: f64) -> i64 {
    dollars_to_nano(price) as i64
}

/// Helper to convert nanodollars back to dollars
fn from_nano(nano: i64) -> f64 {
    nano as f64 / 1_000_000_000.0
}

/// Test pricing logic: verify cost calculation matches expected formula
/// Formula: cost = (prompt_tokens * input_price / 1M) + (completion_tokens * output_price / 1M)
#[tokio::test]
async fn test_pricing_cost_calculation() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Set up pricing for test model
    // Input: $30/1M tokens, Output: $60/1M tokens (like GPT-4)
    // All prices are stored as i64 nanodollars
    let input = PriceV2Input {
        model: "test-pricing-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(30.0),
        output_price: to_nano(60.0),
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("test".to_string()),
        region: Some("international".to_string()),
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &input).await?;

    // Get the price and verify
    let price = PriceV2Model::get(&_db, "test-pricing-model", "USD", Some("international")).await?;
    assert!(price.is_some(), "Price should be found");

    let price = price.unwrap();
    assert_eq!(price.input_price, to_nano(30.0));
    assert_eq!(price.output_price, to_nano(60.0));

    // Calculate cost for 100 prompt + 200 completion tokens
    // Expected: (100 * 30 / 1_000_000) + (200 * 60 / 1_000_000)
    //         = 0.003 + 0.012 = 0.015
    let cost_nano = PriceV2Model::calculate_cost(&price, 100, 200);
    let cost = from_nano(cost_nano);
    let expected_cost = (100.0 / 1_000_000.0) * 30.0 + (200.0 / 1_000_000.0) * 60.0;

    println!("Calculated cost: ${:.6}", cost);
    println!("Expected cost: ${:.6}", expected_cost);

    assert!(
        (cost - expected_cost).abs() < 0.000001,
        "Cost calculation should match"
    );

    Ok(())
}

/// Test price listing
#[tokio::test]
async fn test_pricing_list() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // List all prices (should include default pricing)
    let prices = PriceV2Model::list(&_db, 100, 0, None).await?;

    println!("Found {} prices", prices.len());

    // Default prices should include at least some models
    assert!(!prices.is_empty(), "Should have some prices");

    // Print all prices for debugging
    for price in &prices {
        println!(
            "  {} -> ${:.4}/1M input, ${:.4}/1M output (region: {:?})",
            price.model,
            from_nano(price.input_price),
            from_nano(price.output_price),
            price.region
        );
    }

    Ok(())
}

/// Test price delete and recreate
#[tokio::test]
async fn test_pricing_delete_and_recreate() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Create a test model
    let input = PriceV2Input {
        model: "test-delete-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(10.0),
        output_price: to_nano(20.0),
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("test".to_string()),
        region: Some("international".to_string()),
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &input).await?;

    // Verify it exists
    let price = PriceV2Model::get(&_db, "test-delete-model", "USD", Some("international")).await?;
    assert!(price.is_some());

    // Delete it
    PriceV2Model::delete(&_db, "test-delete-model", "USD", Some("international")).await?;

    // Verify it's gone
    let price = PriceV2Model::get(&_db, "test-delete-model", "USD", Some("international")).await?;
    assert!(price.is_none());

    // Recreate with different price
    let input2 = PriceV2Input {
        model: "test-delete-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(50.0),
        output_price: to_nano(100.0),
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("test".to_string()),
        region: Some("international".to_string()),
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &input2).await?;

    // Verify new price
    let price = PriceV2Model::get(&_db, "test-delete-model", "USD", Some("international")).await?;
    assert!(price.is_some());
    let price = price.unwrap();
    assert_eq!(price.input_price, to_nano(50.0));
    assert_eq!(price.output_price, to_nano(100.0));

    Ok(())
}
