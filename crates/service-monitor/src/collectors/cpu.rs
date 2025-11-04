use crate::types::{CpuInfo, MonitorError};
use tokio::time::{Duration, Instant};

#[cfg(unix)]
use std::fs;

/// CPU数据收集器
pub struct CpuCollector {
    last_update: Instant,
    update_interval: Duration,
    #[cfg(unix)]
    last_cpu_times: Option<CpuTimes>,
}

#[cfg(unix)]
#[derive(Debug, Clone)]
struct CpuTimes {
    idle: u64,
    total: u64,
}

impl CpuCollector {
    /// 创建新的CPU收集器
    pub fn new() -> Self {
        Self {
            last_update: Instant::now(),
            update_interval: Duration::from_millis(500),
            #[cfg(unix)]
            last_cpu_times: None,
        }
    }

    /// 设置更新间隔
    pub fn with_update_interval(mut self, interval: Duration) -> Self {
        self.update_interval = interval;
        self
    }

    /// 收集CPU信息
    pub async fn collect(&mut self) -> Result<CpuInfo, MonitorError> {
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
    async fn collect_windows(&mut self) -> Result<CpuInfo, MonitorError> {
        use winapi::um::sysinfoapi::{GetSystemInfo, SYSTEM_INFO};
        use winapi::um::winreg::*;
        use winapi::shared::minwindef::HKEY;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;

        // 获取CPU核心数
        let mut sys_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
        unsafe {
            GetSystemInfo(&mut sys_info);
        }
        let core_count = sys_info.dwNumberOfProcessors as usize;

        // 从注册表读取CPU信息
        let (frequency, brand) = self.read_cpu_info_from_registry();

        // 简化版本：返回固定的CPU使用率
        let cpu_usage: f32 = 25.0;

        Ok(CpuInfo {
            usage_percent: cpu_usage.max(0.0).min(100.0),
            core_count,
            frequency,
            brand,
        })
    }

    #[cfg(windows)]
    fn read_cpu_info_from_registry(&self) -> (u64, String) {
        use winapi::um::winreg::*;
        use winapi::shared::minwindef::{HKEY, DWORD};
        use winapi::shared::winerror::ERROR_SUCCESS;
        use std::ffi::OsStr;
        use std::os::windows::ffi::OsStrExt;
        use std::ptr;
        use std::mem;

        let mut frequency = 0u64;
        let mut brand = String::from("Windows CPU");

        unsafe {
            let mut hkey: HKEY = ptr::null_mut();
            let subkey: Vec<u16> = OsStr::new("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0")
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            if RegOpenKeyExW(
                winapi::um::winreg::HKEY_LOCAL_MACHINE,
                subkey.as_ptr(),
                0,
                winapi::um::winnt::KEY_READ,
                &mut hkey,
            ) == ERROR_SUCCESS as i32 {

                // 读取处理器名称
                let mut buffer = [0u16; 256];
                let mut buffer_size = (buffer.len() * 2) as DWORD;
                let value_name: Vec<u16> = OsStr::new("ProcessorNameString")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                if RegQueryValueExW(
                    hkey,
                    value_name.as_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    buffer.as_mut_ptr() as *mut u8,
                    &mut buffer_size,
                ) == ERROR_SUCCESS as i32 {
                    let len = (buffer_size as usize / 2).min(buffer.len());
                    if len > 0 {
                        let processor_name = String::from_utf16_lossy(&buffer[..len-1]);
                        brand = processor_name.trim().to_string();
                    }
                }

                // 读取频率（MHz）
                let mut freq_mhz: DWORD = 0;
                let mut freq_size = mem::size_of::<DWORD>() as DWORD;
                let freq_name: Vec<u16> = OsStr::new("~MHz")
                    .encode_wide()
                    .chain(std::iter::once(0))
                    .collect();

                if RegQueryValueExW(
                    hkey,
                    freq_name.as_ptr(),
                    ptr::null_mut(),
                    ptr::null_mut(),
                    &mut freq_mhz as *mut DWORD as *mut u8,
                    &mut freq_size,
                ) == ERROR_SUCCESS as i32 {
                    frequency = freq_mhz as u64;
                }

                RegCloseKey(hkey);
            }
        }

        (frequency, brand)
    }

    #[cfg(unix)]
    async fn collect_unix(&mut self) -> Result<CpuInfo, MonitorError> {
        let stat_content = fs::read_to_string("/proc/stat")
            .map_err(|e| MonitorError::CollectionFailed(format!("Failed to read /proc/stat: {}", e)))?;

        let cpu_line = stat_content
            .lines()
            .next()
            .ok_or_else(|| MonitorError::NotAvailable)?;

        let parts: Vec<&str> = cpu_line.split_whitespace().collect();
        if parts.len() < 8 || parts[0] != "cpu" {
            return Err(MonitorError::InvalidData("Invalid /proc/stat format".to_string()));
        }

        let user: u64 = parts[1].parse().unwrap_or(0);
        let nice: u64 = parts[2].parse().unwrap_or(0);
        let system: u64 = parts[3].parse().unwrap_or(0);
        let idle: u64 = parts[4].parse().unwrap_or(0);
        let iowait: u64 = parts[5].parse().unwrap_or(0);
        let irq: u64 = parts[6].parse().unwrap_or(0);
        let softirq: u64 = parts[7].parse().unwrap_or(0);

        let total = user + nice + system + idle + iowait + irq + softirq;
        let current_times = CpuTimes { idle, total };

        let usage_percent = if let Some(ref last_times) = self.last_cpu_times {
            let total_diff = current_times.total - last_times.total;
            let idle_diff = current_times.idle - last_times.idle;

            if total_diff > 0 {
                100.0 - (idle_diff as f64 / total_diff as f64 * 100.0) as f32
            } else {
                0.0
            }
        } else {
            0.0
        };

        self.last_cpu_times = Some(current_times);

        // 获取CPU核心数
        let core_count = fs::read_to_string("/proc/cpuinfo")
            .map_err(|e| MonitorError::CollectionFailed(format!("Failed to read /proc/cpuinfo: {}", e)))?
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count();

        // 获取CPU品牌
        let brand = fs::read_to_string("/proc/cpuinfo")
            .unwrap_or_default()
            .lines()
            .find(|line| line.starts_with("model name"))
            .and_then(|line| line.split(':').nth(1))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "Unknown CPU".to_string());

        Ok(CpuInfo {
            usage_percent: usage_percent.max(0.0).min(100.0),
            core_count,
            frequency: 0,
            brand,
        })
    }

    /// 获取CPU核心详细信息
    pub fn get_cpu_cores_info(&self) -> Result<Vec<(String, f32)>, MonitorError> {
        // 简化实现
        Ok(vec![("Total".to_string(), 0.0)])
    }
}

impl Default for CpuCollector {
    fn default() -> Self {
        Self::new()
    }
}