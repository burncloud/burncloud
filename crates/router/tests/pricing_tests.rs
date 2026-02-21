mod common;

use burncloud_database_models::{PriceInput, PriceModel};
use common::setup_db;

/// Test pricing logic: verify cost calculation matches expected formula
/// Formula: cost = (prompt_tokens * input_price / 1M) + (completion_tokens * output_price / 1M)
#[tokio::test]
async fn test_pricing_cost_calculation() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Set up pricing for test model
    // Input: $30/1M tokens, Output: $60/1M tokens (like GPT-4)
    let input = PriceInput {
        model: "test-pricing-model".to_string(),
        input_price: 30.0,
        output_price: 60.0,
        currency: Some("USD".to_string()),
        alias_for: None,
        ..Default::default()
    };
    PriceModel::upsert(&_db, &input).await?;

    // Get the price and verify
    let price = PriceModel::get(&_db, "test-pricing-model").await?;
    assert!(price.is_some(), "Price should be found");

    let price = price.unwrap();
    assert_eq!(price.input_price, 30.0);
    assert_eq!(price.output_price, 60.0);

    // Calculate cost for 100 prompt + 200 completion tokens
    // Expected: (100 * 30 / 1_000_000) + (200 * 60 / 1_000_000)
    //         = 0.003 + 0.012 = 0.015
    let cost = PriceModel::calculate_cost(&price, 100, 200);
    let expected_cost = (100.0 / 1_000_000.0) * 30.0 + (200.0 / 1_000_000.0) * 60.0;

    println!("Calculated cost: ${:.6}", cost);
    println!("Expected cost: ${:.6}", expected_cost);

    assert!(
        (cost - expected_cost).abs() < 0.000001,
        "Cost calculation should match"
    );

    Ok(())
}

/// Test model alias resolution for pricing
#[tokio::test]
async fn test_pricing_alias_resolution() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Set up base model pricing
    let base_input = PriceInput {
        model: "gpt-4".to_string(),
        input_price: 30.0,
        output_price: 60.0,
        currency: Some("USD".to_string()),
        alias_for: None,
        ..Default::default()
    };
    PriceModel::upsert(&_db, &base_input).await?;

    // Set up alias model
    let alias_input = PriceInput {
        model: "gpt-4-turbo".to_string(),
        input_price: 0.0,  // Not used
        output_price: 0.0, // Not used
        currency: Some("USD".to_string()),
        alias_for: Some("gpt-4".to_string()),
        ..Default::default()
    };
    PriceModel::upsert(&_db, &alias_input).await?;

    // Get price for alias - should resolve to base model's price
    let price = PriceModel::get(&_db, "gpt-4-turbo").await?;
    assert!(price.is_some(), "Price should be found via alias");

    let price = price.unwrap();
    assert_eq!(price.model, "gpt-4", "Should resolve to base model");
    assert_eq!(price.input_price, 30.0);
    assert_eq!(price.output_price, 60.0);

    Ok(())
}

/// Test price listing
#[tokio::test]
async fn test_pricing_list() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // List all prices (should include default pricing)
    let prices = PriceModel::list(&_db, 100, 0).await?;

    println!("Found {} prices", prices.len());

    // Default prices should include at least some models
    assert!(!prices.is_empty(), "Should have some prices");

    // Print all prices for debugging
    for price in &prices {
        println!(
            "  {} -> ${:.4}/1M input, ${:.4}/1M output",
            price.model, price.input_price, price.output_price
        );
    }

    Ok(())
}

/// Test price delete and recreate
#[tokio::test]
async fn test_pricing_delete_and_recreate() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Create a test model
    let input = PriceInput {
        model: "test-delete-model".to_string(),
        input_price: 10.0,
        output_price: 20.0,
        currency: Some("USD".to_string()),
        alias_for: None,
        ..Default::default()
    };
    PriceModel::upsert(&_db, &input).await?;

    // Verify it exists
    let price = PriceModel::get(&_db, "test-delete-model").await?;
    assert!(price.is_some());

    // Delete it
    PriceModel::delete(&_db, "test-delete-model").await?;

    // Verify it's gone
    let price = PriceModel::get(&_db, "test-delete-model").await?;
    assert!(price.is_none());

    // Recreate with different price
    let input2 = PriceInput {
        model: "test-delete-model".to_string(),
        input_price: 50.0,
        output_price: 100.0,
        currency: Some("USD".to_string()),
        alias_for: None,
        ..Default::default()
    };
    PriceModel::upsert(&_db, &input2).await?;

    // Verify new price
    let price = PriceModel::get(&_db, "test-delete-model").await?;
    assert!(price.is_some());
    let price = price.unwrap();
    assert_eq!(price.input_price, 50.0);
    assert_eq!(price.output_price, 100.0);

    Ok(())
}
