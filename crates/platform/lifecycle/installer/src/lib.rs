//! BurnCloud Third-Party Software Installer
//!
//! This crate provides functionality to install third-party AI software
//! through the BurnCloud CLI.
//!
//! # Supported Software
//!
//! - **OpenClaw**: Open source personal AI assistant (https://openclaw.ai)
//! - **Cherry Studio**: AI productivity tool (https://github.com/CherryHQ/cherry-studio)
//!
//! # Example
//!
//! ```rust,no_run
//! use burncloud_installer::{Installer, InstallerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = InstallerConfig::new()
//!         .with_auto_deps(true);
//!
//!     let installer = Installer::new(config);
//!
//!     // List available software
//!     for software in installer.list_available() {
//!         println!("- {} ({})", software.name, software.id);
//!     }
//!
//!     // Install software
//!     installer.install("openclaw").await?;
//!
//!     Ok(())
//! }
//! ```

#[expect(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    reason = "bundle creation walks platform-specific dependency graphs and archive layouts in legacy routines; keep the existing maintainability debt scoped to this module"
)]
pub mod bundle;
pub mod error;
#[expect(
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::let_underscore_must_use,
    reason = "the legacy installer coordinates many platform/package-manager branches and contains one documented best-effort launcher status; correctness refactors must be split by install method rather than hidden globally"
)]
pub mod installer;
pub mod npm;
pub mod platform;
pub mod registry;
pub mod software;

pub use bundle::{BundleCreator, BundleManifest, BundleVerifier};
pub use error::{InstallerError, InstallerResult};
pub use installer::{Installer, InstallerConfig};
pub use npm::NpmInstaller;
pub use platform::{Arch, Platform, OS};
pub use registry::{get_software, is_valid_software, list_software};
pub use software::{
    Dependency, GitHubAsset, GitHubRelease, InstallMethod, InstallStatus, ShellType, Software,
};
