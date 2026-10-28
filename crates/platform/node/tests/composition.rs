use burncloud_node_runtime::{
    deployment::{DeploymentErrorKind, DeploymentMonitorError},
    ArtifactPreparation, Digest, FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe,
    FakeProcessManager, FakeReadinessProbe, FakeRuntimePreparer, HardwareProbe, HealthProbe,
    NodeComposition, NodeState, ProcessManager, ProcessSpec, ReadinessProbe, ReadinessTarget,
    ReconcileAction, ReconcileEvidence, RuntimePreparer, RuntimeRequest,
};
use url::Url;

fn closed_monitor_errors() -> tokio::sync::mpsc::UnboundedReceiver<DeploymentMonitorError> {
    let (_, receiver) = tokio::sync::mpsc::unbounded_channel();
    receiver
}

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
        std::sync::Arc::new(FakeProcessManager::new(4242)),
        std::sync::Arc::new(FakeReadinessProbe),
        std::sync::Arc::new(FakeHealthProbe),
        closed_monitor_errors(),
    )
}

#[tokio::test]
async fn composition_root_forwards_monitor_errors_and_reports_channel_closure() {
    let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
    let expected = DeploymentMonitorError {
        model_id: "model-1".into(),
        pid: 4242,
        kind: DeploymentErrorKind::HealthProbeFailed,
        message: "probe failed".into(),
        cleanup_error: None,
    };
    assert!(sender.send(expected.clone()).is_ok());
    let mut node = NodeComposition::start(
        FakeHardwareProbe::default(),
        FakeArtifactPreparer,
        FakeRuntimePreparer,
        std::sync::Arc::new(FakeProcessManager::new(4242)),
        std::sync::Arc::new(FakeReadinessProbe),
        std::sync::Arc::new(FakeHealthProbe),
        receiver,
    );

    assert_eq!(node.next_deployment_monitor_error().await, Some(expected));
    drop(sender);
    assert_eq!(node.next_deployment_monitor_error().await, None);
}

#[tokio::test]
async fn composition_root_wires_every_machine_socket() {
    let mut node = fake_composition();
    assert!(node.context().started());

    let profile = node.hardware().inspect().await.unwrap();
    assert_eq!(profile.accelerators.len(), 1);

    let artifact = node
        .artifacts()
        .prepare(
            Url::parse("https://modelscope.cn/models/qwen-4b/resolve/main/fake.gguf").unwrap(),
            Digest::parse(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    assert!(artifact.is_verified());

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
            args: vec![artifact.local_path().to_string_lossy().into_owned()],
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
