use burncloud_node_runtime::{DemandReconciler, NodeState, ReconcileAction, ReconcileEvidence};

#[test]
fn reconciler_drives_golden_path_one_evidence_backed_step_at_a_time() {
    let mut r = DemandReconciler::new();

    assert_eq!(r.next_action().unwrap(), ReconcileAction::Resolve);
    r.observe(ReconcileEvidence::Resolved).unwrap();

    assert_eq!(r.next_action().unwrap(), ReconcileAction::PrepareArtifact);
    r.observe(ReconcileEvidence::ArtifactPrepared).unwrap();

    assert_eq!(r.next_action().unwrap(), ReconcileAction::PrepareRuntime);
    r.observe(ReconcileEvidence::RuntimePrepared).unwrap();

    assert_eq!(r.next_action().unwrap(), ReconcileAction::StartProcess);
    r.observe(ReconcileEvidence::ProcessStarted).unwrap();
    assert_eq!(r.state(), NodeState::WaitingReady);

    assert_eq!(r.next_action().unwrap(), ReconcileAction::VerifyReadiness);
    r.observe(ReconcileEvidence::ReadinessVerified).unwrap();
    assert_eq!(r.state(), NodeState::Ready);

    assert_eq!(r.next_action().unwrap(), ReconcileAction::AttachToExistingRouter);
    r.observe(ReconcileEvidence::RouterAttached).unwrap();

    assert_eq!(r.state(), NodeState::Routable);
    assert_eq!(r.next_action().unwrap(), ReconcileAction::Noop);
}

#[test]
fn spawned_process_is_not_ready() {
    let mut r = DemandReconciler::new();
    r.next_action().unwrap();
    r.observe(ReconcileEvidence::Resolved).unwrap();
    r.observe(ReconcileEvidence::ArtifactPrepared).unwrap();
    r.next_action().unwrap();
    r.observe(ReconcileEvidence::RuntimePrepared).unwrap();
    r.observe(ReconcileEvidence::ProcessStarted).unwrap();

    assert_eq!(r.state(), NodeState::WaitingReady);
    assert_ne!(r.state(), NodeState::Ready);
    assert!(!r.state().is_serving());
}

#[test]
fn wrong_evidence_cannot_skip_the_rail() {
    let mut r = DemandReconciler::new();
    r.next_action().unwrap();

    assert!(r.observe(ReconcileEvidence::RouterAttached).is_err());
    assert_eq!(r.state(), NodeState::Resolving);
}
