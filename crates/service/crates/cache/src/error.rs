//! Error types for cache operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("cache connection failed")]
    Connection,

    #[error("cache backend operation failed")]
    Operation,

    #[error("cache is disabled")]
    Disabled,

}

pub type CacheResult<T> = Result<T, CacheError>;
