use super::{
    model::{ModelCategory, ModelStatus, RoutingTier, MODEL_CATALOG},
    state::MarketplaceState,
};
use crate::app::router::routes::Route;

#[test]
fn reference_catalog_contains_six_expected_models() {
    assert_eq!(MODEL_CATALOG.len(), 6);
    assert_eq!(
        MODEL_CATALOG
            .iter()
            .map(|model| model.id)
            .collect::<Vec<_>>(),
        vec![
            "deepseek-v3",
            "deepseek-r1",
            "qwen-2.5-72b-instruct",
            "claude-3-5-sonnet",
            "llama-3.3-70b-instruct",
            "glm-4-plus",
        ]
    );
    assert_eq!(MODEL_CATALOG[0].input_price_per_million, 0.14);
    assert_eq!(MODEL_CATALOG[1].p95_latency_ms, 620);
    assert_eq!(MODEL_CATALOG[5].status, ModelStatus::Degraded);
}

#[test]
fn catalog_preserves_reference_categories_and_tiers() {
    assert_eq!(MODEL_CATALOG[0].category, ModelCategory::General);
    assert_eq!(MODEL_CATALOG[1].category, ModelCategory::Reasoning);
    assert_eq!(MODEL_CATALOG[2].category, ModelCategory::Coding);
    assert_eq!(MODEL_CATALOG[4].category, ModelCategory::LowLatency);
    assert_eq!(
        MODEL_CATALOG[0].supported_tiers,
        &[
            RoutingTier::Economy,
            RoutingTier::Standard,
            RoutingTier::Performance
        ]
    );
    assert_eq!(
        MODEL_CATALOG[4].supported_tiers,
        &[RoutingTier::Economy, RoutingTier::Standard]
    );
}

#[test]
fn search_matches_name_family_and_tagline_case_insensitively() {
    let mut state = MarketplaceState::default();

    state.set_search("QWEN");
    assert_eq!(state.filtered_models().len(), 1);
    assert_eq!(state.filtered_models()[0].id, "qwen-2.5-72b-instruct");

    state.set_search("anthropic");
    assert_eq!(state.filtered_models()[0].id, "claude-3-5-sonnet");

    state.set_search("chain-of-thought");
    assert_eq!(state.filtered_models()[0].id, "deepseek-r1");
}

#[test]
fn category_and_search_filters_are_combined() {
    let mut state = MarketplaceState::default();
    state.set_category(ModelCategory::Coding);
    assert_eq!(state.filtered_models().len(), 2);

    state.set_search("claude");
    assert_eq!(state.filtered_models().len(), 1);
    assert_eq!(state.filtered_models()[0].id, "claude-3-5-sonnet");

    state.set_search("missing");
    assert!(state.filtered_models().is_empty());
}

#[test]
fn drawer_state_resets_slo_expansion_when_switching_models_or_closing() {
    let mut state = MarketplaceState::default();
    state.open_details("deepseek-v3");
    assert!(state.drawer_open);
    assert_eq!(
        state.selected_model().expect("selected model").id,
        "deepseek-v3"
    );

    state.toggle_slo();
    assert!(state.slo_expanded);
    state.open_details("glm-4-plus");
    assert!(!state.slo_expanded);
    assert_eq!(
        state.selected_model().expect("selected model").id,
        "glm-4-plus"
    );

    state.toggle_slo();
    state.close_drawer();
    assert!(!state.drawer_open);
    assert!(!state.slo_expanded);
}

#[test]
fn marketplace_route_parses_to_its_dedicated_component() {
    assert!(matches!(
        "/buyer/marketplace".parse::<Route>(),
        Ok(Route::Marketplace {})
    ));
}
