use super::model::{AdminCapacityModel, CapacityStatus};

#[test]
fn mock_values_match_capacity_reference_page() {
    let model = AdminCapacityModel::mock();

    assert_eq!(model.safety_margin, "34.2%");
    assert_eq!(model.peak_throughput, "48,200");
    assert_eq!(model.replicas, "58");
    assert_eq!(model.latency_target, "< 200ms");
    assert_eq!(model.models.len(), 4);
    assert_eq!(model.models[0].active, "24,800 tok/s");
    assert_eq!(model.models[0].status, CapacityStatus::Healthy);
    assert_eq!(model.models[3].headroom, "15.0%");
    assert_eq!(model.models[3].status, CapacityStatus::Degraded);
}
