//! Database operations for sys_ domain (settings, installations, downloads).
//!
//! This crate aggregates all system-level database operations:
//! - [`setting`] - System settings key/value store (SysSetting, SettingDatabase)
//! - [`installer`] - Software installation records (InstallerDB)
//! - [`download`] - Download tasks (SysDownload, DownloadDB)

pub mod download;
pub mod installer;
pub mod setting;

// Re-export primary types for convenience
pub use download::{DownloadDB, SysDownload};
pub use installer::InstallerDB;
pub use setting::{SettingDatabase, SysSetting};

// Re-export shared DatabaseError for consumers
pub use burncloud_database::DatabaseError;
