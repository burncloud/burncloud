use super::{
    model::{NodeStatus, SupplierOverviewModel},
    state::SupplierOverviewState,
};
use crate::app::router::routes::Route;

#[test]
fn mock_values_match_supplier_reference() {
    let model = SupplierOverviewModel::mock();

    assert_eq!(model.today_earnings, 382.40);
    assert_eq!(model.active_gpu_count, 24);
    assert_eq!(model.total_gpu_count, 28);
    assert_eq!(model.cluster_count, 3);
    assert_eq!(model.gpu_utilization, 74.0);
    assert_eq!(model.peak_compute, 88.6);
    assert_eq!(model.tokens_today, 24.2);
    assert_eq!(model.nodes.len(), 4);
    assert_eq!(model.nodes[0].hardware, "8 x NVIDIA H100 SXM5 80GB");
    assert_eq!(model.nodes[3].status, NodeStatus::Degraded);
    assert_eq!(model.nodes[3].temperature_c, 72);
}

#[test]
fn install_modal_state_resets_copy_feedback() {
    let mut state = SupplierOverviewState::default();

    assert!(!state.install_modal_open);
    state.open_install_modal();
    assert!(state.install_modal_open);
    state.mark_command_copied();
    assert!(state.command_copied);
    state.close_install_modal();
    assert!(!state.install_modal_open);
    assert!(!state.command_copied);
}

#[test]
fn supplier_routes_parse_to_expected_variants() {
    assert!(matches!(
        "/supplier".parse::<Route>(),
        Ok(Route::Supplier {})
    ));
    assert!(matches!(
        "/supplier/overview".parse::<Route>(),
        Ok(Route::SupplierOverview {})
    ));
}
