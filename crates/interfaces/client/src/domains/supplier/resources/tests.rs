use super::{model::SupplierResourcesModel, state::SupplierResourcesState};
use crate::app::router::routes::Route;
use crate::domains::supplier::overview::model::NodeStatus;

#[test]
fn mock_resources_match_supplier_reference() {
    let model = SupplierResourcesModel::mock();

    assert_eq!(model.nodes.len(), 4);
    assert_eq!(model.online_count(), 3);
    assert_eq!(model.nodes[0].node.earnings, 184.20);
    assert_eq!(model.nodes[0].node.utilization, 91.4);
    assert_eq!(model.nodes[0].node.vram, "640 GB High-Bandwidth");
    assert_eq!(model.nodes[0].node.interconnect, "NVLink 900 GB/s");
    assert_eq!(model.nodes[0].uptime_hours, 742);
    assert_eq!(model.nodes[2].uptime_hours, 1240);
    assert_eq!(model.nodes[3].node.status, NodeStatus::Degraded);
    assert_eq!(model.nodes[3].uptime_hours, 88);
}

#[test]
fn drain_state_resets_after_cancel_or_confirm() {
    let mut state = SupplierResourcesState::default();

    assert!(state.selected_node().is_none());
    state.open_drain("SJC-Pod-01-Rack4");
    assert_eq!(
        state.selected_node().map(|resource| resource.node.name),
        Some("SJC-Pod-01-Rack4")
    );
    state.close_drain();
    assert!(state.selected_node().is_none());

    state.open_drain("SJC-Pod-01-Rack5");
    state.confirm_drain();
    assert!(state.selected_node().is_none());
}

#[test]
fn degraded_nodes_are_not_drainable_in_the_reference_model() {
    let state = SupplierResourcesState::default();
    let resource = state
        .model
        .nodes
        .iter()
        .find(|resource| resource.node.status == NodeStatus::Degraded)
        .expect("mock model contains a degraded node");

    assert_eq!(resource.node.name, "HKG-Edge-RTX-Pool");
}

#[test]
fn supplier_resources_route_parses_to_the_dedicated_component() {
    assert!(matches!(
        "/supplier/resources".parse::<Route>(),
        Ok(Route::SupplierResources {})
    ));
}
