#[derive(Debug, thiserror::Error)]
pub enum BurnCloudError {
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Download failed: {0}")]
    DownloadFailed(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Password hash error: {0}")]
    PasswordHashError(String),
}

pub type Result<T> = std::result::Result<T, BurnCloudError>;
