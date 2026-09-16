use burncloud_node_runtime::{
    FakeHardwareProbe, FakeProcessManager, HardwareProbe, ProcessManager, ProcessSpec,
};

#[tokio::test]
async fn fake_hardware_is_deterministic() {
    let probe = FakeHardwareProbe::default();
    let first = probe.inspect().await.expect("fake probe must succeed");
    let second = probe.inspect().await.expect("fake probe must succeed");

    assert_eq!(first, second);
    assert_eq!(first.accelerators.len(), 1);
}

#[tokio::test]
async fn fake_process_never_requires_a_real_os_process() {
    let manager = FakeProcessManager::new(4242);
    let handle = manager
        .start(ProcessSpec {
            program: "not-a-real-runtime".to_string(),
            args: vec![],
        })
        .await
        .expect("fake process must start");

    assert_eq!(handle.pid, 4242);
    manager.stop(handle).await.expect("fake process must stop");
}
