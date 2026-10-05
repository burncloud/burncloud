use crate::types::{DiskInfo, MonitorError};

#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;
#[cfg(windows)]
use std::ptr;
#[cfg(windows)]
use winapi::um::fileapi::GetDiskFreeSpaceExW;

#[cfg(unix)]
use std::fs;

/// 磁盘数据收集器
pub struct DiskCollector;

impl DiskCollector {
    /// 创建新的磁盘收集器
    pub fn new() -> Self {
        Self
    }

    /// 收集所有磁盘信息
    pub async fn collect_all(&self) -> Result<Vec<DiskInfo>, MonitorError> {
        #[cfg(windows)]
        {
            self.collect_all_windows().await
        }
        #[cfg(unix)]
        {
            self.collect_all_unix().await
        }
    }

    #[cfg(windows)]
    async fn collect_all_windows(&self) -> Result<Vec<DiskInfo>, MonitorError> {
        let mut disks = Vec::new();

        // 简化版本：只检查C盘
        if let Ok(disk_info) = self.get_disk_info_windows("C:\\").await {
            disks.push(disk_info);
        }

        Ok(disks)
    }

    #[cfg(windows)]
    async fn get_disk_info_windows(&self, path: &str) -> Result<DiskInfo, MonitorError> {
        // SAFETY: `path_wide` is NUL-terminated and all output pointers refer to valid local u64 storage.
        // GetDiskFreeSpaceExW writes only for the duration of the call and no pointer escapes this block.
        unsafe {
            let path_wide: Vec<u16> = OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();

            let mut free_bytes: u64 = 0;
            let mut total_bytes: u64 = 0;

            if GetDiskFreeSpaceExW(
                path_wide.as_ptr(),
                &mut free_bytes as *mut u64 as *mut _,
                &mut total_bytes as *mut u64 as *mut _,
                ptr::null_mut(),
            ) == 0
            {
                return Err(MonitorError::CollectionFailed(format!(
                    "Failed to get disk space for {}",
                    path
                )));
            }

            // The same guard as the Unix branch: `free_bytes` comes from the platform and is not guaranteed to
            // be at most `total_bytes`, so the subtraction saturates rather than wrapping.
            let used = total_bytes.saturating_sub(free_bytes);
            let usage_percent = super::usage_percent(used, total_bytes);

            Ok(DiskInfo {
                total: total_bytes,
                used,
                available: free_bytes,
                usage_percent,
                mount_point: path.to_string(),
            })
        }
    }

    #[cfg(unix)]
    async fn collect_all_unix(&self) -> Result<Vec<DiskInfo>, MonitorError> {
        let mounts_content = fs::read_to_string("/proc/mounts").map_err(|e| {
            MonitorError::CollectionFailed(format!("Failed to read /proc/mounts: {}", e))
        })?;

        let mut disks = Vec::new();

        for line in mounts_content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let mount_point = parts[1];
                let fs_type = parts[2];

                // 只处理本地文件系统
                if self.is_local_filesystem(fs_type) && self.is_valid_mount_point(mount_point) {
                    if let Ok(disk_info) = self.get_disk_info_unix(mount_point).await {
                        disks.push(disk_info);
                    }
                }
            }
        }

        Ok(disks)
    }

    #[cfg(unix)]
    async fn get_disk_info_unix(&self, mount_point: &str) -> Result<DiskInfo, MonitorError> {
        use std::ffi::CString;
        use std::mem;

        let path = CString::new(mount_point)
            .map_err(|e| MonitorError::InvalidData(format!("Invalid path: {}", e)))?;

        // SAFETY: `path` is a live NUL-terminated CString and `statvfs` is valid writable storage for libc.
        unsafe {
            let mut statvfs: libc::statvfs = mem::zeroed();
            if libc::statvfs(path.as_ptr(), &mut statvfs) != 0 {
                return Err(MonitorError::CollectionFailed(format!(
                    "Failed to get filesystem stats for {}",
                    mount_point
                )));
            }

            // libc exposes these fields as different unsigned widths across
            // Unix targets (u64 on Linux, u32 for block counts on macOS ARM).
            // Normalize before doing byte arithmetic and building DiskInfo.
            let block_size = u64::from(statvfs.f_frsize);
            let total_blocks = u64::from(statvfs.f_blocks);
            let free_blocks = u64::from(statvfs.f_bavail);

            let total = total_blocks * block_size;
            let available = free_blocks * block_size;
            // `saturating_sub`, because `f_bavail > f_blocks` is possible: `f_bavail` counts blocks available
            // to an unprivileged process, and a filesystem with reserved blocks or a container overlay can
            // report more available than total. The plain subtraction underflowed to a value near `u64::MAX`,
            // which then produced a usage percentage far above 100 and a nonsensical "used" figure.
            let used = total.saturating_sub(available);

            let usage_percent = super::usage_percent(used, total);

            Ok(DiskInfo {
                total,
                used,
                available,
                usage_percent,
                mount_point: mount_point.to_string(),
            })
        }
    }

    #[cfg(unix)]
    fn is_local_filesystem(&self, fs_type: &str) -> bool {
        matches!(
            fs_type,
            "ext2"
                | "ext3"
                | "ext4"
                | "xfs"
                | "btrfs"
                | "zfs"
                | "reiserfs"
                | "jfs"
                | "ntfs"
                | "vfat"
                | "exfat"
                | "hfs"
                | "apfs"
        )
    }

    #[cfg(unix)]
    fn is_valid_mount_point(&self, mount_point: &str) -> bool {
        // 排除特殊的挂载点
        !mount_point.starts_with("/proc")
            && !mount_point.starts_with("/sys")
            && !mount_point.starts_with("/dev")
            && !mount_point.starts_with("/run")
            && mount_point != "/tmp"
            && mount_point != "/var/tmp"
    }

    /// 收集系统主磁盘信息 (通常是根目录所在磁盘)
    pub async fn collect_main_disk(&self) -> Result<Option<DiskInfo>, MonitorError> {
        let all_disks = self.collect_all().await?;

        // 查找根目录磁盘或第一个磁盘
        let main_disk = all_disks
            .iter()
            .find(|disk| {
                let mount_point = disk.mount_point.as_str();
                mount_point == "/" || mount_point.starts_with("C:")
            })
            .cloned()
            .or_else(|| all_disks.into_iter().next());

        Ok(main_disk)
    }
}

impl Default for DiskCollector {
    fn default() -> Self {
        Self::new()
    }
}
