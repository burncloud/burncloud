use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe,
    FakeProcessManager, FakeReadinessProbe, FakeRuntimePreparer, HardwareProbe, HealthProbe,
    NodeComposition, NodeState, ProcessManager, ProcessSpec, ReadinessProbe, ReadinessTarget,
    ReconcileAction, ReconcileEvidence, RuntimePreparer, RuntimeRequest,
};
use burncloud_service_models::{FakeModelResolver, ModelResolutionRequest, ModelResolver};

/// Phase-0.5 acceptance: every success evidence item must follow a successful
/// call to the capability that owns that work. Tests may not claim work happened.
#[tokio::test]
async fn fake_golden_path_converges_from_absent_to_routable() {
    let mut node = NodeComposition::start(
        FakeHardwareProbe::default(),
        FakeProcessManager::new(4242),
    );
    let resolver = FakeModelResolver;
    let artifacts = FakeArtifactPreparer;
    let runtimes = FakeRuntimePreparer;
    let readiness = FakeReadinessProbe;
    let health = FakeHealthProbe;

    assert!(node.context().started());
    assert_eq!(node.reconciler().state(), NodeState::Absent);

    let hardware = node.hardware().inspect().await.expect("fake hardware probe");
    assert!(!hardware.accelerators.is_empty());

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::Resolve);
    let resolved = resolver
        .resolve(ModelResolutionRequest { model: "qwen-4b".into() })
        .await
        .expect("supply-owned fake model resolution");
    assert_eq!(resolved.runtime, "llama.cpp");
    assert!(resolved.artifact.ends_with(".gguf"));
    node.reconciler_mut().observe(ReconcileEvidence::Resolved).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::PrepareArtifact);
    let artifact = artifacts
        .prepare(ArtifactRequest {
            source: resolved.artifact,
            expected_digest: None,
        })
        .await
        .expect("fake artifact preparation");
    assert!(artifact.verified);
    node.reconciler_mut().observe(ReconcileEvidence::ArtifactPrepared).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::PrepareRuntime);
    let runtime = runtimes
        .prepare(RuntimeRequest {
            runtime: resolved.runtime,
            version: None,
        })
        .await
        .expect("fake runtime preparation");
    assert!(runtime.executable.contains("llama.cpp"));
    node.reconciler_mut().observe(ReconcileEvidence::RuntimePrepared).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::StartProcess);
    let handle = node
        .processes()
        .start(ProcessSpec {
            program: runtime.executable,
            args: vec![artifact.local_path],
        })
        .await
        .expect("fake process start");
    assert_eq!(handle.pid, 4242);
    node.reconciler_mut().observe(ReconcileEvidence::ProcessStarted).unwrap();

    assert_eq!(node.reconciler().state(), NodeState::WaitingReady);
    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::VerifyReadiness);
    let target = ReadinessTarget {
        endpoint: "http://127.0.0.1:39122/health".into(),
    };
    readiness.wait_ready(target.clone()).await.expect("fake readiness");
    assert!(health.is_healthy(target).await.expect("fake health"));
    node.reconciler_mut().observe(ReconcileEvidence::ReadinessVerified).unwrap();

    assert_eq!(node.reconciler().state(), NodeState::Ready);
    assert!(!node.reconciler().state().is_serving());
    assert_eq!(
        node.reconciler_mut().next_action().unwrap(),
        ReconcileAction::AttachToExistingRouter
    );
    node.reconciler_mut().observe(ReconcileEvidence::RouterAttached).unwrap();

    assert_eq!(node.reconciler().state(), NodeState::Routable);
    assert!(node.reconciler().state().is_serving());
    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::Noop);
    node.processes().stop(handle).await.expect("fake process stop");
}

#[tokio::test]
async fn fake_golden_path_cannot_serve_before_router_attachment_evidence() {
    let mut node = NodeComposition::start(
        FakeHardwareProbe::default(),
        FakeProcessManager::default(),
    );
    let resolver = FakeModelResolver;
    let artifacts = FakeArtifactPreparer;
    let runtimes = FakeRuntimePreparer;
    let readiness = FakeReadinessProbe;

    node.reconciler_mut().next_action().unwrap();
    let resolved = resolver
        .resolve(ModelResolutionRequest { model: "qwen-4b".into() })
        .await
        .unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::Resolved).unwrap();

    artifacts
        .prepare(ArtifactRequest { source: resolved.artifact, expected_digest: None })
        .await
        .unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ArtifactPrepared).unwrap();

    node.reconciler_mut().next_action().unwrap();
    runtimes
        .prepare(RuntimeRequest { runtime: resolved.runtime, version: None })
        .await
        .unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::RuntimePrepared).unwrap();

    let handle = node
        .processes()
        .start(ProcessSpec {
            program: "/fake/runtime/llama.cpp/server".into(),
            args: vec![],
        })
        .await
        .unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ProcessStarted).unwrap();

    readiness
        .wait_ready(ReadinessTarget { endpoint: "http://127.0.0.1:39122/health".into() })
        .await
        .unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ReadinessVerified).unwrap();

    assert_eq!(node.reconciler().state(), NodeState::Ready);
    assert!(!node.reconciler().state().is_serving());
    node.processes().stop(handle).await.unwrap();
}
