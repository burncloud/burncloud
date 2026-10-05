//! BurnCloud 自动更新库
//!
//! 提供从 GitHub 自动更新应用程序的功能，失败时提供手动下载链接。
//!
//! # 示例
//!
//! ```rust,no_run
//! use burncloud_auto_update::{AutoUpdater, UpdateConfig, UpdateResult};
//!
//! fn main() -> UpdateResult<()> {
//!     let updater = AutoUpdater::with_default_config();
//!
//!     // 使用同步 API 避免运行时冲突
//!     if updater.sync_check_for_updates()? {
//!         updater.sync_update()?;
//!     }
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod error;
#[expect(
    clippy::cognitive_complexity,
    reason = "the synchronous updater preserves platform/release fallback branches required by the current self-update flow; keep the legacy decomposition debt local to updater.rs"
)]
pub mod updater;

pub use config::UpdateConfig;
pub use error::{UpdateError, UpdateResult};
pub use updater::{is_upgrade_over, AutoUpdater};
