pub use crate::{error::HardwareProbeError, types::HardwareProfile};
use async_trait::async_trait;

/// Node runtime 用于读取本机硬件信息的公开接口。
/// GPU 单独不可用时仍返回通用硬件信息，加速器列表可以为空或包含部分结果。
///
/// # 示例
///
/// ```no_run
/// use burncloud_node_runtime::{HardwareProbe, SystemHardwareProbe};
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let profile = SystemHardwareProbe::new().inspect().await?;
/// println!("{}", profile.cpu);
/// # Ok(())
/// # }
/// ```
#[async_trait]
pub trait HardwareProbe: Send + Sync {
    /// 检测本机硬件并返回稳定的机器级描述。
    ///
    /// 当前工作目录不可映射到系统可见磁盘时返回 `DiskError`。
    async fn inspect(&self) -> Result<HardwareProfile, HardwareProbeError>;
}

/// Process specification for a local executable prepared by a higher-level
/// owner. The Node runtime executes it, but does not decide which BurnCloud
/// model/provider/route should be used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSpec {
    /// 要启动的可执行文件路径或可执行文件名。
    pub program: String,
    /// 按原顺序传给可执行文件的命令行参数。
    pub args: Vec<String>,
}

/// 由操作系统返回的本地子进程句柄。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessHandle {
    /// 操作系统分配的非零进程 ID。
    pub pid: u32,
}

/// 本地进程生命周期操作失败时的错误。
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProcessError {
    /// 启动计划无法交给操作系统执行。
    #[error("process start failed: {0}")]
    StartFailed(String),

    /// 已登记的子进程无法被停止。
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
