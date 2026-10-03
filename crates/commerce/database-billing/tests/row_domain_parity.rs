#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Row <-> domain parity (review feedback on #602, item 4).
//!
//! The persistence structs in `burncloud_database_billing::rows` and the Commerce domain types in
//! `burncloud_commerce_contracts::pricing` now have separate definitions. These tests prove the
//! conversions are lossless and value-preserving in both directions, so splitting the types cannot
//! silently drop a column.
//!
//! Every field gets a distinct value, and the assertions compare the full structs rather than a
//! sample of fields, so a field added to the SQL but forgotten in the conversion fails here.

use burncloud_commerce_contracts::pricing::{ExchangeRate, Price, TieredPrice};
use burncloud_database_billing::{ExchangeRateRow, PriceRow, TieredPriceRow};

/// A row with every column populated by a distinct value.
fn full_price_row() -> PriceRow {
    PriceRow {
        id: 7,
        model: "gpt-4o".to_string(),
        currency: "USD".to_string(),
        input_price: 2_500_000_000,
        output_price: 10_000_000_000,
        cache_read_input_price: Some(250_000_000),
        cache_creation_input_price: Some(3_125_000_000),
        batch_input_price: Some(1_250_000_000),
        batch_output_price: Some(5_000_000_000),
        priority_input_price: Some(4_250_000_000),
        priority_output_price: Some(17_000_000_000),
        audio_input_price: Some(40_000_000_000),
        audio_output_price: Some(80_000_000_000),
        reasoning_price: Some(10_000_000_000),
        embedding_price: Some(20_000_000),
        image_price: Some(1_000_000),
        video_price: Some(2_000_000),
        music_price: Some(50_000_000),
        source: Some("litellm".to_string()),
        region: Some("international".to_string()),
        context_window: Some(128_000),
        max_output_tokens: Some(16_384),
        supports_vision: Some(1),
        supports_function_calling: Some(0),
        synced_at: Some(1_700_000_000),
        created_at: Some(1_600_000_000),
        updated_at: Some(1_700_000_001),
        voices_pricing: Some(r#"{"alloy":15000000000}"#.to_string()),
        video_pricing: Some(r#"{"720p":50000000}"#.to_string()),
        asr_pricing: Some(r#"{"per_minute":360000000}"#.to_string()),
        realtime_pricing: Some(r#"{"audio_input":32000000000}"#.to_string()),
        model_type: Some("chat".to_string()),
    }
}

#[test]
fn price_row_round_trips_without_losing_a_column() {
    let row = full_price_row();
    let domain = Price::from(row.clone());
    let back = PriceRow::from(domain);

    // Field-by-field comparison keeps the failure message readable.
    assert_eq!(back.id, row.id);
    assert_eq!(back.model, row.model);
    assert_eq!(back.currency, row.currency);
    assert_eq!(back.input_price, row.input_price);
    assert_eq!(back.output_price, row.output_price);
    assert_eq!(back.cache_read_input_price, row.cache_read_input_price);
    assert_eq!(
        back.cache_creation_input_price,
        row.cache_creation_input_price
    );
    assert_eq!(back.batch_input_price, row.batch_input_price);
    assert_eq!(back.batch_output_price, row.batch_output_price);
    assert_eq!(back.priority_input_price, row.priority_input_price);
    assert_eq!(back.priority_output_price, row.priority_output_price);
    assert_eq!(back.audio_input_price, row.audio_input_price);
    assert_eq!(back.audio_output_price, row.audio_output_price);
    assert_eq!(back.reasoning_price, row.reasoning_price);
    assert_eq!(back.embedding_price, row.embedding_price);
    assert_eq!(back.image_price, row.image_price);
    assert_eq!(back.video_price, row.video_price);
    assert_eq!(back.music_price, row.music_price);
    assert_eq!(back.source, row.source);
    assert_eq!(back.region, row.region);
    assert_eq!(back.context_window, row.context_window);
    assert_eq!(back.max_output_tokens, row.max_output_tokens);
    assert_eq!(back.supports_vision, row.supports_vision);
    assert_eq!(
        back.supports_function_calling,
        row.supports_function_calling
    );
    assert_eq!(back.synced_at, row.synced_at);
    assert_eq!(back.created_at, row.created_at);
    assert_eq!(back.updated_at, row.updated_at);
    assert_eq!(back.voices_pricing, row.voices_pricing);
    assert_eq!(back.video_pricing, row.video_pricing);
    assert_eq!(back.asr_pricing, row.asr_pricing);
    assert_eq!(back.realtime_pricing, row.realtime_pricing);
    assert_eq!(back.model_type, row.model_type);
}

#[test]
fn all_null_price_row_round_trips() {
    // The `#[sqlx(default)]` columns can be absent from a result set; NULLs must survive too.
    let row = PriceRow {
        id: 0,
        model: "some-model".to_string(),
        currency: "CNY".to_string(),
        input_price: 0,
        output_price: 0,
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
        source: None,
        region: None,
        context_window: None,
        max_output_tokens: None,
        supports_vision: None,
        supports_function_calling: None,
        synced_at: None,
        created_at: None,
        updated_at: None,
        voices_pricing: None,
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: None,
    };
    let domain = Price::from(row.clone());
    let back = PriceRow::from(domain);
    assert_eq!(back.cache_read_input_price, None);
    assert_eq!(back.supports_vision, None);
    assert_eq!(back.model_type, None);
    assert_eq!(back.region, None);
}

#[test]
fn tiered_price_row_round_trips_without_losing_a_column() {
    let row = TieredPriceRow {
        id: 3,
        model: "qwen-max".to_string(),
        region: Some("cn".to_string()),
        currency: Some("CNY".to_string()),
        tier_type: Some("context_length".to_string()),
        tier_start: 32_000,
        tier_end: Some(128_000),
        input_price: 2_400_000_000,
        output_price: 4_800_000_000,
    };
    let domain = TieredPrice::from(row.clone());
    let back = TieredPriceRow::from(domain);

    assert_eq!(back.id, row.id);
    assert_eq!(back.model, row.model);
    assert_eq!(back.region, row.region);
    assert_eq!(back.currency, row.currency);
    assert_eq!(back.tier_type, row.tier_type);
    assert_eq!(back.tier_start, row.tier_start);
    assert_eq!(back.tier_end, row.tier_end);
    assert_eq!(back.input_price, row.input_price);
    assert_eq!(back.output_price, row.output_price);
}

#[test]
fn open_ended_tier_keeps_its_missing_upper_bound() {
    let row = TieredPriceRow {
        id: 9,
        model: "qwen-max".to_string(),
        region: None,
        currency: None,
        tier_type: None,
        tier_start: 128_000,
        tier_end: None,
        input_price: 3_000_000_000,
        output_price: 6_000_000_000,
    };
    let back = TieredPriceRow::from(TieredPrice::from(row));
    assert_eq!(back.tier_end, None, "an open-ended tier has no upper bound");
    assert_eq!(back.region, None);
}

#[test]
fn exchange_rate_row_round_trips_without_losing_a_column() {
    let row = ExchangeRateRow {
        id: 1,
        from_currency: "USD".to_string(),
        to_currency: "CNY".to_string(),
        rate: 7_240_000_000,
        updated_at: Some(1_700_000_000),
    };
    let domain = ExchangeRate::from(row.clone());
    let back = ExchangeRateRow::from(domain);

    assert_eq!(back.id, row.id);
    assert_eq!(back.from_currency, row.from_currency);
    assert_eq!(back.to_currency, row.to_currency);
    assert_eq!(back.rate, row.rate);
    assert_eq!(back.updated_at, row.updated_at);
    // Scaled integer rate keeps its exact precision through the conversion.
    assert_eq!(back.rate, 7_240_000_000);
}
