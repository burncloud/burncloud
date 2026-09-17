use burncloud_node_runtime::{
    FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
    FakeReadinessProbe, FakeRuntimePreparer, NodeComposition, NodeState, ReconcileEvidence,
};
use burncloud_server::node_attachment::{
    attach_ready_node_route, begin_detached_route_recovery, detach_unhealthy_node_route,
};
use burncloud_server::node_orchestrator::{ModelDemand, NodeOrchestrator};
use burncloud_service_models::FakeModelResolver;

fn fake_orchestrator() -> NodeOrchestrator<
    FakeModelResolver,
    FakeHardwareProbe,
    FakeArtifactPreparer,
    FakeRuntimePreparer,
    FakeProcessManager,
    FakeReadinessProbe,
    FakeHealthProbe,
> {
    NodeOrchestrator::new(
        FakeModelResolver,
        NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            FakeProcessManager::default(),
            FakeReadinessProbe,
            FakeHealthProbe,
        ),
    )
}

/// S0-14 acceptance: the test supplies only desired demand and fake boundary
/// side effects. It never manually emits success evidence for Resolve,
/// Artifact, Runtime, Process, Readiness, Attach, Unhealthy, or Recovery.
#[tokio::test]
async fn model_demand_runs_full_fake_route_failure_recovery_and_reattach_rail() {
    let demand = ModelDemand::new("qwen-4b").unwrap();
    let mut node = fake_orchestrator();

    node.prepare_until_ready(demand.clone()).await.unwrap();
    assert_eq!(node.machine().reconciler().state(), NodeState::Ready);

    let first_route = attach_ready_node_route(
        node.machine_mut().reconciler_mut(),
        || async { Ok(101_i32) },
    )
    .await
    .unwrap();
    assert_eq!(first_route, 101);
    assert_eq!(node.machine().reconciler().state(), NodeState::Routable);

    let detached = detach_unhealthy_node_route(
        node.machine_mut().reconciler_mut(),
        first_route,
        |id| async move {
            assert_eq!(id, 101);
            Ok(())
        },
    )
    .await
    .unwrap();
    assert_eq!(node.machine().reconciler().state(), NodeState::Unhealthy);
    assert!(!node.machine().reconciler().state().is_serving());

    begin_detached_route_recovery(node.machine_mut().reconciler_mut(), detached).unwrap();
    assert_eq!(node.machine().reconciler().state(), NodeState::Starting);

    // Recovery reuses the same application orchestrator and owner ports. The
    // framework emits ProcessStarted and ReadinessVerified only after the fake
    // capability calls complete.
    node.recover_until_ready().await.unwrap();
    assert_eq!(node.machine().reconciler().state(), NodeState::Ready);

    let second_route = attach_ready_node_route(
        node.machine_mut().reconciler_mut(),
        || async { Ok(202_i32) },
    )
    .await
    .unwrap();
    assert_eq!(second_route, 202);
    assert_eq!(node.machine().reconciler().state(), NodeState::Routable);
}

#[tokio::test]
async fn failed_detach_cannot_enter_recovery_rail() {
    let demand = ModelDemand::new("qwen-4b").unwrap();
    let mut node = fake_orchestrator();
    node.prepare_until_ready(demand).await.unwrap();
    let route = attach_ready_node_route(
        node.machine_mut().reconciler_mut(),
        || async { Ok(303_i32) },
    )
    .await
    .unwrap();

    let result = detach_unhealthy_node_route(
        node.machine_mut().reconciler_mut(),
        route,
        |_| async { anyhow::bail!("detach failed") },
    )
    .await;

    assert!(result.is_err());
    assert_eq!(node.machine().reconciler().state(), NodeState::Unhealthy);
    assert!(!node.machine().reconciler().state().is_serving());
}

// Keep this import assertion explicit: application acceptance tests should not
// need to push successful stage evidence themselves.
#[allow(dead_code)]
fn success_evidence_is_framework_internal(_: ReconcileEvidence) {}
