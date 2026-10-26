use serde::{Deserialize, Serialize};

/// 当前进程可读取的硬件信息。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::HardwareProfile;
///
/// let profile = HardwareProfile {
///     cpu: "Example CPU".to_owned(),
///     memory_bytes: 1024,
///     disk_available_bytes: 2048,
///     accelerators: Vec::new(),
/// };
/// assert_eq!(profile.memory_bytes, 1024);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareProfile {
    /// 操作系统报告的 CPU 型号；无法读取时为 `Unknown CPU`。
    pub cpu: String,
    /// 物理内存总量，单位为字节。
    pub memory_bytes: u64,
    /// 当前工作目录所在文件系统的可用空间，单位为字节。
    pub disk_available_bytes: u64,
    /// 已识别的加速器；无设备或 GPU 探测不可用时可以为空。
    pub accelerators: Vec<AcceleratorProfile>,
}

/// 一块已识别的图形或计算加速器。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::{AcceleratorKind, AcceleratorProfile};
///
/// let gpu = AcceleratorProfile {
///     kind: AcceleratorKind::Intel,
///     name: "Example GPU".to_owned(),
///     memory_bytes: None,
/// };
/// assert_eq!(gpu.kind, AcceleratorKind::Intel);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceleratorProfile {
    /// 加速器厂商或类别。
    pub kind: AcceleratorKind,
    /// 操作系统或驱动报告的设备名称。
    pub name: String,
    /// 设备内存容量，单位为字节；Apple Silicon 使用系统物理内存总量表示统一内存。
    /// `None` 表示平台无法报告可用容量。
    pub memory_bytes: Option<u64>,
}

/// 支持识别的加速器厂商类别。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::AcceleratorKind;
///
/// let vendor = AcceleratorKind::Intel;
/// assert_eq!(vendor, AcceleratorKind::Intel);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcceleratorKind {
    Nvidia,
    Amd,
    Intel,
    Apple,
    Other,
}

/// 当前编译目标对应的平台。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::Platform;
///
/// let _platform = Platform::Windows;
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Linux,
    MacOS,
    Windows,
    Other,
}

/// 返回当前编译目标的平台。
///
/// # 示例
///
/// ```
/// use burncloud_node_runtime::{detect_platform, Platform};
///
/// let _platform = detect_platform();
/// ```
pub(crate) const fn detect_platform() -> Platform {
    #[cfg(target_os = "linux")]
    {
        Platform::Linux
    }
    #[cfg(target_os = "macos")]
    {
        Platform::MacOS
    }
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        Platform::Other
    }
}

/// 返回 Rust 编译目标的操作系统标识，而非运行时可变属性。
pub(crate) const fn current_os() -> &'static str {
    std::env::consts::OS
}

/// 返回 Rust 编译目标的体系结构标识，而非运行时可变属性。
pub(crate) const fn current_arch() -> &'static str {
    std::env::consts::ARCH
}
