pub mod collectors;
pub mod service;
pub mod types;

// 重新导出主要的公共API
pub use types::{CpuInfo, DiskInfo, MemoryInfo, MonitorError, SystemMetrics};

pub use service::{SystemMonitor, SystemMonitorService};

pub use collectors::{CpuCollector, DetailedMemoryInfo, DiskCollector, MemoryCollector};
