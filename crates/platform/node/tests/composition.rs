use burncloud_node_runtime::{
    FakeHardwareProbe, FakeProcessManager, HardwareProbe, NodeComposition, NodeState,
    ProcessManager, ProcessSpec, ReconcileAction, ReconcileEvidence,
};

#[tokio::test]
async fn composition_root_wires_runtime_reconciler_and_replaceable_adapters() {
    let mut node = NodeComposition::start(
        FakeHardwareProbe::default(),
        FakeProcessManager::new(4242),
    );

    assert!(node.context().started());

    let profile = node.hardware().inspect().await.expect("fake hardware must work");
    assert_eq!(profile.accelerators.len(), 1);

    assert_eq!(node.reconciler_mut().next_action().unwrap(), ReconcileAction::Resolve);
    node.reconciler_mut().observe(ReconcileEvidence::Resolved).unwrap();
    assert_eq!(node.reconciler().state(), NodeState::PreparingArtifact);

    let handle = node
        .processes()
        .start(ProcessSpec {
            program: "fake-runtime".into(),
            args: vec![],
        })
        .await
        .expect("fake process must work");
    assert_eq!(handle.pid, 4242);
    node.processes().stop(handle).await.expect("fake process must stop");
}

#[test]
fn composition_root_does_not_make_a_node_routable_on_start() {
    let node = NodeComposition::start(FakeHardwareProbe::default(), FakeProcessManager::default());
    assert_eq!(node.reconciler().state(), NodeState::Absent);
    assert!(!node.reconciler().state().is_serving());
}
