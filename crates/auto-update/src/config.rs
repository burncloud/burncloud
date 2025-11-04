//! 自动更新配置模块

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

impl UpdateConfig {
    /// 创建新的配置
    pub fn new(
        github_owner: impl Into<String>,
        github_repo: impl Into<String>,
        bin_name: impl Into<String>,
        current_version: impl Into<String>,
    ) -> Self {
        Self {
            github_owner: github_owner.into(),
            github_repo: github_repo.into(),
            bin_name: bin_name.into(),
            current_version: current_version.into(),
        }
    }

    /// 设置 GitHub 仓库信息
    pub fn with_github_repo(mut self, owner: impl Into<String>, repo: impl Into<String>) -> Self {
        self.github_owner = owner.into();
        self.github_repo = repo.into();
        self
    }

    /// 设置二进制文件名
    pub fn with_bin_name(mut self, name: impl Into<String>) -> Self {
        self.bin_name = name.into();
        self
    }

    /// 设置当前版本
    pub fn with_current_version(mut self, version: impl Into<String>) -> Self {
        self.current_version = version.into();
        self
    }

    /// 获取 GitHub 下载链接
    pub fn github_releases_url(&self) -> String {
        format!(
            "https://github.com/{}/{}/releases",
            self.github_owner, self.github_repo
        )
    }

    /// 获取 Gitee 下载链接
    pub fn gitee_releases_url(&self) -> String {
        format!(
            "https://gitee.com/{}/{}/releases",
            self.github_owner, self.github_repo
        )
    }

    /// 获取下载链接元组 (GitHub, Gitee)
    pub fn download_links(&self) -> (String, String) {
        (self.github_releases_url(), self.gitee_releases_url())
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
        assert_eq!(config.current_version, env!("CARGO_PKG_VERSION"));
    }

    #[test]
    fn test_config_builder() {
        let config = UpdateConfig::new("owner", "repo", "app", "1.0.0")
            .with_github_repo("new_owner", "new_repo")
            .with_bin_name("new_app")
            .with_current_version("2.0.0");

        assert_eq!(config.github_owner, "new_owner");
        assert_eq!(config.github_repo, "new_repo");
        assert_eq!(config.bin_name, "new_app");
        assert_eq!(config.current_version, "2.0.0");
    }

    #[test]
    fn test_download_links() {
        let config = UpdateConfig::default();
        let (github, gitee) = config.download_links();

        assert_eq!(github, "https://github.com/burncloud/burncloud/releases");
        assert_eq!(gitee, "https://gitee.com/burncloud/burncloud/releases");
    }
}