use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, DemandReconciler, FakeArtifactPreparer, FakeReadinessProbe,
    FakeRuntimePreparer, NodeState, ReadinessProbe, ReadinessTarget, ReconcileAction,
    ReconcileEvidence, RuntimePreparer, RuntimeRequest,
};

/// Evidence is a receipt for a completed capability call, not permission to
/// skip the capability. This test demonstrates the intended orchestration
/// pattern for future real adapters.
#[tokio::test]
async fn evidence_follows_successful_capability_calls() {
    let mut r = DemandReconciler::new();

    assert_eq!(r.next_action().unwrap(), ReconcileAction::Resolve);
    // Resolution is intentionally external to platform/node.
    r.observe(ReconcileEvidence::Resolved).unwrap();

    let artifact = FakeArtifactPreparer
        .prepare(ArtifactRequest {
            source: "qwen-4b.gguf".into(),
            expected_digest: None,
        })
        .await
        .unwrap();
    assert!(artifact.verified);
    r.observe(ReconcileEvidence::ArtifactPrepared).unwrap();

    assert_eq!(r.next_action().unwrap(), ReconcileAction::PrepareRuntime);
    let runtime = FakeRuntimePreparer
        .prepare(RuntimeRequest {
            runtime: "llama.cpp".into(),
            version: None,
        })
        .await
        .unwrap();
    assert!(!runtime.executable.is_empty());
    r.observe(ReconcileEvidence::RuntimePrepared).unwrap();

    // Process execution is separately owned by ProcessManager and is already
    // covered by the golden path. Move to readiness to focus this test on the
    // newly added ports.
    r.observe(ReconcileEvidence::ProcessStarted).unwrap();
    assert_eq!(r.state(), NodeState::WaitingReady);

    FakeReadinessProbe
        .wait_ready(ReadinessTarget {
            endpoint: "fake://ready".into(),
        })
        .await
        .unwrap();
    r.observe(ReconcileEvidence::ReadinessVerified).unwrap();

    assert_eq!(r.state(), NodeState::Ready);
}
