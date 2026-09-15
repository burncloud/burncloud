use crate::types::{MemoryInfo, MonitorError};

#[cfg(windows)]
use winapi::um::sysinfoapi::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

#[cfg(unix)]
use std::fs;

/// 内存数据收集器
pub struct MemoryCollector;

impl MemoryCollector {
    /// 创建新的内存收集器
    pub fn new() -> Self {
        Self
    }

    /// 收集内存信息
    pub async fn collect(&self) -> Result<MemoryInfo, MonitorError> {
        #[cfg(windows)]
        {
            self.collect_windows().await
        }
        #[cfg(unix)]
        {
            self.collect_unix().await
        }
    }

    #[cfg(windows)]
    async fn collect_windows(&self) -> Result<MemoryInfo, MonitorError> {
        unsafe {
            let mut mem_status: MEMORYSTATUSEX = std::mem::zeroed();
            mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

            if GlobalMemoryStatusEx(&mut mem_status) == 0 {
                return Err(MonitorError::CollectionFailed(
                    "Failed to get memory status".to_string(),
                ));
            }

            let total = mem_status.ullTotalPhys;
            let available = mem_status.ullAvailPhys;
            let used = total - available;

            Ok(MemoryInfo::new(total, used, available))
        }
    }

    #[cfg(unix)]
    async fn collect_unix(&self) -> Result<MemoryInfo, MonitorError> {
        let meminfo_content = fs::read_to_string("/proc/meminfo").map_err(|e| {
            MonitorError::CollectionFailed(format!("Failed to read /proc/meminfo: {}", e))
        })?;

        let mut total_kb = 0u64;
        let mut available_kb = 0u64;
        let mut free_kb = 0u64;
        let mut buffers_kb = 0u64;
        let mut cached_kb = 0u64;

        for line in meminfo_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let value = parts[1].parse::<u64>().unwrap_or(0);
                match parts[0] {
                    "MemTotal:" => total_kb = value,
                    "MemAvailable:" => available_kb = value,
                    "MemFree:" => free_kb = value,
                    "Buffers:" => buffers_kb = value,
                    "Cached:" => cached_kb = value,
                    _ => {}
                }
            }
        }

        if total_kb == 0 {
            return Err(MonitorError::NotAvailable);
        }

        let total = total_kb * 1024; // 转换为字节

        // 如果没有 MemAvailable，使用 MemFree + Buffers + Cached 估算
        let available = if available_kb > 0 {
            available_kb * 1024
        } else {
            (free_kb + buffers_kb + cached_kb) * 1024
        };

        let used = total - available;

        Ok(MemoryInfo::new(total, used, available))
    }

    /// 获取详细内存信息
    pub async fn get_detailed_info(&self) -> Result<DetailedMemoryInfo, MonitorError> {
        #[cfg(windows)]
        {
            self.get_detailed_info_windows().await
        }
        #[cfg(unix)]
        {
            self.get_detailed_info_unix().await
        }
    }

    #[cfg(windows)]
    async fn get_detailed_info_windows(&self) -> Result<DetailedMemoryInfo, MonitorError> {
        let memory = self.collect_windows().await?;

        // Windows doesn't easily expose swap information through winapi
        // 可以通过性能计数器或WMI获取，这里简化处理
        Ok(DetailedMemoryInfo {
            memory,
            swap_total: 0,
            swap_used: 0,
            swap_free: 0,
            swap_usage_percent: 0.0,
        })
    }

    #[cfg(unix)]
    async fn get_detailed_info_unix(&self) -> Result<DetailedMemoryInfo, MonitorError> {
        let meminfo_content = fs::read_to_string("/proc/meminfo").map_err(|e| {
            MonitorError::CollectionFailed(format!("Failed to read /proc/meminfo: {}", e))
        })?;

        let mut total_kb = 0u64;
        let mut available_kb = 0u64;
        let mut free_kb = 0u64;
        let mut buffers_kb = 0u64;
        let mut cached_kb = 0u64;
        let mut swap_total_kb = 0u64;
        let mut swap_free_kb = 0u64;

        for line in meminfo_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let value = parts[1].parse::<u64>().unwrap_or(0);
                match parts[0] {
                    "MemTotal:" => total_kb = value,
                    "MemAvailable:" => available_kb = value,
                    "MemFree:" => free_kb = value,
                    "Buffers:" => buffers_kb = value,
                    "Cached:" => cached_kb = value,
                    "SwapTotal:" => swap_total_kb = value,
                    "SwapFree:" => swap_free_kb = value,
                    _ => {}
                }
            }
        }

        if total_kb == 0 {
            return Err(MonitorError::NotAvailable);
        }

        let total = total_kb * 1024;
        let available = if available_kb > 0 {
            available_kb * 1024
        } else {
            (free_kb + buffers_kb + cached_kb) * 1024
        };
        let used = total - available;

        let swap_total = swap_total_kb * 1024;
        let swap_free = swap_free_kb * 1024;
        let swap_used = swap_total - swap_free;

        Ok(DetailedMemoryInfo {
            memory: MemoryInfo::new(total, used, available),
            swap_total,
            swap_used,
            swap_free,
            swap_usage_percent: if swap_total > 0 {
                (swap_used as f64 / swap_total as f64 * 100.0) as f32
            } else {
                0.0
            },
        })
    }
}

impl Default for MemoryCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// 详细内存信息
#[derive(Debug, Clone)]
pub struct DetailedMemoryInfo {
    /// 基本内存信息
    pub memory: MemoryInfo,
    /// 交换区总大小
    pub swap_total: u64,
    /// 交换区已使用大小
    pub swap_used: u64,
    /// 交换区可用大小
    pub swap_free: u64,
    /// 交换区使用百分比
    pub swap_usage_percent: f32,
}

impl DetailedMemoryInfo {
    pub fn swap_used_formatted(&self) -> String {
        MemoryInfo::format_size(self.swap_used)
    }

    pub fn swap_total_formatted(&self) -> String {
        MemoryInfo::format_size(self.swap_total)
    }
}
