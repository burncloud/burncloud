use super::model::{format_price, tier_label};
use crate::domains::buyer::marketplace::model::{ModelStatus, RoutingTier, MODEL_CATALOG};

#[test]
fn catalog_rows_match_the_admin_reference() {
    assert_eq!(MODEL_CATALOG.len(), 6);
    assert_eq!(MODEL_CATALOG[0].name, "DeepSeek V3 (671B MoE)");
    assert_eq!(MODEL_CATALOG[0].input_price_per_million, 0.14);
    assert_eq!(MODEL_CATALOG[0].output_price_per_million, 0.28);
    assert_eq!(MODEL_CATALOG[0].context_window, "128K tokens");
    assert_eq!(
        MODEL_CATALOG[0].supported_tiers,
        &[
            RoutingTier::Economy,
            RoutingTier::Standard,
            RoutingTier::Performance
        ]
    );
    assert_eq!(MODEL_CATALOG[0].p95_latency_ms, 380);
    assert_eq!(MODEL_CATALOG[0].availability, 99.99);
    assert_eq!(MODEL_CATALOG[0].status, ModelStatus::Healthy);
    assert_eq!(MODEL_CATALOG[5].name, "GLM-4 Plus");
    assert_eq!(MODEL_CATALOG[5].status, ModelStatus::Degraded);
}

#[test]
fn prices_and_tiers_use_the_reference_format() {
    assert_eq!(format_price(0.14), "0.14");
    assert_eq!(format_price(0.70), "0.7");
    assert_eq!(format_price(3.0), "3");
    assert_eq!(tier_label(RoutingTier::Economy), "ECONOMY");
    assert_eq!(tier_label(RoutingTier::Standard), "STANDARD");
    assert_eq!(tier_label(RoutingTier::Performance), "PERFORMANCE");
}
