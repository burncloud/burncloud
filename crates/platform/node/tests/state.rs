use burncloud_node_runtime::{NodeState, NodeStateMachine};

#[test]
fn golden_runtime_path_reaches_routable_without_skipping_states() {
    let mut machine = NodeStateMachine::new();

    for next in [
        NodeState::Resolving,
        NodeState::PreparingArtifact,
        NodeState::ArtifactReady,
        NodeState::PreparingRuntime,
        NodeState::Starting,
        NodeState::Ready,
        NodeState::Routable,
    ] {
        machine.transition(next).expect("golden transition must be valid");
    }

    assert_eq!(machine.state(), NodeState::Routable);
    assert!(machine.state().is_serving());
}

#[test]
fn cannot_claim_ready_without_runtime_evidence() {
    let mut machine = NodeStateMachine::new();

    let error = machine
        .transition(NodeState::Ready)
        .expect_err("Absent -> Ready must be forbidden");

    assert_eq!(error.from, NodeState::Absent);
    assert_eq!(error.to, NodeState::Ready);
    assert_eq!(machine.state(), NodeState::Absent);
}

#[test]
fn cannot_claim_routable_before_ready() {
    let mut machine = NodeStateMachine::new();
    machine.transition(NodeState::Resolving).unwrap();

    assert!(machine.transition(NodeState::Routable).is_err());
    assert_eq!(machine.state(), NodeState::Resolving);
}

#[test]
fn failures_can_retry_from_resolution() {
    let mut machine = NodeStateMachine::new();
    machine.transition(NodeState::Resolving).unwrap();
    machine.transition(NodeState::Failed).unwrap();
    assert!(machine.state().is_failure());

    machine.transition(NodeState::Resolving).unwrap();
    assert_eq!(machine.state(), NodeState::Resolving);
}

#[test]
fn unhealthy_workload_is_not_serving() {
    let mut machine = NodeStateMachine::new();
    for next in [
        NodeState::Resolving,
        NodeState::PreparingArtifact,
        NodeState::ArtifactReady,
        NodeState::PreparingRuntime,
        NodeState::Starting,
        NodeState::Ready,
        NodeState::Routable,
        NodeState::Unhealthy,
    ] {
        machine.transition(next).unwrap();
    }

    assert!(machine.state().is_failure());
    assert!(!machine.state().is_serving());
}
