use super::model::AdminSupplyModel;

#[test]
fn mock_values_match_supply_reference_page() {
    let model = AdminSupplyModel::mock();

    assert_eq!(model.total_gpus, "420");
    assert_eq!(model.providers, "24");
    assert_eq!(model.core_dc_gpus, "96");
    assert_eq!(model.standby_gpus, "44");
    assert_eq!(model.clusters.len(), 4);
    assert_eq!(model.clusters[0].name, "Silicon-Bay Core Pod 1-4");
    assert_eq!(model.clusters[1].region, "eu-central-fra");
    assert_eq!(model.clusters[2].attestation, "Confidential Nitro");
    assert_eq!(model.clusters[3].utilization, "32.0%");
}
