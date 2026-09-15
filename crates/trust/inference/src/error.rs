use thiserror::Error;

#[derive(Error, Debug)]
pub enum InferenceError {
    #[error("Database error: {0}")]
    Database(#[from] burncloud_database::error::DatabaseError),

    #[error("Process spawn failed: {0}")]
    ProcessSpawnFailed(String),

    #[error("Process kill failed: {0}")]
    ProcessKillFailed(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type Result<T> = std::result::Result<T, InferenceError>;
