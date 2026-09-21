use std::path::PathBuf;

// 常量定义
pub(crate) const DEFAULT_PORT: u16 = 6800;
pub(crate) const MAX_PORT_RANGE: u16 = 100;
pub(crate) const ARIA2_MAIN_URL: &str = "https://github.com/aria2/aria2/releases/download/release-1.37.0/aria2-1.37.0-win-64bit-build1.zip";
pub(crate) const ARIA2_BACKUP_URL: &str =
    "https://gitee.com/burncloud/aria2/raw/master/aria2-1.37.0-win-64bit-build1.zip";

/// 获取 BurnCloud 目录路径
// 输入：无。
// 功能：读取用户配置并构造 BurnCloud 本地数据目录路径。
// 错误：环境变量读取失败时使用默认目录，不返回错误。
pub(crate) fn get_burncloud_dir() -> PathBuf {
    // 读取用户配置并构造目录路径
    std::env::var("USERPROFILE")
        .map(|profile| {
            PathBuf::from(profile)
                .join("AppData")
                .join("Local")
                .join("BurnCloud")
        })
        .unwrap_or_else(|_| PathBuf::from(r"C:\Users\Default\AppData\Local\BurnCloud"))
}
