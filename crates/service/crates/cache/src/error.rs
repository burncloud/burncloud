//! Error types for cache operations.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Redis connection error: {0}")]
    Connection(String),
    
    #[error("Redis operation error: {0}")]
    Operation(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Cache disabled")]
    Disabled,
    
    #[error("Key not found: {0}")]
    NotFound(String),
}

pub type CacheResult<T> = Result<T, CacheError>;
