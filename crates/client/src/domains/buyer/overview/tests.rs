//! buyer/overview page boundary.
use super::{
    model::OverviewModel,
    state::{BalanceState, OverviewState},
};
use crate::app::router::routes::Route;

#[test]
fn mock_values_match_reference_page() {
    let model = OverviewModel::mock();
    assert_eq!(model.today_spend, 14.28);
    assert_eq!(model.prepaid_balance, Some(128.50));
    assert_eq!(model.api_availability, Some(99.99));
    assert_eq!(model.tokens_today, Some(1_840_000));
    assert_eq!(
        model.models.iter().map(|m| m.name).collect::<Vec<_>>(),
        vec![
            "DeepSeek V3 (671B MoE)",
            "DeepSeek R1 Reasoning",
            "Qwen 2.5 72B Instruct"
        ]
    );
    assert_eq!(
        model.models.iter().map(|m| m.tokens).collect::<Vec<_>>(),
        vec![1_120_400, 410_200, 310_000]
    );
    assert_eq!(model.recent_activity.len(), 3);
    assert_eq!(model.recent_activity[0].time, "12 mins ago");
}

#[test]
fn balance_states_are_explicit() {
    assert_eq!(
        BalanceState::from_balance(Some(128.5)),
        BalanceState::Healthy
    );
    assert_eq!(BalanceState::from_balance(Some(19.99)), BalanceState::Low);
    assert_eq!(
        BalanceState::from_balance(Some(0.0)),
        BalanceState::Critical
    );
    assert_eq!(BalanceState::from_balance(None), BalanceState::Unknown);
}

#[test]
fn attention_only_for_low_or_critical_balances() {
    assert!(!OverviewState::default().balance_state.needs_attention());
    assert!(BalanceState::Low.needs_attention());
    assert!(BalanceState::Critical.needs_attention());
    assert!(!BalanceState::Unknown.needs_attention());
}

#[test]
fn overview_aliases_parse_to_buyer_routes() {
    assert!(matches!("/".parse::<Route>(), Ok(Route::Home {})));
    assert!(matches!("/buyer".parse::<Route>(), Ok(Route::Buyer {})));
    assert!(matches!(
        "/buyer/overview".parse::<Route>(),
        Ok(Route::BuyerOverviewRoute {})
    ));
    assert!(matches!("/console".parse::<Route>(), Ok(Route::Console {})));
    assert!(matches!(
        "/console/dashboard".parse::<Route>(),
        Ok(Route::ConsoleDashboard {})
    ));
    assert!(matches!(
        "/console/buyer/overview".parse::<Route>(),
        Ok(Route::ConsoleBuyerOverview {})
    ));
}
