use async_trait::async_trait;

use crate::{
    AcceleratorKind, AcceleratorProfile, ArtifactPrepareError, ArtifactPreparer, ArtifactRequest,
    HardwareProbe, HardwareProbeError, HardwareProfile, HealthError, HealthProbe, PreparedArtifact,
    PreparedRuntime, ProcessError, ProcessHandle, ProcessManager, ProcessSpec, ReadinessError,
    ReadinessProbe, ReadinessTarget, RuntimePrepareError, RuntimePreparer, RuntimeRequest,
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

/// Deterministic fake artifact preparer. It performs no download or file I/O.
#[derive(Debug, Clone, Default)]
pub struct FakeArtifactPreparer;

#[async_trait]
impl ArtifactPreparer for FakeArtifactPreparer {
    async fn prepare(
        &self,
        request: ArtifactRequest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        Ok(PreparedArtifact {
            local_path: format!("/fake/artifacts/{}", request.source.replace('/', "_")),
            verified: true,
        })
    }
}

/// Deterministic fake runtime preparer. It never downloads llama.cpp or any
/// other runtime; it only proves the runtime preparation contract can be wired.
#[derive(Debug, Clone, Default)]
pub struct FakeRuntimePreparer;

#[async_trait]
impl RuntimePreparer for FakeRuntimePreparer {
    async fn prepare(
        &self,
        request: RuntimeRequest,
    ) -> Result<PreparedRuntime, RuntimePrepareError> {
        Ok(PreparedRuntime {
            executable: format!("/fake/runtime/{}/server", request.runtime),
        })
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

/// Deterministic fake readiness probe. It performs no network request.
#[derive(Debug, Clone, Default)]
pub struct FakeReadinessProbe;

#[async_trait]
impl ReadinessProbe for FakeReadinessProbe {
    async fn wait_ready(&self, _target: ReadinessTarget) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// Deterministic fake health probe. It performs no network request.
#[derive(Debug, Clone, Default)]
pub struct FakeHealthProbe;

#[async_trait]
impl HealthProbe for FakeHealthProbe {
    async fn is_healthy(&self, _target: ReadinessTarget) -> Result<bool, HealthError> {
        Ok(true)
    }
}
