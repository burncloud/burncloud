use burncloud_node_runtime::{
    FakeHardwareProbe, FakeProcessManager, HardwareProbe, NodeComposition, NodeState, ProcessManager,
    ProcessSpec, ReconcileAction, ReconcileEvidence,
};

/// Phase-0 acceptance test: prove the complete Node skeleton can converge from
/// ABSENT to ROUTABLE using deterministic fake stage owners only.
///
/// This deliberately does not claim real model resolution, downloads, runtime
/// preparation, readiness probes, or router mutation. Those implementations
/// belong to later phases and/or their owning business domains.
#[tokio::test]
async fn fake_golden_path_converges_from_absent_to_routable() {
    let mut node = NodeComposition::start(
        FakeHardwareProbe::default(),
        FakeProcessManager::new(4242),
    );

    assert!(node.context().started());
    assert_eq!(node.reconciler().state(), NodeState::Absent);

    // Machine evidence exists before higher-level resolution begins.
    let hardware = node.hardware().inspect().await.expect("fake hardware probe");
    assert!(!hardware.accelerators.is_empty());

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::Resolve);
    node.reconciler_mut().observe(ReconcileEvidence::Resolved).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::PrepareArtifact);
    node.reconciler_mut().observe(ReconcileEvidence::ArtifactPrepared).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::PrepareRuntime);
    node.reconciler_mut().observe(ReconcileEvidence::RuntimePrepared).unwrap();

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::StartProcess);
    let handle = node
        .processes()
        .start(ProcessSpec {
            program: "fake-inference-runtime".into(),
            args: vec!["--fake".into()],
        })
        .await
        .expect("fake process start");
    assert_eq!(handle.pid, 4242);
    node.reconciler_mut().observe(ReconcileEvidence::ProcessStarted).unwrap();

    // ProcessStarted is intentionally insufficient for READY.
    assert_eq!(node.reconciler().state(), NodeState::WaitingReady);
    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::VerifyReadiness);
    node.reconciler_mut().observe(ReconcileEvidence::ReadinessVerified).unwrap();

    assert_eq!(node.reconciler().state(), NodeState::Ready);
    assert!(!node.reconciler().state().is_serving());

    // Phase 0 uses evidence only; it does not mutate a real ModelRouter.
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

    node.reconciler_mut().next_action().unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::Resolved).unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ArtifactPrepared).unwrap();
    node.reconciler_mut().next_action().unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::RuntimePrepared).unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ProcessStarted).unwrap();
    node.reconciler_mut().observe(ReconcileEvidence::ReadinessVerified).unwrap();

    assert_eq!(node.reconciler().state(), NodeState::Ready);
    assert!(!node.reconciler().state().is_serving());
}
