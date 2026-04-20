//! Database operations for sys_ domain (settings, installations, downloads).
//!
//! This crate aggregates all system-level database operations:
//! - [`setting`] - System settings key/value store (SysSetting, SettingDatabase)
//! - [`installer`] - Software installation records (SysInstallation, InstallerDB)
//! - [`download`] - Download tasks (SysDownload, DownloadDB)

pub mod setting;
pub mod installer;
pub mod download;

// Re-export primary types for convenience
pub use setting::{SettingDatabase, SysSetting};
pub use installer::{InstallerDB, SysInstallation};
pub use download::{DownloadDB, SysDownload};

// Re-export shared DatabaseError for consumers
pub use burncloud_database::DatabaseError;
