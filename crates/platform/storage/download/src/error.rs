use burncloud_database::DatabaseError;
use burncloud_download_aria2::Aria2Error;
use thiserror::Error;

/// 下载管理和任务执行错误。
#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("数据库错误：{0}")]
    Database(#[from] DatabaseError),
    #[error("aria2 错误：{0}")]
    Aria2(#[from] Aria2Error),
    #[error("无效的下载 URL：{0}")]
    InvalidUrl(String),
    #[error("无效的下载目录：{0}")]
    InvalidDirectory(String),
    #[error("无法从 URL 提取有效文件名")]
    InvalidFilename,
    #[error("下载任务失败：{0}")]
    Failed(String),
    #[error("下载任务通知通道已关闭")]
    NotificationClosed,
    #[error("操作失败：{operation}；清理任务也失败：{cleanup}")]
    Compensation { operation: String, cleanup: String },
    #[error("恢复数据库任务失败：{0}")]
    InvalidRecord(String),
}

pub(super) type Result<T> = std::result::Result<T, DownloadError>;
