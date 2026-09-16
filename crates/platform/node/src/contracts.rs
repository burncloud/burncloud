use async_trait::async_trait;

/// Stable machine-level facts produced by the Node runtime.
///
/// This type deliberately contains no BurnCloud business concepts such as
/// Model, Provider, Route, Billing, or Tenant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HardwareProfile {
    pub cpu_threads: usize,
    pub memory_bytes: u64,
    pub disk_available_bytes: u64,
    pub accelerators: Vec<AcceleratorProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceleratorProfile {
    pub kind: AcceleratorKind,
    pub name: String,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceleratorKind {
    Nvidia,
    Amd,
    Apple,
    Other,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum HardwareProbeError {
    #[error("hardware detection failed: {0}")]
    DetectionFailed(String),
}

/// Port owned by the Node runtime for inspecting the local machine.
///
/// Implementations may use platform APIs or vendor tooling, but callers only
/// receive the stable machine-level `HardwareProfile` contract.
#[async_trait]
pub trait HardwareProbe: Send + Sync {
    async fn inspect(&self) -> Result<HardwareProfile, HardwareProbeError>;
}

/// Process specification for a local executable prepared by a higher-level
/// owner. The Node runtime executes it, but does not decide which BurnCloud
/// model/provider/route should be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSpec {
    pub program: String,
    pub args: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessHandle {
    pub pid: u32,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProcessError {
    #[error("process start failed: {0}")]
    StartFailed(String),

    #[error("process stop failed: {0}")]
    StopFailed(String),
}

/// Port owned by the Node runtime for local process lifecycle only.
#[async_trait]
pub trait ProcessManager: Send + Sync {
    async fn start(&self, spec: ProcessSpec) -> Result<ProcessHandle, ProcessError>;
    async fn stop(&self, handle: ProcessHandle) -> Result<(), ProcessError>;
}
