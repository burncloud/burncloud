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
        match error {
            self_update::errors::Error::Network(_) => {
                UpdateError::Network(error.to_string())
            }
            self_update::errors::Error::Release(_) => {
                UpdateError::GitHub(error.to_string())
            }
            _ => UpdateError::Unknown(error.to_string()),
        }
    }
}

/// 自动更新结果类型
pub type UpdateResult<T> = Result<T, UpdateError>;