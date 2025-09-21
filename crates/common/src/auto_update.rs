//! 自动更新模块
//!
//! 重新导出 burncloud-auto-update crate 的功能，提供统一的接口。

pub use burncloud_auto_update::{
    AutoUpdater,
    UpdateConfig,
    UpdateError,
    UpdateResult,
};