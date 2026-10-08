use super::model::{AdminEventKind, AdminOverviewModel};
use crate::app::router::routes::Route;

#[test]
fn mock_values_match_admin_reference_page() {
    let model = AdminOverviewModel::mock();

    assert_eq!(model.gmv, "$18,450");
    assert_eq!(model.demand_share, "32.4%");
    assert_eq!(model.compute, "420");
    assert_eq!(model.sla, "99.99%");
    assert_eq!(model.events.len(), 3);
    assert_eq!(model.events[0].kind, AdminEventKind::Capacity);
    assert_eq!(model.events[1].kind, AdminEventKind::Failover);
    assert_eq!(model.events[2].kind, AdminEventKind::Economics);
    assert_eq!(model.supply_breakdown[0], "280 GPUs (66.7%)");
    assert_eq!(model.demand_breakdown[2], "1,420 Active Teams");
}

#[test]
fn admin_overview_routes_parse_to_expected_variants() {
    assert!(matches!("/admin".parse::<Route>(), Ok(Route::Admin {})));
    assert!(matches!(
        "/admin/overview".parse::<Route>(),
        Ok(Route::AdminOverview {})
    ));
}
