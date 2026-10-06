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
