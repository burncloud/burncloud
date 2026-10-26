use crate::{error::HardwareProbeError, types::HardwareProfile};

/// 为缺少平台实现的编译目标提供后备探测器。
pub struct SystemHardwareProbe;

impl SystemHardwareProbe {
    /// 创建后备探测器。
    ///
    /// # 示例
    ///
    /// ```
    /// use burncloud_node_runtime::SystemHardwareProbe;
    ///
    /// let _probe = SystemHardwareProbe::new();
    /// ```
    pub fn new() -> Self {
        Self
    }

    /// 返回当前目标不受支持的错误。
    pub(crate) async fn inspect(&self) -> Result<HardwareProfile, HardwareProbeError> {
        Err(HardwareProbeError::UnsupportedPlatform(
            std::env::consts::OS.to_owned(),
        ))
    }
}
