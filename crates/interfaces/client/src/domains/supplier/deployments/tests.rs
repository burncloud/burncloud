use super::model::{DeploymentStatus, SupplierDeploymentsModel};
use crate::app::router::routes::Route;

#[test]
fn mock_deployments_match_supplier_reference() {
    let model = SupplierDeploymentsModel::mock();

    assert_eq!(model.deployments.len(), 4);
    assert_eq!(model.deployments[0].model, "DeepSeek V3 (671B MoE)");
    assert_eq!(model.deployments[0].throughput, "85.2 tokens/s");
    assert_eq!(model.deployments[0].score, "99.8 / 100");
    assert_eq!(model.deployments[2].node, "FRA-DC2-Compute-08 (8x A100)");
    assert_eq!(model.deployments[3].status, DeploymentStatus::Degraded);
}

#[test]
fn supplier_deployments_route_parses_to_the_dedicated_component() {
    assert!(matches!(
        "/supplier/deployments".parse::<Route>(),
        Ok(Route::SupplierDeployments {})
    ));
}
