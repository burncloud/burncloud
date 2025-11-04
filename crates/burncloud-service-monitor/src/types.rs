use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

/// 系统监控错误类型
#[derive(thiserror::Error, Debug)]
pub enum MonitorError {
    #[error("Failed to collect system information: {0}")]
    CollectionFailed(String),

    #[error("System information not available")]
    NotAvailable,

    #[error("Invalid data: {0}")]
    InvalidData(String),
}

/// 内存信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryInfo {
    /// 总内存 (字节)
    pub total: u64,
    /// 已使用内存 (字节)
    pub used: u64,
    /// 可用内存 (字节)
    pub available: u64,
    /// 使用百分比 (0-100)
    pub usage_percent: f32,
}

impl MemoryInfo {
    pub fn new(total: u64, used: u64, available: u64) -> Self {
        let usage_percent = if total > 0 {
            (used as f64 / total as f64 * 100.0) as f32
        } else {
            0.0
        };

        Self {
            total,
            used,
            available,
            usage_percent,
        }
    }

    /// 格式化内存大小显示
    pub fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;

        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }

        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.1} {}", size, UNITS[unit_index])
        }
    }

    pub fn used_formatted(&self) -> String {
        Self::format_size(self.used)
    }

    pub fn total_formatted(&self) -> String {
        Self::format_size(self.total)
    }
}

/// CPU信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuInfo {
    /// CPU使用百分比 (0-100)
    pub usage_percent: f32,
    /// CPU核心数
    pub core_count: usize,
    /// CPU频率 (MHz)
    pub frequency: u64,
    /// CPU品牌信息
    pub brand: String,
}

/// 磁盘信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    /// 总空间 (字节)
    pub total: u64,
    /// 已使用空间 (字节)
    pub used: u64,
    /// 可用空间 (字节)
    pub available: u64,
    /// 使用百分比 (0-100)
    pub usage_percent: f32,
    /// 挂载点
    pub mount_point: String,
}

/// 系统监控数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU信息
    pub cpu: CpuInfo,
    /// 内存信息
    pub memory: MemoryInfo,
    /// 磁盘信息列表
    pub disks: Vec<DiskInfo>,
    /// 数据采集时间戳 (Unix时间戳)
    pub timestamp: u64,
}

impl SystemMetrics {
    pub fn new(cpu: CpuInfo, memory: MemoryInfo, disks: Vec<DiskInfo>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            cpu,
            memory,
            disks,
            timestamp,
        }
    }
}