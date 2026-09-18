use burncloud_node_runtime::{NodeState, NodeStateMachine};

fn golden_path() -> [NodeState; 8] {
    [
        NodeState::Resolving,
        NodeState::PreparingArtifact,
        NodeState::ArtifactReady,
        NodeState::PreparingRuntime,
        NodeState::Starting,
        NodeState::WaitingReady,
        NodeState::Ready,
        NodeState::Routable,
    ]
}

#[test]
fn golden_runtime_path_reaches_routable_without_skipping_states() {
    let mut machine = NodeStateMachine::new();
    for next in golden_path() {
        machine
            .transition(next)
            .expect("golden transition must be valid");
    }
    assert_eq!(machine.state(), NodeState::Routable);
    assert!(machine.state().is_serving());
}

#[test]
fn process_spawn_does_not_prove_readiness() {
    let mut machine = NodeStateMachine::new();
    for next in golden_path().into_iter().take(5) {
        machine.transition(next).unwrap();
    }
    assert_eq!(machine.state(), NodeState::Starting);
    assert!(machine.transition(NodeState::Ready).is_err());
    machine.transition(NodeState::WaitingReady).unwrap();
    assert!(!machine.state().is_serving());
}

#[test]
fn cannot_claim_ready_without_runtime_evidence() {
    let mut machine = NodeStateMachine::new();
    let error = machine
        .transition(NodeState::Ready)
        .expect_err("Absent -> Ready must be forbidden");
    assert_eq!(error.from, NodeState::Absent);
    assert_eq!(error.to, NodeState::Ready);
}

#[test]
fn cannot_claim_routable_before_ready() {
    let mut machine = NodeStateMachine::new();
    machine.transition(NodeState::Resolving).unwrap();
    assert!(machine.transition(NodeState::Routable).is_err());
}

#[test]
fn failures_can_retry_from_resolution() {
    let mut machine = NodeStateMachine::new();
    machine.transition(NodeState::Resolving).unwrap();
    machine.transition(NodeState::Failed).unwrap();
    assert!(machine.state().is_failure());
    machine.transition(NodeState::Resolving).unwrap();
}

#[test]
fn unhealthy_workload_is_not_serving() {
    let mut machine = NodeStateMachine::new();
    for next in golden_path() {
        machine.transition(next).unwrap();
    }
    machine.transition(NodeState::Unhealthy).unwrap();
    assert!(machine.state().is_failure());
    assert!(!machine.state().is_serving());
}
