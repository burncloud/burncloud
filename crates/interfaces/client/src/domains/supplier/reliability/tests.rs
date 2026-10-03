use super::model::{
    ReliabilityAuditKind, ReliabilityTierKind, ReliabilityTierStatus, SupplierReliabilityModel,
};
use crate::app::router::routes::Route;

#[test]
fn reference_reliability_matches_supplier_page() {
    let model = SupplierReliabilityModel::reference();

    assert_eq!(model.score, "98.6");
    assert_eq!(model.availability, "99.94%");
    assert_eq!(model.current_tier, "L3 Pro");
    assert_eq!(model.revenue_share, "80.0%");
    assert_eq!(model.tiers.len(), 4);
    assert_eq!(model.audits.len(), 3);
    assert_eq!(model.tiers[2].kind, ReliabilityTierKind::Professional);
    assert_eq!(model.tiers[2].status, ReliabilityTierStatus::Current);
    assert_eq!(model.tiers[3].status, ReliabilityTierStatus::Eligible);
    assert_eq!(model.audits[0].kind, ReliabilityAuditKind::Thermal);
    assert_eq!(model.audits[2].node, "8x H100 SXM5");
}

#[test]
fn supplier_reliability_route_parses_to_the_dedicated_page() {
    assert!(matches!(
        "/supplier/reliability".parse::<Route>(),
        Ok(Route::SupplierReliability {})
    ));
}
