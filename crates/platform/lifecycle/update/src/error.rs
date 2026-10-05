//! 自动更新错误类型

use std::fmt;

/// 自动更新错误类型
#[derive(Debug)]
pub enum UpdateError {
    /// 网络错误
    Network(String),
    /// GitHub API 错误
    GitHub(String),
    /// 版本解析错误
    Version(String),
    /// 文件系统错误
    FileSystem(String),
    /// 权限错误
    Permission(String),
    /// 配置错误
    Configuration(String),
    /// 其他错误
    Other(String),
    /// 未知错误
    Unknown(String),
}

impl fmt::Display for UpdateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateError::Network(msg) => write!(f, "网络错误: {}", msg),
            UpdateError::GitHub(msg) => write!(f, "GitHub API 错误: {}", msg),
            UpdateError::Version(msg) => write!(f, "版本解析错误: {}", msg),
            UpdateError::FileSystem(msg) => write!(f, "文件系统错误: {}", msg),
            UpdateError::Permission(msg) => write!(f, "权限错误: {}", msg),
            UpdateError::Configuration(msg) => write!(f, "配置错误: {}", msg),
            UpdateError::Other(msg) => write!(f, "其他错误: {}", msg),
            UpdateError::Unknown(msg) => write!(f, "未知错误: {}", msg),
        }
    }
}

impl std::error::Error for UpdateError {}

impl From<anyhow::Error> for UpdateError {
    fn from(error: anyhow::Error) -> Self {
        UpdateError::Unknown(error.to_string())
    }
}

impl From<self_update::errors::Error> for UpdateError {
    fn from(error: self_update::errors::Error) -> Self {
        let message = error.to_string();
        match &error {
            // self_update 1.x replaced the old Network variant with a transport
            // error for requests that never completed.
            self_update::errors::Error::Transport(_) => UpdateError::Network(message),
            // Completed HTTP failures and release-list/asset failures are remote
            // GitHub/update-source failures from BurnCloud's point of view.
            self_update::errors::Error::NotFound { .. }
            | self_update::errors::Error::Unauthorized { .. }
            | self_update::errors::Error::HttpStatus { .. }
            | self_update::errors::Error::RateLimited { .. }
            | self_update::errors::Error::NoReleaseFound { .. }
            | self_update::errors::Error::MissingAssetField { .. }
            | self_update::errors::Error::InvalidResponse { .. } => UpdateError::GitHub(message),
            // Error is #[non_exhaustive] in self_update 1.x, so new categories
            // must remain safely representable without breaking BurnCloud.
            _ => UpdateError::Unknown(message),
        }
    }
}

/// 自动更新结果类型
pub type UpdateResult<T> = Result<T, UpdateError>;
