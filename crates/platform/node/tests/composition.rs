use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe,
    FakeProcessManager, FakeReadinessProbe, FakeRuntimePreparer, HardwareProbe, HealthProbe,
    NodeComposition, NodeState, ProcessManager, ProcessSpec, ReadinessProbe, ReadinessTarget,
    ReconcileAction, ReconcileEvidence, RuntimePreparer, RuntimeRequest,
};

fn fake_composition() -> NodeComposition<
    FakeHardwareProbe,
    FakeArtifactPreparer,
    FakeRuntimePreparer,
    FakeProcessManager,
    FakeReadinessProbe,
    FakeHealthProbe,
> {
    NodeComposition::start(
        FakeHardwareProbe::default(),
        FakeArtifactPreparer,
        FakeRuntimePreparer,
        FakeProcessManager::new(4242),
        FakeReadinessProbe,
        FakeHealthProbe,
    )
}

#[tokio::test]
async fn composition_root_wires_every_machine_socket() {
    let mut node = fake_composition();
    assert!(node.context().started());

    let profile = node.hardware().inspect().await.unwrap();
    assert_eq!(profile.accelerators.len(), 1);

    let artifact = node
        .artifacts()
        .prepare(ArtifactRequest {
            source: "qwen/fake.gguf".into(),
            expected_digest: None,
        })
        .await
        .unwrap();
    assert!(artifact.verified);

    let runtime = node
        .runtimes()
        .prepare(RuntimeRequest {
            runtime: "llama.cpp".into(),
            version: None,
        })
        .await
        .unwrap();

    let handle = node
        .processes()
        .start(ProcessSpec {
            program: runtime.executable,
            args: vec![artifact.local_path],
        })
        .await
        .unwrap();
    assert_eq!(handle.pid, 4242);

    let target = ReadinessTarget {
        endpoint: "http://127.0.0.1:39122/health".into(),
    };
    node.readiness().wait_ready(target.clone()).await.unwrap();
    assert!(node.health().is_healthy(target).await.unwrap());
    node.processes().stop(handle).await.unwrap();

    assert_eq!(
        node.reconciler_mut().next_action().unwrap(),
        ReconcileAction::Resolve
    );
    node.reconciler_mut()
        .observe(ReconcileEvidence::Resolved)
        .unwrap();
    assert_eq!(node.reconciler().state(), NodeState::PreparingArtifact);
}

#[test]
fn composition_root_does_not_make_a_node_routable_on_start() {
    let node = fake_composition();
    assert_eq!(node.reconciler().state(), NodeState::Absent);
    assert!(!node.reconciler().state().is_serving());
}
