use burncloud_node_runtime::{FakeProcessManager, ProcessManager, ProcessSpec};

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
