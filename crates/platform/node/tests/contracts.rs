use async_trait::async_trait;
use burncloud_node_runtime::{
    HardwareProbe, ProcessError, ProcessHandle, ProcessManager, ProcessSpec, SystemHardwareProbe,
};

struct FakeProcess;

#[async_trait]
impl ProcessManager for FakeProcess {
    async fn start(&self, _spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        Ok(ProcessHandle { pid: 42 })
    }

    async fn stop(&self, _handle: ProcessHandle) -> Result<(), ProcessError> {
        Ok(())
    }
}

#[tokio::test]
async fn hardware_contract_uses_the_system_probe_without_business_types() {
    let probe: Box<dyn HardwareProbe> = Box::new(SystemHardwareProbe::new());
    let profile = probe.inspect().await.expect("真实硬件探测应成功");
    assert!(!profile.cpu.trim().is_empty());
    assert!(profile.memory_bytes > 0);
}

#[tokio::test]
async fn process_contract_only_manages_local_process_lifecycle() {
    let manager = FakeProcess;
    let handle = manager
        .start(ProcessSpec {
            program: "fake-runtime".to_string(),
            args: vec!["--serve".to_string()],
        })
        .await
        .expect("fake process start must succeed");

    assert_eq!(handle.pid, 42);
    manager
        .stop(handle)
        .await
        .expect("fake process stop must succeed");
}
