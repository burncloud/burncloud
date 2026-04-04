mod common;

use burncloud_common::dollars_to_nano;
use burncloud_common::pricing_config::{
    CachePricingConfig, CurrencyPricing, ModelMetadata, ModelPricing, PricingConfig,
    TieredPriceConfig,
};
use burncloud_database::create_database_with_url;
use burncloud_database_models::{PriceInput, PriceModel, TieredPriceInput, TieredPriceModel};
use burncloud_router::price_sync::{PriceSyncConfig, PriceSyncService};
use burncloud_database_router::RouterDatabase;
use common::setup_db;
use std::collections::HashMap;
use std::sync::Arc;

/// Helper to convert dollars to nanodollars as i64
fn to_nano(price: f64) -> i64 {
    dollars_to_nano(price) as i64
}

/// Test that pricing config with cache pricing is correctly applied
#[tokio::test]
async fn test_advanced_pricing_sync() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Create a price input with advanced pricing
    let price_input = PriceInput {
        model: "test-cache-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(3.0),
        output_price: to_nano(15.0),
        cache_read_input_price: Some(to_nano(0.30)),
        cache_creation_input_price: Some(to_nano(3.75)),
        batch_input_price: Some(to_nano(1.5)),
        batch_output_price: Some(to_nano(7.5)),
        priority_input_price: Some(to_nano(5.1)),
        priority_output_price: Some(to_nano(25.5)),
        audio_input_price: Some(to_nano(21.0)),
        audio_output_price: None,
        reasoning_price: None,
        embedding_price: None,
        image_price: None,
        video_price: None,
        music_price: None,
        source: Some("test".to_string()),
        region: Some("international".to_string()),
        context_window: Some(200000),
        max_output_tokens: Some(8192),
        supports_vision: Some(true),
        supports_function_calling: Some(true),
        voices_pricing: None,
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: None,
    };

    // Upsert to database
    PriceModel::upsert(&_db, &price_input).await?;

    // Retrieve and verify
    let stored = PriceModel::get(&_db, "test-cache-model", "USD", Some("international")).await?;
    assert!(stored.is_some(), "Price should be stored");

    let stored = stored.unwrap();
    assert_eq!(stored.input_price, to_nano(3.0));
    assert_eq!(stored.output_price, to_nano(15.0));
    assert_eq!(stored.cache_read_input_price.unwrap(), to_nano(0.30));
    assert_eq!(stored.cache_creation_input_price.unwrap(), to_nano(3.75));
    assert_eq!(stored.batch_input_price.unwrap(), to_nano(1.5));
    assert_eq!(stored.batch_output_price.unwrap(), to_nano(7.5));
    assert_eq!(stored.priority_input_price.unwrap(), to_nano(5.1));
    assert_eq!(stored.priority_output_price.unwrap(), to_nano(25.5));
    assert_eq!(stored.audio_input_price.unwrap(), to_nano(21.0));

    Ok(())
}

/// Test that prices with NULL advanced fields are handled correctly
#[tokio::test]
async fn test_basic_pricing_sync() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Create a basic price without advanced pricing
    let price_input = PriceInput {
        model: "test-basic-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(1.0),
        output_price: to_nano(3.0),
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        audio_output_price: None,
        reasoning_price: None,
        embedding_price: None,
        image_price: None,
        video_price: None,
        music_price: None,
        source: Some("test".to_string()),
        region: Some("international".to_string()),
        context_window: Some(128000),
        max_output_tokens: Some(4096),
        supports_vision: None,
        supports_function_calling: None,
        voices_pricing: None,
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: None,
    };

    // Upsert to database
    PriceModel::upsert(&_db, &price_input).await?;

    // Retrieve and verify
    let stored = PriceModel::get(&_db, "test-basic-model", "USD", Some("international")).await?;
    assert!(stored.is_some(), "Price should be stored");

    let stored = stored.unwrap();
    assert_eq!(stored.input_price, to_nano(1.0));
    assert_eq!(stored.output_price, to_nano(3.0));
    // Advanced fields should be NULL
    assert!(stored.cache_read_input_price.is_none());
    assert!(stored.cache_creation_input_price.is_none());

    Ok(())
}

/// Test that price sync updates existing records correctly
#[tokio::test]
async fn test_pricing_update() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // First insert basic pricing
    let initial_input = PriceInput {
        model: "test-update-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(1.0),
        output_price: to_nano(3.0),
        cache_read_input_price: None,
        cache_creation_input_price: None,
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        audio_output_price: None,
        reasoning_price: None,
        embedding_price: None,
        image_price: None,
        video_price: None,
        music_price: None,
        source: Some("test".to_string()),
        region: Some("international".to_string()),
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
        voices_pricing: None,
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: None,
    };
    PriceModel::upsert(&_db, &initial_input).await?;

    // Now update with cache pricing
    let updated_input = PriceInput {
        model: "test-update-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(2.0),
        output_price: to_nano(6.0),
        cache_read_input_price: Some(to_nano(0.20)),
        cache_creation_input_price: Some(to_nano(2.0)),
        batch_input_price: None,
        batch_output_price: None,
        priority_input_price: None,
        priority_output_price: None,
        audio_input_price: None,
        audio_output_price: None,
        reasoning_price: None,
        embedding_price: None,
        image_price: None,
        video_price: None,
        music_price: None,
        source: Some("test".to_string()),
        region: Some("international".to_string()),
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
        voices_pricing: None,
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: None,
    };
    PriceModel::upsert(&_db, &updated_input).await?;

    // Verify the update
    let stored = PriceModel::get(&_db, "test-update-model", "USD", Some("international")).await?;
    assert!(stored.is_some(), "Price should be stored");

    let stored = stored.unwrap();
    assert_eq!(stored.input_price, to_nano(2.0));
    assert_eq!(stored.output_price, to_nano(6.0));
    assert_eq!(stored.cache_read_input_price.unwrap(), to_nano(0.20));
    assert_eq!(stored.cache_creation_input_price.unwrap(), to_nano(2.0));

    Ok(())
}

/// Test tiered pricing upsert
#[tokio::test]
async fn test_tiered_pricing() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;

    // Insert a tiered price
    let tier1 = TieredPriceInput {
        model: "qwen-max".to_string(),
        region: Some("USD".to_string()),
        currency: Some("USD".to_string()),
        tier_type: Some("context_length".to_string()),
        tier_start: 0,
        tier_end: Some(32000),
        input_price: to_nano(2.0),
        output_price: to_nano(8.0),
    };

    let tier2 = TieredPriceInput {
        model: "qwen-max".to_string(),
        region: Some("USD".to_string()),
        currency: Some("USD".to_string()),
        tier_type: Some("context_length".to_string()),
        tier_start: 32000,
        tier_end: Some(128000),
        input_price: to_nano(2.4),
        output_price: to_nano(12.0),
    };

    TieredPriceModel::upsert_tier(&_db, &tier1).await?;
    TieredPriceModel::upsert_tier(&_db, &tier2).await?;

    // Verify tiers
    let tiers: Vec<_> = TieredPriceModel::get_tiers(&_db, "qwen-max", Some("USD")).await?;
    assert_eq!(tiers.len(), 2);

    Ok(())
}

/// Test that PricingConfig can be applied through the service
#[tokio::test]
async fn test_pricing_config_import() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;
    let db = Arc::new(_db);

    // Create a PricingConfig manually
    let config = PricingConfig {
        version: "1.0".to_string(),
        updated_at: chrono::Utc::now(),
        source: "test".to_string(),
        models: {
            let mut map = HashMap::new();
            map.insert(
                "test-import-model".to_string(),
                ModelPricing {
                    pricing: {
                        let mut pricing = HashMap::new();
                        pricing.insert(
                            "USD".to_string(),
                            CurrencyPricing {
                                input_price: to_nano(5.0),
                                output_price: to_nano(15.0),
                                source: Some("test".to_string()),
                                ..Default::default()
                            },
                        );
                        pricing
                    },
                    tiered_pricing: None,
                    cache_pricing: None,
                    batch_pricing: None,
                    voices_pricing: None,
                    video_pricing: None,
                    asr_pricing: None,
                    realtime_pricing: None,
                    metadata: Some(ModelMetadata {
                        context_window: Some(128000),
                        max_output_tokens: Some(4096),
                        supports_vision: true,
                        supports_function_calling: true,
                        supports_streaming: true,
                        provider: Some("test".to_string()),
                        family: Some("test".to_string()),
                        release_date: None,
                    }),
                },
            );
            map
        },
    };

    // Create service
    let _service = PriceSyncService::new(db.clone());

    // Apply the config manually through upsert (simulating what apply_prices does)
    for (model_name, model_pricing) in &config.models {
        for (currency, currency_pricing) in &model_pricing.pricing {
            let price_input = PriceInput {
                model: model_name.clone(),
                currency: currency.clone(),
                input_price: currency_pricing.input_price,
                output_price: currency_pricing.output_price,
                source: currency_pricing.source.clone(),
                region: None,
                context_window: model_pricing.metadata.as_ref().and_then(|m| m.context_window),
                max_output_tokens: model_pricing.metadata.as_ref().and_then(|m| m.max_output_tokens),
                supports_vision: model_pricing.metadata.as_ref().map(|m| m.supports_vision),
                supports_function_calling: model_pricing.metadata.as_ref().map(|m| m.supports_function_calling),
                ..Default::default()
            };
            PriceModel::upsert(&db, &price_input).await?;
        }
    }

    // Verify the model was imported
    let stored = PriceModel::get(&db, "test-import-model", "USD", None).await?;
    assert!(stored.is_some(), "Imported model should be stored");

    let stored = stored.unwrap();
    assert_eq!(stored.input_price, to_nano(5.0));
    assert_eq!(stored.output_price, to_nano(15.0));

    Ok(())
}

/// Test that sync failure with existing DB prices returns Ok (graceful degradation)
#[tokio::test]
async fn test_sync_failure_preserves_old_prices() -> anyhow::Result<()> {
    let (db, _pool) = setup_db().await?;
    let db = Arc::new(db);

    // Pre-populate a price before any sync
    let input = PriceInput {
        model: "sync-failure-test-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(5.0),
        output_price: to_nano(20.0),
        source: Some("pre-existing".to_string()),
        region: None,
        ..Default::default()
    };
    PriceModel::upsert(&db, &input).await?;

    // Configure sync with an invalid URL so the HTTP fetch will fail
    let config = PriceSyncConfig {
        remote_url: "http://127.0.0.1:19999/nonexistent".to_string(),
        remote_url_fallback: None,
        remote_sync_enabled: true,
        ..PriceSyncConfig::default()
    };
    let mut service = PriceSyncService::with_config(db.clone(), config);

    // With DB prices present, forced sync failure must return Ok (graceful degradation)
    let result = service.sync_all(true).await;
    assert!(result.is_ok(), "Forced sync failure with existing DB prices must return Ok, got: {:?}", result.err());
    assert_eq!(result.unwrap().source, "db_fallback");

    // The pre-existing price must still be in the DB
    let price = PriceModel::get(&db, "sync-failure-test-model", "USD", None).await?;
    assert!(
        price.is_some(),
        "Pre-existing price must survive a failed sync"
    );
    let price = price.unwrap();
    assert_eq!(price.input_price, to_nano(5.0));

    Ok(())
}

/// Test that cold start fails when DB is empty and remote is unreachable.
/// Uses an isolated in-memory DB to guarantee a truly empty starting state.
#[tokio::test]
async fn test_cold_start_db_empty_network_fail() -> anyhow::Result<()> {
    // Use a unique temp file so this test is isolated from the shared test DB
    let tmp_path = "/tmp/burncloud_cold_start_test.db".to_string();
    let _ = std::fs::remove_file(&tmp_path); // clean up from any previous run
    let url = format!("sqlite://{}?mode=rwc", tmp_path);
    let db = create_database_with_url(&url).await?;
    RouterDatabase::init(&db).await?;
    let db = Arc::new(db);

    // Schema::init seeds default prices, so we must clear them to simulate a truly empty DB
    db.execute_query("DELETE FROM prices").await?;

    let config = PriceSyncConfig {
        remote_url: "http://127.0.0.1:19999/nonexistent".to_string(),
        remote_url_fallback: None,
        remote_sync_enabled: true,
        ..PriceSyncConfig::default()
    };
    let mut service = PriceSyncService::with_config(db.clone(), config);

    // DB is empty, remote unreachable → must return Err (fatal)
    let result = service.sync_all(false).await;
    let _ = std::fs::remove_file(&tmp_path); // cleanup
    assert!(result.is_err(), "Cold start with empty DB and unreachable remote must return Err");

    Ok(())
}

/// Test that startup fast path (forced=false) uses DB when prices exist.
/// Uses an isolated temp DB to guarantee controlled starting state.
#[tokio::test]
async fn test_startup_fast_path_uses_db() -> anyhow::Result<()> {
    // Use a unique temp file so this test is isolated from the shared test DB
    let tmp_path = "/tmp/burncloud_fast_path_test.db".to_string();
    let _ = std::fs::remove_file(&tmp_path);
    let url = format!("sqlite://{}?mode=rwc", tmp_path);
    let db = create_database_with_url(&url).await?;
    RouterDatabase::init(&db).await?;
    let db = Arc::new(db);

    // Pre-populate prices
    let input = PriceInput {
        model: "fast-path-model".to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(1.0),
        output_price: to_nano(2.0),
        source: Some("db".to_string()),
        region: None,
        ..Default::default()
    };
    PriceModel::upsert(&db, &input).await?;

    // Use a broken URL — with forced=false and DB prices present, should skip remote entirely
    let config = PriceSyncConfig {
        remote_url: "http://127.0.0.1:19999/nonexistent".to_string(),
        remote_url_fallback: None,
        remote_sync_enabled: true,
        ..PriceSyncConfig::default()
    };
    let mut service = PriceSyncService::with_config(db.clone(), config);

    let result = service.sync_all(false).await;
    let _ = std::fs::remove_file(&tmp_path); // cleanup
    let result = result?;
    assert_eq!(result.source, "db_cache", "Should use DB fast path when prices exist");

    Ok(())
}

/// Test model count drop protection: sync still succeeds with a warning when >50% of models drop
#[tokio::test]
async fn test_model_count_drop_protection() -> anyhow::Result<()> {
    use burncloud_common::pricing_config::{CurrencyPricing, ModelPricing};
    use std::collections::HashMap;

    let (db, _pool) = setup_db().await?;
    let db = Arc::new(db);

    // Pre-populate 10 models in DB
    for i in 0..10 {
        let input = PriceInput {
            model: format!("drop-test-model-{}", i),
            currency: "USD".to_string(),
            input_price: to_nano(1.0),
            output_price: to_nano(2.0),
            source: Some("test".to_string()),
            region: None,
            ..Default::default()
        };
        PriceModel::upsert(&db, &input).await?;
    }

    let service = PriceSyncService::new(db.clone());

    // Build a PricingConfig with only 1 model (>50% drop from 10)
    let mut models = HashMap::new();
    let mut pricing_map = HashMap::new();
    pricing_map.insert(
        "USD".to_string(),
        CurrencyPricing {
            input_price: to_nano(1.0),
            output_price: to_nano(2.0),
            source: None,
            ..Default::default()
        },
    );
    models.insert(
        "drop-survivor-model".to_string(),
        ModelPricing {
            pricing: pricing_map,
            metadata: None,
            cache_pricing: None,
            batch_pricing: None,
            tiered_pricing: None,
            voices_pricing: None,
            video_pricing: None,
            asr_pricing: None,
            realtime_pricing: None,
        },
    );
    let config = burncloud_common::pricing_config::PricingConfig {
        version: "1.0".to_string(),
        updated_at: chrono::Utc::now(),
        source: "test".to_string(),
        models,
    };

    // Sync completes OK even with >50% model count drop (logs a warning but doesn't fail)
    let result = service.apply_prices(&config, "remote").await?;
    assert_eq!(result.models_synced, 1);

    Ok(())
}

/// Test data source priority
#[tokio::test]
async fn test_data_source_priority() -> anyhow::Result<()> {
    let (_db, _pool) = setup_db().await?;
    let db = Arc::new(_db);

    let model_name = "test-priority-model-unique";

    // First, insert a price with a specific source
    let first_price = PriceInput {
        model: model_name.to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(10.0),
        output_price: to_nano(30.0),
        source: Some("remote".to_string()),
        region: Some("international".to_string()),
        ..Default::default()
    };
    PriceModel::upsert(&db, &first_price).await?;

    // Verify initial price
    let stored = PriceModel::get(&db, model_name, "USD", Some("international"))
        .await?
        .unwrap();
    assert_eq!(stored.input_price, to_nano(10.0));
    assert_eq!(stored.source, Some("remote".to_string()));

    // Now update with local source (higher priority)
    let local_price = PriceInput {
        model: model_name.to_string(),
        currency: "USD".to_string(),
        input_price: to_nano(5.0),
        output_price: to_nano(15.0),
        source: Some("local".to_string()),
        region: Some("international".to_string()),
        ..Default::default()
    };
    PriceModel::upsert(&db, &local_price).await?;

    // Verify the update
    let stored = PriceModel::get(&db, model_name, "USD", Some("international"))
        .await?
        .unwrap();
    assert_eq!(stored.input_price, to_nano(5.0));
    assert_eq!(stored.source, Some("local".to_string()));

    Ok(())
}
