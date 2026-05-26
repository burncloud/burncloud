//! Cache error types

use thiserror::Error;

/// Cache operation errors
#[derive(Debug, Error)]
pub enum CacheError {
    /// Redis connection error
    #[error("Redis connection error: {0}")]
    ConnectionError(String),

    /// Redis operation error
    #[error("Redis operation error: {0}")]
    OperationError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Cache miss (not an error, used for control flow)
    #[error("Cache miss for key: {0}")]
    CacheMiss(String),

    /// Database fallback error
    #[error("Database error: {0}")]
    DatabaseError(#[from] burncloud_database::DatabaseError),

    /// Invalid configuration
    #[error("Invalid cache configuration: {0}")]
    ConfigError(String),
}

/// Type alias for Result with CacheError
pub type CacheResult<T> = std::result::Result<T, CacheError>;
