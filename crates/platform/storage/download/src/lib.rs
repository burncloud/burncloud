//! 基于 aria2 与 SQLite 持久化的下载管理。

mod error;
mod manager;
mod utils;

pub use error::DownloadError;
pub use manager::DownloadManager;
