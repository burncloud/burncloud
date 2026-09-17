use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe,
    FakeProcessManager, FakeReadinessProbe, FakeRuntimePreparer, HardwareProbe, HealthProbe,
    NodeComposition, NodeState, ProcessManager, ProcessSpec, ReadinessProbe, ReadinessTarget,
    ReconcileAction, ReconcileEvidence, RuntimePreparer, RuntimeRequest,
};
use burncloud_service_models::{FakeModelResolver, ModelResolutionOutcome, ModelResolutionRequest, ModelResolver};

type FakeNode = NodeComposition<FakeHardwareProbe, FakeArtifactPreparer, FakeRuntimePreparer, FakeProcessManager, FakeReadinessProbe, FakeHealthProbe>;
fn fake_node(pid: u32) -> FakeNode { NodeComposition::start(FakeHardwareProbe::default(), FakeArtifactPreparer, FakeRuntimePreparer, FakeProcessManager::new(pid), FakeReadinessProbe, FakeHealthProbe) }

async fn resolve_local(node: &FakeNode) -> burncloud_service_models::ResolvedModel {
    let hardware = node.hardware().inspect().await.unwrap();
    let accelerator_memory_bytes = hardware.accelerators.iter().filter_map(|a| a.memory_bytes).max();
    match FakeModelResolver.resolve(ModelResolutionRequest { model: "qwen-4b".into(), accelerator_memory_bytes }).await.unwrap() {
        ModelResolutionOutcome::Local(resolved) => resolved,
        ModelResolutionOutcome::Unsupported(reason) => panic!("fake golden path unexpectedly unsupported: {reason:?}"),
    }
}

#[tokio::test]
async fn fake_golden_path_converges_from_absent_to_routable() {
    let mut node = fake_node(4242);
    assert!(node.context().started());
    assert_eq!(node.reconciler().state(), NodeState::Absent);
    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::Resolve);
    let resolved = resolve_local(&node).await;
    node.reconciler_mut().observe(ReconcileEvidence::Resolved).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::PrepareArtifact);
    let artifact = node.artifacts().prepare(ArtifactRequest { source: resolved.artifact_source, expected_digest: resolved.artifact_digest }).await.unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ArtifactPrepared).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::PrepareRuntime);
    let runtime = node.runtimes().prepare(RuntimeRequest { runtime: resolved.runtime, version: resolved.runtime_version }).await.unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::RuntimePrepared).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::StartProcess);
    let handle = node.processes().start(ProcessSpec { program: runtime.executable, args: vec![artifact.local_path] }).await.unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ProcessStarted).unwrap();

    assert_eq!(node.reconciler().state(), NodeState::WaitingReady);
    let target = ReadinessTarget { endpoint: "http://127.0.0.1:39122/health".into() };
    node.readiness().wait_ready(target.clone()).await.unwrap();
    assert!(node.health().is_healthy(target).await.unwrap());
    node.reconciler_mut().observe(ReconcileEvidence::ReadinessVerified).unwrap();
    assert_eq!(node.reconciler().state(), NodeState::Ready);
    assert!(!node.reconciler().state().is_serving());
    node.reconciler_mut().observe(ReconcileEvidence::RouterAttached).unwrap();
    assert_eq!(node.reconciler().state(), NodeState::Routable);
    assert!(node.reconciler().state().is_serving());
    node.processes().stop(handle).await.unwrap();
}

#[tokio::test]
async fn fake_golden_path_cannot_serve_before_router_attachment_evidence() {
    let mut node = fake_node(4242);
    node.reconciler_mut().next_action().unwrap();
    let resolved = resolve_local(&node).await;
    node.reconciler_mut().observe(ReconcileEvidence::Resolved).unwrap();
    let artifact = node.artifacts().prepare(ArtifactRequest { source: resolved.artifact_source, expected_digest: resolved.artifact_digest }).await.unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ArtifactPrepared).unwrap();
    node.reconciler_mut().next_action().unwrap();
    let runtime = node.runtimes().prepare(RuntimeRequest { runtime: resolved.runtime, version: resolved.runtime_version }).await.unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::RuntimePrepared).unwrap();
    let handle = node.processes().start(ProcessSpec { program: runtime.executable, args: vec![artifact.local_path] }).await.unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ProcessStarted).unwrap();
    node.readiness().wait_ready(ReadinessTarget { endpoint: "http://127.0.0.1:39122/health".into() }).await.unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ReadinessVerified).unwrap();
    assert_eq!(node.reconciler().state(), NodeState::Ready);
    assert!(!node.reconciler().state().is_serving());
    node.processes().stop(handle).await.unwrap();
}
