#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    reason = "Test-only file: the assertions on the byte-identical JSON fixtures are the test, and `serde_json::Value` is read as a document at this boundary."
)]
//! S1-B compatibility evidence: the legacy `burncloud_common` price/pricing paths and the
//! Commerce-owned contract path must resolve to the same types and produce byte-identical
//! JSON. Serialization shape is a hard compatibility promise for `pricing.json`, the
//! `billing_prices` rows and the CLI price import/export commands.

use serde_json::json;

/// Same value built through the legacy path.
fn legacy_price() -> burncloud_common::Price {
    burncloud_common::Price {
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
        embedding_price: None,
        image_price: Some(1_000_000),
        video_price: None,
        music_price: None,
        source: Some("litellm".to_string()),
        region: Some("international".to_string()),
        context_window: Some(128_000),
        max_output_tokens: Some(16_384),
        supports_vision: Some(1),
        supports_function_calling: Some(1),
        synced_at: Some(1_700_000_000),
        created_at: Some(1_600_000_000),
        updated_at: Some(1_700_000_000),
        voices_pricing: Some(r#"{"alloy":15000000000}"#.to_string()),
        video_pricing: None,
        asr_pricing: None,
        realtime_pricing: None,
        model_type: Some("chat".to_string()),
    }
}

#[test]
fn legacy_and_contract_price_are_the_same_type() {
    // Compiles only if both paths name one definition.
    let legacy: burncloud_common::Price = legacy_price();
    let via_contract: burncloud_commerce_contracts::pricing::Price = legacy.clone();
    let back: burncloud_common::types::BillingPrice = via_contract;
    assert_eq!(back.id, legacy.id);
}

#[test]
fn price_json_is_byte_identical_across_paths() {
    let legacy = legacy_price();
    let contract: burncloud_commerce_contracts::pricing::Price = legacy.clone();
    let legacy_json = serde_json::to_string(&legacy).unwrap();
    let contract_json = serde_json::to_string(&contract).unwrap();
    assert_eq!(legacy_json, contract_json);

    // Deserializing through either path yields the same value and the same JSON again.
    let from_legacy: burncloud_commerce_contracts::pricing::Price =
        serde_json::from_str(&legacy_json).unwrap();
    let from_contract: burncloud_common::Price = serde_json::from_str(&contract_json).unwrap();
    assert_eq!(
        serde_json::to_string(&from_legacy).unwrap(),
        serde_json::to_string(&from_contract).unwrap()
    );
}

#[test]
fn price_input_default_and_json_match() {
    let legacy = burncloud_common::PriceInput::default();
    let contract: burncloud_commerce_contracts::pricing::PriceInput = legacy.clone();
    assert_eq!(
        serde_json::to_string(&legacy).unwrap(),
        serde_json::to_string(&contract).unwrap()
    );
    assert_eq!(legacy.currency, "USD");
}

#[test]
fn currency_serde_is_still_lowercase() {
    // JSON encoding of Currency is a compatibility promise (kernel/domain contract).
    let legacy = burncloud_common::Currency::CNY;
    let contract: burncloud_commerce_contracts::pricing::Currency = legacy;
    assert_eq!(serde_json::to_string(&legacy).unwrap(), "\"cny\"");
    assert_eq!(
        serde_json::to_string(&contract).unwrap(),
        serde_json::to_string(&legacy).unwrap()
    );
    assert_eq!(
        burncloud_common::Currency::default(),
        burncloud_common::Currency::USD
    );
}

#[test]
fn pricing_config_roundtrip_is_unchanged() {
    // A v7-shaped document: legacy path and contract path must parse and re-serialize equal.
    let json = json!({
        "version": "7.0",
        "updated_at": "2026-01-01T00:00:00Z",
        "source": "compat-test",
        "models": {
            "gemini-pro": {
                "USD": {
                    "text": { "in": 1.25, "out": 5.0 },
                    "cache": { "read": 0.125, "write": 1.5625 },
                    "batch": { "in": 0.625, "out": 2.5 },
                    "tiered": [
                        { "tier_start": 0, "tier_end": 32000, "in": 1.2, "out": 4.8 }
                    ]
                }
            }
        }
    })
    .to_string();

    let legacy = burncloud_common::PricingConfig::from_json(&json).unwrap();
    let contract = burncloud_commerce_contracts::pricing::PricingConfig::from_json(&json).unwrap();

    let legacy_out = legacy.to_json().unwrap();
    let contract_out = contract.to_json().unwrap();
    assert_eq!(legacy_out, contract_out);

    // The nanodollar values parsed out of the document agree as well.
    let legacy_usd = legacy.get_pricing("gemini-pro", "USD").unwrap();
    let contract_usd = contract.get_pricing("gemini-pro", "USD").unwrap();
    assert_eq!(legacy_usd.input_price, contract_usd.input_price);
    assert_eq!(legacy_usd.input_price, 1_250_000_000);
    assert_eq!(legacy_usd.output_price, contract_usd.output_price);

    // Aliases and tier/cache lookup helpers still exist on both paths.
    let legacy_tier = legacy.get_tiered_pricing("gemini-pro", "USD").unwrap();
    let contract_tier = contract.get_tiered_pricing("gemini-pro", "USD").unwrap();
    assert_eq!(legacy_tier[0].tier_start, contract_tier[0].tier_start);
    assert!(legacy.get_cache_pricing("gemini-pro", "USD").is_some());
    assert!(contract.get_cache_pricing("gemini-pro", "USD").is_some());

    // The v7 conversion does not populate `metadata` (it does so for v1 documents only), so
    // this asserts the documented behavior is identical on both paths rather than claiming
    // metadata survives the v7 path.
    assert_eq!(
        legacy.models["gemini-pro"].metadata.is_none(),
        contract.models["gemini-pro"].metadata.is_none()
    );
}

#[test]
fn exchange_rate_and_tiered_price_aliases_still_resolve() {
    let rate = burncloud_common::ExchangeRate {
        id: 1,
        from_currency: "USD".to_string(),
        to_currency: "CNY".to_string(),
        rate: 7_240_000_000,
        updated_at: Some(1_700_000_000),
    };
    let alias: burncloud_common::types::BillingExchangeRate = rate.clone();
    let contract: burncloud_commerce_contracts::pricing::ExchangeRate = alias;
    assert_eq!(
        serde_json::to_string(&rate).unwrap(),
        serde_json::to_string(&contract).unwrap()
    );

    let tier = burncloud_common::TieredPrice {
        id: 3,
        model: "qwen-max".to_string(),
        region: Some("cn".to_string()),
        currency: Some("CNY".to_string()),
        tier_type: Some("context_length".to_string()),
        tier_start: 0,
        tier_end: Some(32_000),
        input_price: 1_200_000_000,
        output_price: 2_400_000_000,
    };
    let contract_tier: burncloud_commerce_contracts::pricing::BillingTieredPrice = tier.clone();
    assert_eq!(
        serde_json::to_string(&tier).unwrap(),
        serde_json::to_string(&contract_tier).unwrap()
    );
}
