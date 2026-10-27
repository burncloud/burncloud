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
    /// 检测本机硬件并返回稳定的机器级描述。
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

    /// 操作系统无法判断句柄对应进程是否仍在运行。
    #[error("process liveness check failed: {0}")]
    IsRunningFailed(String),
}

/// Port owned by the Node runtime for local process lifecycle only.
#[async_trait]
pub trait ProcessManager: Send + Sync {
    /// 按给定规格启动本地进程，并返回进程句柄。
    async fn start(&self, spec: ProcessSpec) -> Result<ProcessHandle, ProcessError>;
    /// 停止由本接口持有的本地进程。
    async fn stop(&self, handle: ProcessHandle) -> Result<(), ProcessError>;

    /// 查询句柄对应的进程是否仍在运行。
    ///
    /// PID 为零或无法判断进程状态时返回 [`ProcessError::IsRunningFailed`]；查询错误不表示进程已退出。默认实现返回该错误，具体进程管理器应覆写此方法。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_node_runtime::{ProcessError, ProcessHandle, ProcessManager};
    ///
    /// async fn check_process(
    ///     manager: &impl ProcessManager,
    ///     handle: ProcessHandle,
    /// ) -> Result<bool, ProcessError> {
    ///     manager.is_running(handle).await
    /// }
    /// ```
    async fn is_running(&self, handle: ProcessHandle) -> Result<bool, ProcessError> {
        if handle.pid == 0 {
            return Err(ProcessError::IsRunningFailed(
                "process PID must be non-zero".to_string(),
            ));
        }

        Err(ProcessError::IsRunningFailed(
            "process liveness query is not implemented".to_string(),
        ))
    }
}
