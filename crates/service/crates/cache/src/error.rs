//! Error types for cache operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("cache connection failed")]
    Connection,

    #[error("cache backend operation failed")]
    Operation,

    #[error("cache data is invalid")]
    Serialization,

    #[error("cache is disabled")]
    Disabled,

    #[error("cache key was not found")]
    NotFound,
}

pub type CacheResult<T> = Result<T, CacheError>;
