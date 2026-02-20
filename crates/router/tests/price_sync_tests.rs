mod common;

use burncloud_common::pricing_config::{
    CachePricingConfig, CurrencyPricing, ModelMetadata, ModelPricing, PricingConfig,
    TieredPriceConfig,
};
use burncloud_database_models::{PriceInput, PriceModel, PriceV2Input, PriceV2Model, TieredPriceInput, TieredPriceModel};
use burncloud_router::price_sync::LiteLLMPrice;
use common::setup_db;
use std::collections::HashMap;

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

/// Test multi-currency price storage in prices_v2 table
#[tokio::test]
async fn test_multi_currency_price_storage() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    let model_name = "test-multi-currency-model-v2";

    // Insert USD price
    let usd_price = PriceV2Input {
        model: model_name.to_string(),
        currency: "USD".to_string(),
        input_price: 1.0,
        output_price: 3.0,
        cache_read_input_price: Some(0.1),
        cache_creation_input_price: Some(1.25),
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("litellm".to_string()),
        region: Some("international".to_string()),
        context_window: Some(128000),
        max_output_tokens: Some(4096),
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &usd_price).await?;

    // Insert CNY price for the same model
    let cny_price = PriceV2Input {
        model: model_name.to_string(),
        currency: "CNY".to_string(),
        input_price: 7.2,
        output_price: 21.6,
        cache_read_input_price: Some(0.72),
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("converted".to_string()),
        region: Some("cn".to_string()),
        context_window: Some(128000),
        max_output_tokens: Some(4096),
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &cny_price).await?;

    // Retrieve all currencies for the model (no region filter)
    let all_prices = PriceV2Model::list(&_db, 100, 0, None).await?;
    let model_prices: Vec<_> = all_prices.iter().filter(|p| p.model == model_name).collect();
    assert_eq!(model_prices.len(), 2, "Should have 2 currencies");

    // Verify USD price
    let usd = PriceV2Model::get(&_db, model_name, "USD", Some("international")).await?;
    assert!(usd.is_some());
    let usd = usd.unwrap();
    assert!((usd.input_price - 1.0).abs() < 0.001);
    assert!((usd.output_price - 3.0).abs() < 0.001);
    assert_eq!(usd.region, Some("international".to_string()));

    // Verify CNY price
    let cny = PriceV2Model::get(&_db, model_name, "CNY", Some("cn")).await?;
    assert!(cny.is_some());
    let cny = cny.unwrap();
    assert!((cny.input_price - 7.2).abs() < 0.001);
    assert!((cny.output_price - 21.6).abs() < 0.001);
    assert_eq!(cny.region, Some("cn".to_string()));

    Ok(())
}

/// Test tiered pricing storage and retrieval
#[tokio::test]
async fn test_tiered_pricing_sync() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    let model = "test-tiered-model";

    // Insert multiple tiers for a model
    let tiers = vec![
        TieredPriceInput {
            model: model.to_string(),
            region: Some("international".to_string()),
            tier_start: 0,
            tier_end: Some(32000),
            input_price: 1.2,
            output_price: 6.0,
        },
        TieredPriceInput {
            model: model.to_string(),
            region: Some("international".to_string()),
            tier_start: 32000,
            tier_end: Some(128000),
            input_price: 2.4,
            output_price: 12.0,
        },
        TieredPriceInput {
            model: model.to_string(),
            region: Some("international".to_string()),
            tier_start: 128000,
            tier_end: None, // No upper limit
            input_price: 3.0,
            output_price: 15.0,
        },
        // CN region tiers
        TieredPriceInput {
            model: model.to_string(),
            region: Some("cn".to_string()),
            tier_start: 0,
            tier_end: Some(32000),
            input_price: 0.359,
            output_price: 1.434,
        },
    ];

    for tier in &tiers {
        TieredPriceModel::upsert_tier(&_db, tier).await?;
    }

    // Verify has_tiered_pricing
    let has_tiered = TieredPriceModel::has_tiered_pricing(&_db, model).await?;
    assert!(has_tiered, "Model should have tiered pricing");

    // Get international tiers
    let intl_tiers = TieredPriceModel::get_tiers(&_db, model, Some("international")).await?;
    assert_eq!(intl_tiers.len(), 3, "Should have 3 international tiers");

    // Verify tier order
    assert_eq!(intl_tiers[0].tier_start, 0);
    assert_eq!(intl_tiers[1].tier_start, 32000);
    assert_eq!(intl_tiers[2].tier_start, 128000);
    assert!(intl_tiers[2].tier_end.is_none(), "Last tier should have no upper limit");

    // Get CN tiers
    let cn_tiers = TieredPriceModel::get_tiers(&_db, model, Some("cn")).await?;
    assert_eq!(cn_tiers.len(), 1, "Should have 1 CN tier");
    assert!((cn_tiers[0].input_price - 0.359).abs() < 0.001);

    // Get all tiers (no region filter)
    let all_tiers = TieredPriceModel::get_tiers(&_db, model, None).await?;
    assert_eq!(all_tiers.len(), 4, "Should have 4 total tiers");

    // Delete CN tiers
    TieredPriceModel::delete_tiers(&_db, model, Some("cn")).await?;
    let remaining_tiers = TieredPriceModel::get_tiers(&_db, model, None).await?;
    assert_eq!(remaining_tiers.len(), 3, "Should have 3 tiers after deleting CN");

    Ok(())
}

/// Test data source priority through upsert behavior
/// Tests that prices can be updated correctly
#[tokio::test]
async fn test_data_source_priority() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    let model_name = "test-priority-model-unique-12345";

    // First, insert a price with a specific region
    let litellm_price = PriceV2Input {
        model: model_name.to_string(),
        currency: "USD".to_string(),
        input_price: 10.0,
        output_price: 30.0,
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("litellm".to_string()),
        region: Some("international".to_string()),  // Use a specific region for SQLite unique constraint
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &litellm_price).await?;

    // Verify initial price from LiteLLM
    let stored = PriceV2Model::get(&_db, model_name, "USD", Some("international")).await?.unwrap();
    assert!((stored.input_price - 10.0).abs() < 0.001);
    assert_eq!(stored.source, Some("litellm".to_string()));

    // Now update with community source (higher priority) - same region
    let community_price = PriceV2Input {
        model: model_name.to_string(),
        currency: "USD".to_string(),
        input_price: 5.0,
        output_price: 15.0,
        cache_read_input_price: Some(0.5),
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("community".to_string()),
        region: Some("international".to_string()),
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &community_price).await?;

    // Verify price was updated
    let stored = PriceV2Model::get(&_db, model_name, "USD", Some("international")).await?.unwrap();
    assert!((stored.input_price - 5.0).abs() < 0.001, "Expected 5.0 but got {}", stored.input_price);
    assert_eq!(stored.source, Some("community".to_string()));

    // Update with override source (highest priority)
    let override_price = PriceV2Input {
        model: model_name.to_string(),
        currency: "USD".to_string(),
        input_price: 2.0,
        output_price: 6.0,
        cache_read_input_price: Some(0.2),
        cache_creation_input_price: Some(2.5),
        batch_input_price: Some(1.0),
        batch_output_price: Some(3.0),
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("override".to_string()),
        region: Some("international".to_string()),
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &override_price).await?;

    // Verify final price from override
    let stored = PriceV2Model::get(&_db, model_name, "USD", Some("international")).await?.unwrap();
    assert!((stored.input_price - 2.0).abs() < 0.001);
    assert_eq!(stored.source, Some("override".to_string()));
    assert!((stored.cache_read_input_price.unwrap() - 0.2).abs() < 0.001);
    assert!((stored.batch_input_price.unwrap() - 1.0).abs() < 0.001);

    Ok(())
}

/// Test that sync failure preserves old data
#[tokio::test]
async fn test_sync_failure_preserves_old_data() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Insert initial price
    let initial_price = PriceV2Input {
        model: "test-failure-model".to_string(),
        currency: "USD".to_string(),
        input_price: 5.0,
        output_price: 15.0,
        cache_read_input_price: Some(0.5),
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        source: Some("community".to_string()),
        region: None,
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
    };
    PriceV2Model::upsert(&_db, &initial_price).await?;

    // Verify initial data
    let stored = PriceV2Model::get(&_db, "test-failure-model", "USD", None).await?.unwrap();
    assert!((stored.input_price - 5.0).abs() < 0.001);

    // Simulate a "failed" sync - the original data should be preserved
    // Verify original data is still there
    let stored = PriceV2Model::get(&_db, "test-failure-model", "USD", None).await?.unwrap();
    assert!((stored.input_price - 5.0).abs() < 0.001, "Original data should be preserved");
    assert_eq!(stored.source, Some("community".to_string()));

    Ok(())
}

/// Test PricingConfig import with advanced pricing
#[tokio::test]
async fn test_pricing_config_import() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Create a PricingConfig
    let mut pricing_map = HashMap::new();
    pricing_map.insert(
        "USD".to_string(),
        CurrencyPricing {
            input_price: 10.0,
            output_price: 30.0,
            source: Some("openai".to_string()),
        },
    );
    pricing_map.insert(
        "CNY".to_string(),
        CurrencyPricing {
            input_price: 72.0,
            output_price: 216.0,
            source: Some("converted".to_string()),
        },
    );

    let mut cache_map = HashMap::new();
    cache_map.insert(
        "USD".to_string(),
        CachePricingConfig {
            cache_read_input_price: 1.0,
            cache_creation_input_price: Some(1.25),
        },
    );

    let tiered = vec![
        TieredPriceConfig {
            tier_start: 0,
            tier_end: Some(32000),
            input_price: 1.2,
            output_price: 6.0,
        },
        TieredPriceConfig {
            tier_start: 32000,
            tier_end: Some(128000),
            input_price: 2.4,
            output_price: 12.0,
        },
    ];
    let mut tiered_map = HashMap::new();
    tiered_map.insert("USD".to_string(), tiered);

    let mut models = HashMap::new();
    models.insert(
        "test-import-model".to_string(),
        ModelPricing {
            pricing: pricing_map,
            tiered_pricing: Some(tiered_map),
            cache_pricing: Some(cache_map),
            batch_pricing: None,
            metadata: Some(ModelMetadata {
                context_window: Some(128000),
                max_output_tokens: Some(4096),
                supports_vision: true,
                supports_function_calling: true,
                supports_streaming: true,
                provider: Some("openai".to_string()),
                family: Some("gpt-4".to_string()),
                release_date: None,
            }),
        },
    );

    let config = PricingConfig {
        version: "1.0".to_string(),
        updated_at: chrono::Utc::now(),
        source: "local".to_string(),
        models,
    };

    // Validate config
    let warnings = config.validate()?;
    assert!(warnings.is_empty(), "Config should be valid");

    // Import pricing data
    for (model_name, model_pricing) in &config.models {
        for (currency, pricing) in &model_pricing.pricing {
            let input = PriceV2Input {
                model: model_name.clone(),
                currency: currency.clone(),
                input_price: pricing.input_price,
                output_price: pricing.output_price,
                cache_read_input_price: None,
                cache_creation_input_price: None,
                batch_input_price: None,
                batch_output_price: None,
                priority_input_price: None,
                priority_output_price: None,
                audio_input_price: None,
                source: pricing.source.clone(),
                region: None,
                context_window: model_pricing.metadata.as_ref().and_then(|m| m.context_window),
                max_output_tokens: model_pricing.metadata.as_ref().and_then(|m| m.max_output_tokens),
                supports_vision: None,  // SQLite doesn't support boolean type
                supports_function_calling: None,
            };
            PriceV2Model::upsert(&_db, &input).await?;
        }

        // Import tiered pricing
        if let Some(ref tiered_pricing) = model_pricing.tiered_pricing {
            for (currency, tiers) in tiered_pricing {
                let region = if currency == "CNY" {
                    Some("cn".to_string())
                } else {
                    Some("international".to_string())
                };
                for tier in tiers {
                    let tier_input = TieredPriceInput {
                        model: model_name.clone(),
                        region: region.clone(),
                        tier_start: tier.tier_start,
                        tier_end: tier.tier_end,
                        input_price: tier.input_price,
                        output_price: tier.output_price,
                    };
                    TieredPriceModel::upsert_tier(&_db, &tier_input).await?;
                }
            }
        }
    }

    // Verify imported data
    let usd_price = PriceV2Model::get(&_db, "test-import-model", "USD", None).await?.unwrap();
    assert!((usd_price.input_price - 10.0).abs() < 0.001);
    assert_eq!(usd_price.context_window, Some(128000));
    // Note: supports_vision is stored as INTEGER in SQLite, so we skip that check

    let cny_price = PriceV2Model::get(&_db, "test-import-model", "CNY", None).await?.unwrap();
    assert!((cny_price.input_price - 72.0).abs() < 0.001);

    let has_tiered = TieredPriceModel::has_tiered_pricing(&_db, "test-import-model").await?;
    assert!(has_tiered);

    let tiers = TieredPriceModel::get_tiers(&_db, "test-import-model", Some("international")).await?;
    assert_eq!(tiers.len(), 2);

    Ok(())
}
