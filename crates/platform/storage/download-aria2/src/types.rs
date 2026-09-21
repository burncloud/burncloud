use std::path::PathBuf;
use std::process::Child;

use serde::{Deserialize, Serialize};

use crate::constants::{get_burncloud_dir, DEFAULT_PORT};
use crate::error::{Aria2Error, Aria2Result};

// ============================================================================
// 数据结构定义
// ============================================================================

#[derive(Debug, Clone)]
pub struct Aria2Config {
    pub port: u16,
    pub secret: Option<String>,
    pub download_dir: PathBuf,
    pub max_connections: u8,
    pub split_size: String,
    pub aria2_path: PathBuf,
}

impl Default for Aria2Config {
    // 输入：无。
    // 功能：创建带默认端口、下载目录和 aria2 路径的配置。
    // 错误：无，当前工作目录读取失败时使用空路径。
    fn default() -> Self {
        // 组装默认配置字段
        Self {
            port: DEFAULT_PORT,
            secret: None,
            download_dir: std::env::current_dir()
                .unwrap_or_default()
                .join("downloads"),
            max_connections: 16,
            split_size: "1M".to_string(),
            aria2_path: get_burncloud_dir().join("aria2c.exe"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<u8>,
    #[serde(
        rename = "max-connection-per-server",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_connection_per_server: Option<u8>,
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_download: Option<bool>,
    #[serde(rename = "allow-overwrite", skip_serializing_if = "Option::is_none")]
    pub allow_overwrite: Option<bool>,
    #[serde(rename = "auto-file-renaming", skip_serializing_if = "Option::is_none")]
    pub auto_file_renaming: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DownloadStatus {
    pub gid: String,
    pub status: String,
    #[serde(rename = "totalLength")]
    pub total_length: String,
    #[serde(rename = "completedLength")]
    pub completed_length: String,
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GlobalStat {
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
    #[serde(rename = "numActive")]
    pub num_active: String,
    #[serde(rename = "numWaiting")]
    pub num_waiting: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub uris: Vec<UriInfo>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UriInfo {
    pub uri: String,
    pub status: String,
}

pub struct Aria2Instance {
    pub process: Child,
    pub port: u16,
    pub config: Aria2Config,
}

impl Aria2Instance {
    // 输入：可变的 aria2 进程实例。
    // 功能：探测关联的 aria2 子进程是否仍在运行。
    // 错误：进程状态查询失败时返回 false。
    pub fn is_running(&mut self) -> bool {
        // 查询子进程的退出状态
        matches!(self.process.try_wait(), Ok(None))
    }

    // 输入：可变的 aria2 进程实例。
    // 功能：终止并等待关联的 aria2 子进程退出。
    // 错误：进程终止或等待失败时返回 ProcessError。
    pub fn kill(&mut self) -> Aria2Result<()> {
        // 请求终止子进程
        self.process
            .kill()
            .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;
        // 等待子进程完成退出
        self.process
            .wait()
            .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;
        Ok(())
    }
}
