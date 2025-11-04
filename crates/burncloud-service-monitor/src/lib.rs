pub mod types;
pub mod collectors;
pub mod service;

// 重新导出主要的公共API
pub use types::{
    MonitorError,
    SystemMetrics,
    MemoryInfo,
    CpuInfo,
    DiskInfo
};

pub use service::{
    SystemMonitorService,
    SystemMonitor
};

pub use collectors::{
    CpuCollector,
    MemoryCollector,
    DiskCollector,
    DetailedMemoryInfo
};