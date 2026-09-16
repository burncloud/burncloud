use async_trait::async_trait;
use burncloud_node_runtime::{
    AcceleratorKind, AcceleratorProfile, HardwareProbe, HardwareProbeError, HardwareProfile,
    ProcessError, ProcessHandle, ProcessManager, ProcessSpec,
};

struct FakeHardware;

#[async_trait]
impl HardwareProbe for FakeHardware {
    async fn inspect(&self) -> Result<HardwareProfile, HardwareProbeError> {
        Ok(HardwareProfile {
            cpu_threads: 16,
            memory_bytes: 64 * 1024 * 1024 * 1024,
            disk_available_bytes: 512 * 1024 * 1024 * 1024,
            accelerators: vec![AcceleratorProfile {
                kind: AcceleratorKind::Nvidia,
                name: "Fake GPU".to_string(),
                memory_bytes: Some(24 * 1024 * 1024 * 1024),
            }],
        })
    }
}

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
async fn hardware_contract_can_be_implemented_without_business_types() {
    let profile = FakeHardware.inspect().await.expect("fake hardware probe must succeed");
    assert_eq!(profile.cpu_threads, 16);
    assert_eq!(profile.accelerators.len(), 1);
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
    manager.stop(handle).await.expect("fake process stop must succeed");
}
