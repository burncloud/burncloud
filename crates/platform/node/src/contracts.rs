pub use crate::{error::HardwareProbeError, types::HardwareProfile};
use async_trait::async_trait;

/// Node runtime 用于读取本机硬件信息的公开接口。
///
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
    /// 按给定规格启动本地进程，并返回进程句柄。
    async fn start(&self, spec: ProcessSpec) -> Result<ProcessHandle, ProcessError>;
    /// 停止由本接口持有的本地进程。
    async fn stop(&self, handle: ProcessHandle) -> Result<(), ProcessError>;
}
