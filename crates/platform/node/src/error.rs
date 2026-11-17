use thiserror::Error;

/// 硬件探测过程中可能发生的错误。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::HardwareProbeError;
///
/// let error = HardwareProbeError::DiskError("unavailable".to_owned());
/// assert!(error.to_string().contains("unavailable"));
/// ```
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HardwareProbeError {
    /// 阻塞探测任务无法启动或完成。
    #[error("hardware detection failed: {0}")]
    DetectionFailed(String),
    /// GPU 驱动或查询接口不可用；探测器会将此错误降级为空或部分结果。
    #[error("GPU driver is unavailable: {0}")]
    GpuDriverUnavailable(String),
    /// 无法读取当前工作目录，或找不到包含该目录的可见磁盘。
    #[error("disk could not be inspected: {0}")]
    DiskError(String),
    /// 当前编译目标没有硬件探测实现。
    #[error("unsupported platform: {0}")]
    UnsupportedPlatform(String),
}
