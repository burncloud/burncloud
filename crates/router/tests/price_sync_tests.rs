mod common;

use burncloud_database_models::{PriceInput, PriceModel};
use burncloud_router::price_sync::LiteLLMPrice;
use common::setup_db;

/// Test that LiteLLM price conversion correctly handles advanced pricing fields
#[tokio::test]
async fn test_litellm_advanced_pricing_sync() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Simulate a LiteLLM price with cache pricing
    let litellm_price = LiteLLMPrice {
        model: Some("test-cache-model".to_string()),
        input_cost_per_token: Some(3e-6),      // $3/1M
        output_cost_per_token: Some(15e-6),    // $15/1M
        cache_read_input_token_cost: Some(3e-7), // $0.30/1M (10% of input)
        cache_creation_input_token_cost: Some(3.75e-6), // $3.75/1M
        input_cost_per_token_batches: Some(1.5e-6), // $1.5/1M (50% of input)
        output_cost_per_token_batches: Some(7.5e-6), // $7.5/1M (50% of output)
        input_cost_per_token_priority: Some(5.1e-6), // $5.1/1M (170% of input)
        output_cost_per_token_priority: Some(25.5e-6), // $25.5/1M (170% of output)
        input_cost_per_audio_token: Some(21e-6), // $21/1M (7x input)
        max_input_tokens: Some(200000),
        max_output_tokens: Some(8192),
        pricing_model: None,
        supports_vision: Some(true),
        supports_function_calling: Some(true),
        mode: Some("chat".to_string()),
        search_context_cost_per_query: None,
    };

    // Convert to per-million prices
    let (input_price, output_price) = litellm_price.to_per_million_price();
    let (cache_read_price, cache_creation_price) = litellm_price.to_cache_per_million_price();
    let (batch_input_price, batch_output_price) = litellm_price.to_batch_per_million_price();
    let (priority_input_price, priority_output_price) = litellm_price.to_priority_per_million_price();
    let audio_input_price = litellm_price.to_audio_per_million_price();

    // Create price input with advanced pricing
    let price_input = PriceInput {
        model: "test-cache-model".to_string(),
        input_price: input_price.unwrap(),
        output_price: output_price.unwrap(),
        currency: Some("USD".to_string()),
        alias_for: None,
        cache_read_price,
        cache_creation_price,
        batch_input_price,
        batch_output_price,
        priority_input_price,
        priority_output_price,
        audio_input_price,
        full_pricing: None,
        original_currency: None,
        original_input_price: None,
        original_output_price: None,
    };

    // Upsert to database
    PriceModel::upsert(&_db, &price_input).await?;

    // Retrieve and verify
    let stored = PriceModel::get(&_db, "test-cache-model").await?;
    assert!(stored.is_some(), "Price should be stored");

    let stored = stored.unwrap();
    assert!((stored.input_price - 3.0).abs() < 0.001);
    assert!((stored.output_price - 15.0).abs() < 0.001);
    assert!((stored.cache_read_price.unwrap() - 0.30).abs() < 0.001);
    assert!((stored.cache_creation_price.unwrap() - 3.75).abs() < 0.001);
    assert!((stored.batch_input_price.unwrap() - 1.5).abs() < 0.001);
    assert!((stored.batch_output_price.unwrap() - 7.5).abs() < 0.001);
    assert!((stored.priority_input_price.unwrap() - 5.1).abs() < 0.001);
    assert!((stored.priority_output_price.unwrap() - 25.5).abs() < 0.001);
    assert!((stored.audio_input_price.unwrap() - 21.0).abs() < 0.001);

    Ok(())
}

/// Test that prices with NULL advanced fields are handled correctly
#[tokio::test]
async fn test_litellm_basic_pricing_sync() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Simulate a basic LiteLLM price without advanced pricing
    let litellm_price = LiteLLMPrice {
        model: Some("test-basic-model".to_string()),
        input_cost_per_token: Some(1e-6),   // $1/1M
        output_cost_per_token: Some(3e-6),  // $3/1M
        // No cache/batch/priority pricing
        cache_read_input_token_cost: None,
        cache_creation_input_token_cost: None,
        input_cost_per_token_batches: None,
        output_cost_per_token_batches: None,
        input_cost_per_token_priority: None,
        output_cost_per_token_priority: None,
        input_cost_per_audio_token: None,
        max_input_tokens: Some(4096),
        max_output_tokens: Some(1024),
        pricing_model: None,
        supports_vision: None,
        supports_function_calling: None,
        mode: Some("chat".to_string()),
        search_context_cost_per_query: None,
    };

    // Convert to per-million prices
    let (input_price, output_price) = litellm_price.to_per_million_price();
    let (cache_read_price, cache_creation_price) = litellm_price.to_cache_per_million_price();
    let (batch_input_price, batch_output_price) = litellm_price.to_batch_per_million_price();
    let (priority_input_price, priority_output_price) = litellm_price.to_priority_per_million_price();
    let audio_input_price = litellm_price.to_audio_per_million_price();

    // Create price input
    let price_input = PriceInput {
        model: "test-basic-model".to_string(),
        input_price: input_price.unwrap(),
        output_price: output_price.unwrap(),
        currency: Some("USD".to_string()),
        alias_for: None,
        cache_read_price,
        cache_creation_price,
        batch_input_price,
        batch_output_price,
        priority_input_price,
        priority_output_price,
        audio_input_price,
        full_pricing: None,
        original_currency: None,
        original_input_price: None,
        original_output_price: None,
    };

    // Upsert to database
    PriceModel::upsert(&_db, &price_input).await?;

    // Retrieve and verify
    let stored = PriceModel::get(&_db, "test-basic-model").await?;
    assert!(stored.is_some(), "Price should be stored");

    let stored = stored.unwrap();
    assert!((stored.input_price - 1.0).abs() < 0.001);
    assert!((stored.output_price - 3.0).abs() < 0.001);
    // Advanced pricing should be NULL
    assert_eq!(stored.cache_read_price, None);
    assert_eq!(stored.cache_creation_price, None);
    assert_eq!(stored.batch_input_price, None);
    assert_eq!(stored.batch_output_price, None);
    assert_eq!(stored.priority_input_price, None);
    assert_eq!(stored.priority_output_price, None);
    assert_eq!(stored.audio_input_price, None);

    Ok(())
}

/// Test that price sync updates existing records correctly
#[tokio::test]
async fn test_litellm_pricing_update() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // First insert basic pricing
    let price_input = PriceInput {
        model: "test-update-model".to_string(),
        input_price: 10.0,
        output_price: 30.0,
        currency: Some("USD".to_string()),
        alias_for: None,
        cache_read_price: None,
        cache_creation_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        full_pricing: None,
        original_currency: None,
        original_input_price: None,
        original_output_price: None,
    };
    PriceModel::upsert(&_db, &price_input).await?;

    // Verify initial price
    let stored = PriceModel::get(&_db, "test-update-model").await?.unwrap();
    assert!((stored.input_price - 10.0).abs() < 0.001);

    // Now update with new pricing including cache pricing
    let updated_input = PriceInput {
        model: "test-update-model".to_string(),
        input_price: 3.0,
        output_price: 15.0,
        currency: Some("USD".to_string()),
        alias_for: None,
        cache_read_price: Some(0.30),
        cache_creation_price: Some(3.75),
        batch_input_price: Some(1.5),
        batch_output_price: Some(7.5),
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        full_pricing: None,
        original_currency: None,
        original_input_price: None,
        original_output_price: None,
    };
    PriceModel::upsert(&_db, &updated_input).await?;

    // Verify updated price
    let stored = PriceModel::get(&_db, "test-update-model").await?.unwrap();
    assert!((stored.input_price - 3.0).abs() < 0.001);
    assert!((stored.output_price - 15.0).abs() < 0.001);
    assert!((stored.cache_read_price.unwrap() - 0.30).abs() < 0.001);
    assert!((stored.cache_creation_price.unwrap() - 3.75).abs() < 0.001);
    assert!((stored.batch_input_price.unwrap() - 1.5).abs() < 0.001);
    assert!((stored.batch_output_price.unwrap() - 7.5).abs() < 0.001);

    Ok(())
}
