#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]
//! Golden-fixture acceptance for the Commerce pricing contract.
//!
//! These fixtures are the **independent** record of the pricing contract: they assert the
//! business values parsed out of a fixed document and the exact JSON produced when serializing it
//! back. They deliberately go through `burncloud_commerce_contracts` only -- no `burncloud_common`
//! re-export -- so the acceptance criteria survive the eventual removal of the compatibility layer
//! (review feedback on #602, item 5).
//!
//! The fixtures double as documentation of two behaviours that used to be log lines and are now
//! documented contract behaviour:
//!   * a v7 currency block without a `text` section is skipped;
//!   * a document without a `version` field is parsed as v1.
//!
//! Since #606 `from_json` dispatches on the document **layout** first, falling back to the `version`
//! string only when the layout carries no marker, so `to_json` output (always the normalised v1
//! layout) can be read back whatever version it carries.

use std::fs;
use std::path::PathBuf;

use burncloud_commerce_contracts::pricing::PricingConfig;

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/pricing")
        .join(name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn load(name: &str) -> PricingConfig {
    PricingConfig::from_json(&fixture(name)).unwrap_or_else(|e| panic!("parse {name}: {e}"))
}

/// Assert the documented round-trip contract, independent of the compatibility layer.
///
/// A v1-shaped document is a stable golden fixture: it is the layout the serializer itself
/// produces, so parsing it, serializing it, parsing that output and serializing again yields the
/// same JSON value. Parsed values are compared rather than raw strings because `PricingConfig`
/// keeps its maps in `HashMap` and the serialized key order is therefore not stable.
///
/// A v7 document is normalised into the v1 layout on parse, so its output differs from its
/// input; only the normalisation is asserted for it.
fn assert_round_trip_contract(name: &str, source_is_v1: bool) {
    let source = fixture(name);
    let once = load(name).to_json().unwrap();
    let value_once: serde_json::Value = serde_json::from_str(&once).unwrap();

    for (model, entry) in value_once["models"].as_object().unwrap() {
        assert!(
            entry.get("pricing").is_some(),
            "{name}: model {model} has no normalised pricing block"
        );
    }

    if source_is_v1 {
        let twice = PricingConfig::from_json(&once)
            .unwrap_or_else(|e| panic!("{name}: normalised output does not re-parse: {e}"))
            .to_json()
            .unwrap();
        let value_twice: serde_json::Value = serde_json::from_str(&twice).unwrap();
        assert_eq!(
            value_once, value_twice,
            "{name}: parse/serialize is not idempotent for a v1 document"
        );

        let source_value: serde_json::Value = serde_json::from_str(&source).unwrap();
        assert_eq!(
            source_value, value_once,
            "{name}: a v1 document must reproduce its own layout"
        );
    } else {
        assert_ne!(
            source.trim(),
            once.trim(),
            "{name}: expected a format normalisation on parse"
        );
    }
}

#[test]
fn v1_fixture_parses_to_expected_nanodollars() {
    let config = load("v1.json");
    assert_eq!(config.version, "1.0");
    assert_eq!(config.source, "fixture-v1");

    let usd = config.get_pricing("gpt-4o", "USD").unwrap();
    // dollars * 10^9
    assert_eq!(usd.input_price, 2_500_000_000);
    assert_eq!(usd.output_price, 10_000_000_000);

    let cache = config.get_cache_pricing("gpt-4o", "USD").unwrap();
    assert_eq!(cache.cache_read_input_price, 250_000_000);
    assert_eq!(cache.cache_creation_input_price, Some(3_125_000_000));

    let batch = config.models["gpt-4o"]
        .batch_pricing
        .as_ref()
        .and_then(|m| m.get("USD"))
        .unwrap();
    assert_eq!(batch.batch_input_price, 1_250_000_000);
    assert_eq!(batch.batch_output_price, 5_000_000_000);

    // v1 documents keep their metadata block.
    let metadata = config.models["gpt-4o"].metadata.as_ref().unwrap();
    assert_eq!(metadata.context_window, Some(128_000));
    assert_eq!(metadata.max_output_tokens, Some(16_384));
    assert!(metadata.supports_vision);
    assert!(metadata.supports_function_calling);
    assert!(metadata.supports_streaming);
    assert_eq!(metadata.provider.as_deref(), Some("openai"));
    assert_eq!(metadata.family.as_deref(), Some("gpt-4"));
    assert_eq!(metadata.release_date.as_deref(), Some("2024-05-13"));

    assert_round_trip_contract("v1.json", true);
}

#[test]
fn v7_fixture_parses_every_price_dimension() {
    let config = load("v7.json");
    assert_eq!(config.version, "7.0");

    let usd = config.get_pricing("gemini-flash", "USD").unwrap();
    assert_eq!(usd.input_price, 75_000_000);
    assert_eq!(usd.output_price, 300_000_000);
    // image / audio / music are output-side extras produced by the v7 conversion
    assert_eq!(usd.image_output_price, Some(2_000_000));
    assert_eq!(usd.audio_output_price, Some(600_000_000));
    assert_eq!(usd.music_price, Some(50_000_000));

    let cache = config.get_cache_pricing("gemini-flash", "USD").unwrap();
    assert_eq!(cache.cache_read_input_price, 7_500_000);
    assert_eq!(cache.cache_creation_input_price, Some(93_750_000));

    // The v7 conversion does not populate metadata; that is existing, asserted behaviour.
    assert!(config.models["gemini-flash"].metadata.is_none());

    assert_round_trip_contract("v7.json", false);
}

#[test]
fn tiered_fixture_keeps_tier_order_and_bounds() {
    let config = load("tiered.json");
    let tiers = config.get_tiered_pricing("qwen-max", "USD").unwrap();
    assert_eq!(tiers.len(), 3);
    assert_eq!((tiers[0].tier_start, tiers[0].tier_end), (0, Some(32_000)));
    assert_eq!(
        (tiers[1].tier_start, tiers[1].tier_end),
        (32_000, Some(128_000))
    );
    assert_eq!((tiers[2].tier_start, tiers[2].tier_end), (128_000, None));
    assert_eq!(tiers[2].input_price, 3_000_000_000);
    assert_eq!(tiers[2].output_price, 6_000_000_000);

    assert_round_trip_contract("tiered.json", false);
}

#[test]
fn cache_fixture_distinguishes_read_and_creation_prices() {
    let config = load("cache.json");
    let cache = config
        .get_cache_pricing("claude-3-5-sonnet", "USD")
        .unwrap();
    // read ~10% of input, creation ~125% of input: two different numbers, kept apart
    assert_eq!(cache.cache_read_input_price, 300_000_000);
    assert_eq!(cache.cache_creation_input_price, Some(3_750_000_000));
    assert_ne!(
        cache.cache_read_input_price,
        cache.cache_creation_input_price.unwrap()
    );

    assert_round_trip_contract("cache.json", false);
}

#[test]
fn multicurrency_fixture_keeps_currencies_independent() {
    let config = load("multicurrency.json");
    let usd = config.get_pricing("multi-currency-model", "USD").unwrap();
    let cny = config.get_pricing("multi-currency-model", "CNY").unwrap();
    let eur = config.get_pricing("multi-currency-model", "EUR").unwrap();

    assert_eq!(usd.input_price, 1_000_000_000);
    assert_eq!(cny.input_price, 7_240_000_000);
    assert_eq!(eur.input_price, 920_000_000);
    // No implicit conversion happens at parse time.
    assert_ne!(usd.input_price, cny.input_price);

    assert_eq!(config.list_models().len(), 1);
    assert!(config.get_pricing("multi-currency-model", "GBP").is_none());

    assert_round_trip_contract("multicurrency.json", false);
}

#[test]
fn v7_block_without_text_is_skipped() {
    // Documented behaviour: the currency block has no text section, so no pricing entry is
    // created for it and the model still exists in the map.
    let json = r#"{"version":"7.0","updated_at":"2026-03-29T00:00:00Z","source":"t",
                   "models":{"image-only":{"USD":{"image":{"out":0.04}}}}}"#;
    let config = PricingConfig::from_json(json).unwrap();
    assert!(config.get_pricing("image-only", "USD").is_none());
    assert!(config.models.contains_key("image-only"));
}

#[test]
fn missing_version_is_parsed_as_v1() {
    // Documented fallback: absence of `version` means v1, so the nested v1 shape must parse.
    let json = r#"{"updated_at":"2026-03-29T00:00:00Z","source":"t",
                   "models":{"legacy":{"pricing":{"USD":{"input_price":1.0,"output_price":2.0}}}}}"#;
    let config = PricingConfig::from_json(json).unwrap();
    assert_eq!(config.version, "1.0");
    assert_eq!(
        config.get_pricing("legacy", "USD").unwrap().input_price,
        1_000_000_000
    );
}

#[test]
fn v7_output_keeps_its_price_blocks_when_read_back() {
    // #606: `from_json` dispatches on the document **layout** first and only falls back to the `version`
    // string when the layout carries no marker. `to_json()` writes the normalised v1 layout whatever the
    // version says, so re-parsing its own output of a v7-sourced document finds the v1 body and reads it
    // correctly instead of routing it to the v7 parser, which found no `text` block and dropped every price.
    //
    // Both halves matter, so both are asserted: the prices survive, and the version string is still carried
    // through rather than being rewritten to "1.0".
    let original = load("v7.json");
    assert!(
        original.get_pricing("gemini-flash", "USD").is_some(),
        "the original v7 document does carry the price block"
    );

    let once = original.to_json().unwrap();
    let reparsed = PricingConfig::from_json(&once)
        .unwrap_or_else(|e| panic!("to_json output must re-parse: {e}"));
    assert_eq!(reparsed.version, "7.0", "the version string is preserved");

    let before = original.get_pricing("gemini-flash", "USD").unwrap();
    let after = reparsed
        .get_pricing("gemini-flash", "USD")
        .expect("the v7 price block must survive a serialize/re-parse cycle");
    assert_eq!(
        after.input_price, before.input_price,
        "input price is intact"
    );
    assert_eq!(
        after.output_price, before.output_price,
        "output price is intact"
    );
    assert_eq!(
        reparsed
            .get_cache_pricing("gemini-flash", "USD")
            .expect("cache block survives")
            .cache_read_input_price,
        original
            .get_cache_pricing("gemini-flash", "USD")
            .unwrap()
            .cache_read_input_price,
        "and so is the cache block"
    );

    // A second cycle changes nothing: the round-trip is a fixed point, not merely lossless once.
    let twice = reparsed.to_json().unwrap();
    let value_once: serde_json::Value = serde_json::from_str(&once).unwrap();
    let value_twice: serde_json::Value = serde_json::from_str(&twice).unwrap();
    assert_eq!(
        value_once, value_twice,
        "re-serialising the re-parsed document must reproduce the same JSON"
    );
}
