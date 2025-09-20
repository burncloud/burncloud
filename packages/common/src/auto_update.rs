//! 自动更新模块
//!
//! 提供从 GitHub 自动更新应用程序的功能，失败时提供手动下载链接。

use anyhow::{anyhow, Result};
use log::{info, error};
use self_update::backends::github;
use std::env;

/// 自动更新配置
#[derive(Debug, Clone)]
pub struct UpdateConfig {
    /// GitHub 仓库所有者
    pub github_owner: String,
    /// GitHub 仓库名称
    pub github_repo: String,
    /// 二进制文件名
    pub bin_name: String,
    /// 当前版本
    pub current_version: String,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            github_owner: "burncloud".to_string(),
            github_repo: "burncloud".to_string(),
            bin_name: "burncloud".to_string(),
            current_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// 自动更新器
pub struct AutoUpdater {
    config: UpdateConfig,
}

impl AutoUpdater {
    /// 创建新的自动更新器
    pub fn new(config: UpdateConfig) -> Self {
        Self { config }
    }

    /// 使用默认配置创建自动更新器
    pub fn with_default_config() -> Self {
        Self::new(UpdateConfig::default())
    }

    /// 检查是否有可用更新
    pub async fn check_for_updates(&self) -> Result<bool> {
        info!("检查更新中...");

        match self.check_github_updates().await {
            Ok(has_update) => return Ok(has_update),
            Err(e) => {
                error!("GitHub 检查更新失败: {}", e);
                return Err(anyhow!("检查更新失败: {}", e));
            }
        }
    }

    /// 执行更新
    pub async fn update_with_fallback(&self) -> Result<()> {
        info!("开始更新应用程序...");

        match self.update_from_github().await {
            Ok(_) => {
                info!("从 GitHub 更新成功");
                return Ok(());
            }
            Err(e) => {
                error!("GitHub 更新失败: {}", e);
                return Err(anyhow!("更新失败: {}", e));
            }
        }
    }

    /// 从 GitHub 检查更新
    async fn check_github_updates(&self) -> Result<bool> {
        info!("正在检查 GitHub 更新...");

        let update = github::Update::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .bin_name(&self.config.bin_name)
            .current_version(&self.config.current_version)
            .build()?;

        let latest_release = update.get_latest_release()?;
        let current_version = &self.config.current_version;

        info!("当前版本: {}, 最新版本: {}", current_version, latest_release.version);

        Ok(latest_release.version != *current_version)
    }

    /// 从 GitHub 更新
    async fn update_from_github(&self) -> Result<()> {
        info!("正在从 GitHub 下载更新...");

        let update = github::Update::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .bin_name(&self.config.bin_name)
            .current_version(&self.config.current_version)
            .build()?;

        let status = update.update()?;

        match status.updated() {
            true => {
                info!("更新成功，新版本: {}", status.version());
                Ok(())
            }
            false => {
                info!("已是最新版本");
                Ok(())
            }
        }
    }

    /// 获取当前版本
    pub fn current_version(&self) -> &str {
        &self.config.current_version
    }

    /// 设置新的配置
    pub fn set_config(&mut self, config: UpdateConfig) {
        self.config = config;
    }

    /// 获取手动下载链接
    pub fn get_download_links(&self) -> (String, String) {
        let github_url = format!(
            "https://github.com/{}/{}/releases",
            self.config.github_owner,
            self.config.github_repo
        );
        let gitee_url = format!(
            "https://gitee.com/{}/{}/releases",
            self.config.github_owner,  // 使用相同的组织名
            self.config.github_repo
        );
        (github_url, gitee_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = UpdateConfig::default();
        assert_eq!(config.github_owner, "burncloud");
        assert_eq!(config.github_repo, "burncloud");
        assert_eq!(config.bin_name, "burncloud");
    }

    #[test]
    fn test_auto_updater_creation() {
        let updater = AutoUpdater::with_default_config();
        assert_eq!(updater.current_version(), env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_get_download_links() {
        let updater = AutoUpdater::with_default_config();
        let (github_url, gitee_url) = updater.get_download_links();

        assert_eq!(github_url, "https://github.com/burncloud/burncloud/releases");
        assert_eq!(gitee_url, "https://gitee.com/burncloud/burncloud/releases");
    }

    #[tokio::test]
    async fn test_update_config_customization() {
        let mut config = UpdateConfig::default();
        config.current_version = "1.0.0".to_string();

        let mut updater = AutoUpdater::new(config);
        assert_eq!(updater.current_version(), "1.0.0");

        let new_config = UpdateConfig {
            current_version: "2.0.0".to_string(),
            ..UpdateConfig::default()
        };

        updater.set_config(new_config);
        assert_eq!(updater.current_version(), "2.0.0");
    }
}