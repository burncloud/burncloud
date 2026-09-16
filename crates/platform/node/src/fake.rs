use async_trait::async_trait;

use crate::{
    AcceleratorKind, AcceleratorProfile, HardwareProbe, HardwareProbeError, HardwareProfile,
    ProcessError, ProcessHandle, ProcessManager, ProcessSpec,
};

/// Deterministic fake used to assemble and test the Node skeleton before real
/// hardware detection exists.
#[derive(Debug, Clone)]
pub struct FakeHardwareProbe {
    profile: HardwareProfile,
}

impl Default for FakeHardwareProbe {
    fn default() -> Self {
        Self {
            profile: HardwareProfile {
                cpu_threads: 16,
                memory_bytes: 64 * 1024 * 1024 * 1024,
                disk_available_bytes: 512 * 1024 * 1024 * 1024,
                accelerators: vec![AcceleratorProfile {
                    kind: AcceleratorKind::Nvidia,
                    name: "Fake GPU".to_string(),
                    memory_bytes: Some(24 * 1024 * 1024 * 1024),
                }],
            },
        }
    }
}

impl FakeHardwareProbe {
    pub fn new(profile: HardwareProfile) -> Self {
        Self { profile }
    }
}

#[async_trait]
impl HardwareProbe for FakeHardwareProbe {
    async fn inspect(&self) -> Result<HardwareProfile, HardwareProbeError> {
        Ok(self.profile.clone())
    }
}

/// Deterministic fake process manager. It never spawns an OS process.
#[derive(Debug, Clone)]
pub struct FakeProcessManager {
    pid: u32,
}

impl Default for FakeProcessManager {
    fn default() -> Self {
        Self { pid: 42 }
    }
}

impl FakeProcessManager {
    pub fn new(pid: u32) -> Self {
        Self { pid }
    }
}

#[async_trait]
impl ProcessManager for FakeProcessManager {
    async fn start(&self, _spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        Ok(ProcessHandle { pid: self.pid })
    }

    async fn stop(&self, _handle: ProcessHandle) -> Result<(), ProcessError> {
        Ok(())
    }
}
