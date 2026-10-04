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

    /// Format a byte count for display.
    ///
    /// **The units are binary and are labelled as such.** The divisor is 1024, so the values are kibibytes,
    /// mebibytes and so on, and the suffixes are `KiB`/`MiB`/`GiB`/`TiB`. They were previously written `KB`,
    /// `MB`, `GB`, `TB`, which are the **decimal** units: `1024` bytes printed as `1.0 KB` is wrong by 2.4%,
    /// and the error grows with each step. `B` is unaffected because a byte is a byte.
    ///
    /// Recorded rather than chosen: whether the display should switch to decimal units (dividing by 1000) is a
    /// presentation decision for whoever owns the UI; what is corrected here is the mismatch between the
    /// divisor and the label, which is a defect under either convention.
    pub fn format_size(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
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

    /// The same value with an explicit timestamp.
    ///
    /// Exists so the cache's freshness decision can be exercised at a chosen instant rather than only at
    /// whatever the clock says, which is what the crate's plan asks for ("可控时钟/样本用于确定性校验"). The
    /// public [`SystemMetrics::new`] is unchanged and still reads the clock.
    pub fn with_timestamp(
        cpu: CpuInfo,
        memory: MemoryInfo,
        disks: Vec<DiskInfo>,
        timestamp: u64,
    ) -> Self {
        Self {
            cpu,
            memory,
            disks,
            timestamp,
        }
    }
}

/// Whether metrics collected at `collected_at` are still fresh at `now`, given `ttl_secs`.
///
/// **Extracted from the service so the decision is testable, and because the subtraction it replaces could
/// underflow.** The original check was
///
/// ```text
/// if now - metrics.timestamp < self.update_interval.as_secs()
/// ```
///
/// on `u64`: when the clock moves **backwards** -- an NTP correction, a VM restored from a snapshot, a
/// container whose clock is set from the host -- `now` is smaller than `collected_at` and the subtraction
/// underflows. In a debug build that panics, which would take the caller down; in a release build it wraps to
/// a value near `u64::MAX`, so the comparison is false and the cache is treated as **infinitely stale**,
/// meaning every call collects again. This function treats a timestamp in the future as not fresh, which is
/// the safe reading: the data cannot be shown to be recent.
///
/// A timestamp equal to `now` is fresh, and so is one exactly `ttl_secs` old -- the boundary is `<`, as it
/// was.
pub fn is_fresh(collected_at: u64, now: u64, ttl_secs: u64) -> bool {
    match now.checked_sub(collected_at) {
        // The clock has not moved backwards, so the age is meaningful.
        Some(age) => age < ttl_secs,
        // `collected_at` is in the future. Not fresh: a timestamp ahead of the clock cannot be evidence that
        // the data is recent, and the alternative -- treating it as age zero -- would pin a stale value in the
        // cache until the clock caught up.
        None => false,
    }
}
