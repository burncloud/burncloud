use std::fmt;

#[derive(Debug)]
pub enum HardwareError {
    GpuDriverUnavailable(String),
    CommandExecutionFailed(String),
    ParseError(String),
    UnsupportedGpuVendor(String),
    IoError(std::io::Error),
    SystemInfoError(String),
    InsufficientVram { required_mb: u64, available_mb: u64 },
    InsufficientRam { required_mb: u64, available_mb: u64 },
    InsufficientDisk { required_mb: u64, available_mb: u64 },
    DevicePermissionDenied(String),
    RuntimeIncompatible { runtime: String, reason: String },
}

impl fmt::Display for HardwareError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GpuDriverUnavailable(s) => write!(f, "GPU driver unavailable: {s}"),
            Self::CommandExecutionFailed(s) => write!(f, "command execution failed: {s}"),
            Self::ParseError(s) => write!(f, "parse error: {s}"),
            Self::UnsupportedGpuVendor(s) => write!(f, "unsupported GPU vendor: {s}"),
            Self::IoError(e) => write!(f, "I/O error: {e}"),
            Self::SystemInfoError(s) => write!(f, "system information error: {s}"),
            Self::InsufficientVram { required_mb, available_mb } => write!(f, "insufficient VRAM: required {required_mb} MB, available {available_mb} MB"),
            Self::InsufficientRam { required_mb, available_mb } => write!(f, "insufficient RAM: required {required_mb} MB, available {available_mb} MB"),
            Self::InsufficientDisk { required_mb, available_mb } => write!(f, "insufficient disk: required {required_mb} MB, available {available_mb} MB"),
            Self::DevicePermissionDenied(s) => write!(f, "device permission denied: {s}"),
            Self::RuntimeIncompatible { runtime, reason } => write!(f, "runtime {runtime} is incompatible: {reason}"),
        }
    }
}
impl std::error::Error for HardwareError {}
impl From<std::io::Error> for HardwareError { fn from(e: std::io::Error) -> Self { Self::IoError(e) } }
