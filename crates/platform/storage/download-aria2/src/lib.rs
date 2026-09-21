//! # BurnCloud Aria2 下载库
//!
//! 这是一个简单的 Rust 库，用于下载、配置和管理 aria2 下载器。

mod constants;
mod daemon;
mod downloader;
mod error;
mod manager;
mod process;
mod rpc;
mod types;

pub use daemon::Aria2Daemon;
pub use downloader::download_aria2;
pub use error::{Aria2Error, Aria2Result};
pub use manager::{quick_start, Aria2Manager};
pub use process::{
    check_port_available, find_available_port, kill_existing_aria2, start_aria2_rpc,
};
pub use rpc::Aria2RpcClient;
pub use types::{
    Aria2Config, Aria2Instance, DownloadOptions, DownloadStatus, FileInfo, GlobalStat, UriInfo,
};
