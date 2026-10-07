#![allow(
    clippy::unwrap_used,
    reason = "Test-only file: the assertions are the test."
)]

use burncloud_commerce_contracts::pricing::PricingConfig;

#[test]
fn tiered_pricing_without_standard_pricing_is_still_normalised_v1() {
    let json = r#"{
        "version":"7.0",
        "updated_at":"2026-03-29T00:00:00Z",
        "source":"test",
        "models":{
            "tiered-only":{
                "tiered_pricing":{
                    "USD":[{
                        "tier_start":0,
                        "tier_end":1000,
                        "input_price":3.0,
                        "output_price":4.0
                    }]
                }
            }
        }
    }"#;

    let config = PricingConfig::from_json(json)
        .unwrap_or_else(|e| panic!("tiered-only normalized-v1 document must parse: {e}"));
    let tier = &config
        .get_tiered_pricing("tiered-only", "USD")
        .expect("tiered price must survive layout dispatch")[0];

    assert_eq!(tier.input_price, 3_000_000_000);
    assert_eq!(tier.output_price, 4_000_000_000);
    assert_eq!(config.version, "7.0", "version is metadata, not the layout");
}

#[test]
fn video_pricing_without_standard_pricing_is_still_normalised_v1() {
    let json = r#"{
        "version":"7.0",
        "updated_at":"2026-03-29T00:00:00Z",
        "source":"test",
        "models":{
            "video-only":{
                "video_pricing":{
                    "USD":{"1080p":0.10}
                }
            }
        }
    }"#;

    let config = PricingConfig::from_json(json)
        .unwrap_or_else(|e| panic!("video-only normalized-v1 document must parse: {e}"));
    let model = config.models.get("video-only").expect("model survives");
    let video = model
        .video_pricing
        .as_ref()
        .and_then(|currencies| currencies.get("USD"))
        .expect("video pricing must survive layout dispatch");

    assert_eq!(video.resolutions.get("1080p"), Some(&100_000_000));
}

#[test]
fn metadata_without_any_price_block_is_still_normalised_v1() {
    let json = r#"{
        "version":"7.0",
        "updated_at":"2026-03-29T00:00:00Z",
        "source":"test",
        "models":{
            "metadata-only":{
                "metadata":{
                    "context_window":128000,
                    "supports_vision":true,
                    "supports_function_calling":false
                }
            }
        }
    }"#;

    let config = PricingConfig::from_json(json)
        .unwrap_or_else(|e| panic!("metadata-only normalized-v1 document must parse: {e}"));
    let model = config.models.get("metadata-only").expect("model survives");
    let metadata = model
        .metadata
        .as_ref()
        .expect("metadata must survive layout dispatch");

    assert_eq!(metadata.context_window, Some(128_000));
    assert!(metadata.supports_vision);
    assert!(!metadata.supports_function_calling);
    assert!(metadata.supports_streaming, "default_true must still apply");
}

#[test]
fn flat_v7_with_model_level_metadata_stays_flat_and_keeps_prices() {
    let json = r#"{
        "version":"8.0",
        "updated_at":"2026-03-29T00:00:00Z",
        "source":"future-flat",
        "models":{
            "future-model":{
                "metadata":{
                    "context_window":256000,
                    "supports_vision":true
                },
                "USD":{
                    "text":{"in":1.25,"out":5.0},
                    "cache":{"read":0.125}
                }
            }
        }
    }"#;

    let config = PricingConfig::from_json(json)
        .unwrap_or_else(|e| panic!("flat document with model-level metadata must still parse: {e}"));

    let price = config
        .get_pricing("future-model", "USD")
        .expect("flat USD text pricing must survive metadata extension");
    assert_eq!(price.input_price, 1_250_000_000);
    assert_eq!(price.output_price, 5_000_000_000);
    assert!(
        config.models["future-model"].metadata.is_none(),
        "the flat parser historically ignores model-level metadata; #606 must preserve that tolerance rather than reject the whole document"
    );
    assert_eq!(config.version, "8.0");
}

#[test]
fn mixed_v1_and_v7_models_are_rejected_instead_of_partly_dropped() {
    let json = r#"{
        "version":"7.0",
        "updated_at":"2026-03-29T00:00:00Z",
        "source":"test",
        "models":{
            "normalised":{
                "pricing":{"USD":{"input_price":3.0,"output_price":4.0}}
            },
            "flat":{
                "USD":{"text":{"in":5.0,"out":6.0}}
            }
        }
    }"#;

    let error = PricingConfig::from_json(json)
        .expect_err("a mixed-layout catalogue cannot be parsed without dropping one side");
    assert!(
        error
            .to_string()
            .contains("mixes normalised-v1 and flat-v7"),
        "the rejection must identify the layout conflict: {error}"
    );
}
