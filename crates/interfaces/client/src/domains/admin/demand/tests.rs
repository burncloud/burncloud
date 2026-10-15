use super::model::AdminDemandModel;
use crate::app::router::routes::Route;

#[test]
fn mock_values_match_demand_reference_page() {
    let model = AdminDemandModel::mock();

    assert_eq!(model.throughput, "42,800");
    assert_eq!(model.top_accounts, "1,420");
    assert_eq!(model.low_latency_share, "58.2%");
    assert_eq!(model.prompt_completion_tokens, "1,420");
    assert_eq!(model.accounts.len(), 3);
    assert_eq!(model.accounts[0].tenant, "HyperScale AI Labs (US-West)");
    assert_eq!(model.accounts[0].tokens, "148.2M");
    assert_eq!(model.accounts[1].tier, "Standard tier");
    assert_eq!(model.accounts[2].model, "Qwen 2.5 72B");
    assert_eq!(model.accounts[2].model_spend, "$12,850.00");
}

#[test]
fn admin_demand_route_parses_to_expected_variant() {
    assert!(matches!(
        "/admin/demand".parse::<Route>(),
        Ok(Route::AdminDemand {})
    ));
}
