//! Installer error types

use std::fmt;

/// Installer error types
#[derive(Debug)]
pub enum InstallerError {
    /// Network error
    Network(String),
    /// Script execution error
    Script(String),
    /// GitHub API error
    GitHub(String),
    /// File system error
    FileSystem(String),
    /// Permission error
    Permission(String),
    /// Dependency not found
    DependencyNotFound(String),
    /// Software not found
    SoftwareNotFound(String),
    /// Installation failed
    InstallationFailed(String),
    /// Platform not supported
    PlatformNotSupported(String),
    /// Download error
    Download(String),
    /// Configuration error
    Configuration(String),
    /// Database error
    Database(String),
    /// Other error
    Other(String),
}

impl fmt::Display for InstallerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstallerError::Network(msg) => write!(f, "Network error: {}", msg),
            InstallerError::Script(msg) => write!(f, "Script execution error: {}", msg),
            InstallerError::GitHub(msg) => write!(f, "GitHub API error: {}", msg),
            InstallerError::FileSystem(msg) => write!(f, "File system error: {}", msg),
            InstallerError::Permission(msg) => write!(f, "Permission error: {}", msg),
            InstallerError::DependencyNotFound(msg) => write!(f, "Dependency not found: {}", msg),
            InstallerError::SoftwareNotFound(msg) => write!(f, "Software not found: {}", msg),
            InstallerError::InstallationFailed(msg) => write!(f, "Installation failed: {}", msg),
            InstallerError::PlatformNotSupported(msg) => {
                write!(f, "Platform not supported: {}", msg)
            }
            InstallerError::Download(msg) => write!(f, "Download error: {}", msg),
            InstallerError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            InstallerError::Database(msg) => write!(f, "Database error: {}", msg),
            InstallerError::Other(msg) => write!(f, "Other error: {}", msg),
        }
    }
}

impl std::error::Error for InstallerError {}

impl From<anyhow::Error> for InstallerError {
    fn from(error: anyhow::Error) -> Self {
        InstallerError::Other(error.to_string())
    }
}

impl From<reqwest::Error> for InstallerError {
    fn from(error: reqwest::Error) -> Self {
        InstallerError::Network(error.to_string())
    }
}

impl From<std::io::Error> for InstallerError {
    fn from(error: std::io::Error) -> Self {
        InstallerError::FileSystem(error.to_string())
    }
}

impl From<serde_json::Error> for InstallerError {
    fn from(error: serde_json::Error) -> Self {
        InstallerError::Configuration(format!("JSON error: {}", error))
    }
}

impl From<std::path::StripPrefixError> for InstallerError {
    fn from(error: std::path::StripPrefixError) -> Self {
        InstallerError::FileSystem(format!("Path error: {}", error))
    }
}

/// Installer result type
pub type InstallerResult<T> = Result<T, InstallerError>;
