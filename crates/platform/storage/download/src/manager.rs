use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use burncloud_download_aria2::Aria2Manager;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::DownloadError;

pub(super) const MAX_RETRIES: u8 = 1;
pub(super) type DownloadWaiter = oneshot::Sender<std::result::Result<PathBuf, String>>;

/// 创建并管理 aria2 下载任务，持久化任务状态并监控下载进度。
///
/// # 示例
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let manager = burncloud_download::DownloadManager::new().await?;
/// let path = manager
///     .download("https://example.com/model.gguf", "models")
///     .await?;
/// println!("{}", path.display());
/// # Ok(())
/// # }
/// ```
///
/// # 注意
///
/// 创建管理器需要 Tokio 运行时，并会初始化数据库和 aria2 服务。
pub struct DownloadManager {
    pub(super) aria2: Arc<Aria2Manager>,
    pub(super) db: Arc<burncloud_database_sys::DownloadDB>,
    pub(super) monitor_states: Arc<Mutex<HashMap<String, MonitorState>>>,
    pub(super) start_lock: Arc<Mutex<()>>,
    pub(super) cancellation: CancellationToken,
    pub(super) monitor: Option<JoinHandle<()>>,
}

/// aria2 任务在本地管理器中的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DownloadStatus {
    Pending,
    Active,
    Waiting,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    pub(super) fn from_aria2(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "active" => Some(Self::Active),
            "waiting" => Some(Self::Waiting),
            "paused" => Some(Self::Paused),
            "complete" | "completed" => Some(Self::Completed),
            "error" | "failed" => Some(Self::Failed),
            "removed" | "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Active => "active",
            Self::Waiting => "waiting",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub(super) fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// 保存一个运行中任务及其等待者。
pub(super) struct MonitorState {
    pub(super) db_id: i64,
    pub(super) gid: String,
    pub(super) url: String,
    pub(super) dir: PathBuf,
    pub(super) filename: String,
    pub(super) status: DownloadStatus,
    pub(super) retry_count: u8,
    pub(super) max_retries: u8,
    pub(super) total_length: i64,
    pub(super) completed_length: i64,
    pub(super) download_speed: i64,
    pub(super) waiters: Vec<DownloadWaiter>,
}

impl MonitorState {
    pub(super) fn has_key(&self, url: &str, dir: &str, filename: &str) -> bool {
        self.url == url
            && self.dir.to_string_lossy() == dir
            && self.filename == filename
            && !self.status.is_terminal()
    }

    pub(super) fn update_progress(
        &mut self,
        status: DownloadStatus,
        total_length: i64,
        completed_length: i64,
        download_speed: i64,
    ) {
        self.status = status;
        self.total_length = total_length;
        self.completed_length = completed_length;
        self.download_speed = download_speed;
    }

    pub(super) fn retry_with_gid(&mut self, gid: String, retry_count: u8) {
        self.gid = gid;
        self.retry_count = retry_count;
        self.status = DownloadStatus::Pending;
    }

    pub(super) fn take_waiters(&mut self) -> Vec<DownloadWaiter> {
        std::mem::take(&mut self.waiters)
    }
}

impl Drop for DownloadManager {
    fn drop(&mut self) {
        self.cancellation.cancel();
        let aria2 = std::mem::replace(&mut self.aria2, Arc::new(Aria2Manager::new()));
        let monitor = self.monitor.take();
        match tokio::runtime::Handle::try_current() {
            Ok(runtime) => {
                runtime.spawn(shutdown_aria2(monitor, aria2, false));
            }
            Err(_) => {
                let runtime = match tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                {
                    Ok(runtime) => runtime,
                    Err(error) => {
                        if let Some(monitor) = monitor {
                            monitor.abort();
                        }
                        tracing::error!(error = %error, "创建 aria2 清理运行时失败");
                        return;
                    }
                };
                runtime.block_on(shutdown_aria2(monitor, aria2, true));
            }
        }
    }
}

async fn shutdown_aria2(
    monitor: Option<JoinHandle<()>>,
    aria2: Arc<Aria2Manager>,
    abort_monitor: bool,
) {
    wait_for_monitor(monitor, abort_monitor).await;
    shutdown_aria2_manager(aria2).await;
}

async fn wait_for_monitor(monitor: Option<JoinHandle<()>>, abort_monitor: bool) {
    if let Some(monitor) = monitor {
        if abort_monitor {
            monitor.abort();
        }
        if let Err(error) = monitor.await {
            tracing::error!(error = %error, "等待下载监控任务退出失败");
        }
    }
}

async fn shutdown_aria2_manager(aria2: Arc<Aria2Manager>) {
    match Arc::try_unwrap(aria2) {
        Ok(mut aria2) => {
            if let Err(error) = aria2.shutdown().await {
                tracing::error!(error = %error, "关闭 aria2 管理器失败");
            }
        }
        Err(_) => {
            tracing::error!("关闭 aria2 管理器失败：仍存在其他引用");
        }
    }
}

pub(super) fn invalid_record(message: impl Into<String>) -> DownloadError {
    DownloadError::InvalidRecord(message.into())
}

#[cfg(test)]
#[path = "../tests/support/manager_state.rs"]
mod manager_state_tests;
