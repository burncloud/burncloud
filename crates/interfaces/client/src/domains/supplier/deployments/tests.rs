use super::model::{DeploymentStatus, SupplierDeploymentsModel};
use crate::app::router::routes::Route;

#[test]
fn reference_deployments_match_supplier_page() {
    let model = SupplierDeploymentsModel::reference();

    assert_eq!(model.deployments.len(), 4);
    assert_eq!(model.deployments[0].model, "DeepSeek V3 (671B MoE)");
    assert_eq!(model.deployments[0].parallelism, "TP=8 / FP8 FlashInfer");
    assert_eq!(model.deployments[0].contribution, "99.8 / 100");
    assert_eq!(model.deployments[3].status, DeploymentStatus::Degraded);
    assert_eq!(model.deployments[3].throughput, "42.1 tokens/s");
}

#[test]
fn supplier_deployments_route_parses_to_the_dedicated_page() {
    assert!(matches!(
        "/supplier/deployments".parse::<Route>(),
        Ok(Route::SupplierDeployments {})
    ));
}
