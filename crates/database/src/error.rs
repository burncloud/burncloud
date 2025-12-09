use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Query error: {0}")]
    Query(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Database not initialized")]
    NotInitialized,

    #[error("Failed to resolve default database path: {0}")]
    PathResolution(String),

    #[error("Failed to create database directory: {0}")]
    DirectoryCreation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid data: {message}")]
    InvalidData { message: String },
}

pub type Result<T> = std::result::Result<T, DatabaseError>;
