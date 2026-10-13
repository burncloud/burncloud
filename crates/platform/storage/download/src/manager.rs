use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use burncloud_download_aria2::Aria2Manager;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use crate::DownloadError;

pub(crate) const MAX_RETRIES: u8 = 1;
pub(crate) type DownloadWaiter = oneshot::Sender<std::result::Result<PathBuf, String>>;

/// 统一创建、恢复并监控下载任务。
pub struct DownloadManager {
    pub(crate) aria2: Arc<Aria2Manager>,
    pub(crate) db: Arc<burncloud_database_sys::DownloadDB>,
    pub(crate) monitor_states: Arc<Mutex<HashMap<String, MonitorState>>>,
    pub(crate) cancellation: CancellationToken,
    pub(crate) monitor: Option<JoinHandle<()>>,
}

/// aria2 任务在本地管理器中的状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DownloadStatus {
    Pending,
    Active,
    Waiting,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl DownloadStatus {
    pub(crate) fn from_aria2(value: &str) -> Option<Self> {
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

    pub(crate) fn as_str(self) -> &'static str {
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

    pub(crate) fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// 保存一个运行中任务及其等待者。
pub(crate) struct MonitorState {
    pub(crate) db_id: i64,
    pub(crate) gid: String,
    pub(crate) url: String,
    pub(crate) dir: PathBuf,
    pub(crate) filename: String,
    pub(crate) status: DownloadStatus,
    pub(crate) retry_count: u8,
    pub(crate) max_retries: u8,
    pub(crate) total_length: i64,
    pub(crate) completed_length: i64,
    pub(crate) download_speed: i64,
    pub(crate) waiters: Vec<DownloadWaiter>,
}

impl MonitorState {
    pub(crate) fn has_key(&self, url: &str, dir: &str, filename: &str) -> bool {
        self.url == url
            && self.dir.to_string_lossy() == dir
            && self.filename == filename
            && !self.status.is_terminal()
    }

    pub(crate) fn update_progress(
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

    pub(crate) fn retry_with_gid(&mut self, gid: String, retry_count: u8) {
        self.gid = gid;
        self.retry_count = retry_count;
        self.status = DownloadStatus::Pending;
    }

    pub(crate) fn take_waiters(&mut self) -> Vec<DownloadWaiter> {
        std::mem::take(&mut self.waiters)
    }
}

impl Drop for DownloadManager {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(monitor) = self.monitor.take() {
            match tokio::runtime::Handle::try_current() {
                Ok(runtime) => {
                    runtime.spawn(async move {
                        if let Err(error) = monitor.await {
                            tracing::error!(error = %error, "等待下载监控任务退出失败");
                        }
                    });
                }
                Err(_) => monitor.abort(),
            }
        }
    }
}

pub(crate) fn invalid_record(message: impl Into<String>) -> DownloadError {
    DownloadError::InvalidRecord(message.into())
}
