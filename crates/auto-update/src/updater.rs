//! 自动更新器核心实现

use crate::{UpdateConfig, UpdateError, UpdateResult};
use log::{error, info};
use self_update::backends::github;
use semver;

/// 自动更新器
#[derive(Clone)]
pub struct AutoUpdater {
    pub(crate) config: UpdateConfig,
}

impl AutoUpdater {
    pub fn new(config: UpdateConfig) -> Self {
        Self { config }
    }

    pub fn with_default_config() -> Self {
        Self::new(UpdateConfig::default())
    }

    pub async fn check_for_updates(&self) -> UpdateResult<bool> {
        self.sync_check_for_updates()
    }

    pub async fn update_with_fallback(&self) -> UpdateResult<()> {
        self.sync_update()
    }

    pub fn current_version(&self) -> &str {
        &self.config.current_version
    }

    pub fn set_config(&mut self, config: UpdateConfig) {
        self.config = config;
    }

    pub fn config(&self) -> &UpdateConfig {
        &self.config
    }

    pub fn get_download_links(&self) -> (String, String) {
        self.config.download_links()
    }

    pub fn get_latest_release_info(&self) -> UpdateResult<Option<(String, String)>> {
        info!("获取最新发布版本信息..");

        let target = self_update::get_target();
        let releases = github::ReleaseList::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .with_target(target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.first() {
            Ok(Some((
                latest_release.version.clone(),
                latest_release.name.clone(),
            )))
        } else {
            Ok(None)
        }
    }

    pub fn needs_update(&self) -> UpdateResult<bool> {
        info!("检查是否需要更新..");

        let target = self_update::get_target();
        let releases = github::ReleaseList::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .with_target(target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.first() {
            let current_version = self
                .config
                .current_version
                .trim_start_matches('v')
                .to_string();
            let latest_version = latest_release.version.to_string();

            match (
                semver::Version::parse(&current_version),
                semver::Version::parse(&latest_version),
            ) {
                (Ok(current), Ok(latest)) => Ok(latest > current),
                _ => Ok(latest_version != current_version),
            }
        } else {
            error!("未找到任何发布版本");
            Err(UpdateError::GitHub("未找到任何发布版本".to_string()))
        }
    }

    pub fn sync_update(&self) -> UpdateResult<()> {
        info!("开始同步更新应用程序..");

        let target = self_update::get_target();

        let update = github::Update::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .target(target)
            .bin_name(&self.config.bin_name)
            .current_version(&self.config.current_version)
            .show_download_progress(false)
            .no_confirm(true)
            .build()
            .map_err(UpdateError::from)?;

        let status = update.update().map_err(UpdateError::from)?;
        if status.updated() {
            info!("更新成功，新版本: {}", status.version());
        } else {
            info!("已是最新版本");
        }
        Ok(())
    }

    pub fn sync_check_for_updates(&self) -> UpdateResult<bool> {
        info!("同步检查更新中...");

        let target = self_update::get_target();
        let releases = github::ReleaseList::configure()
            .repo_owner(&self.config.github_owner)
            .repo_name(&self.config.github_repo)
            .with_target(target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.first() {
            let current_version = &self.config.current_version;
            let release_version = latest_release.version.to_string();
            let current_clean = current_version.trim_start_matches('v');
            Ok(release_version != current_clean)
        } else {
            error!("未找到任何发布版本");
            Err(UpdateError::GitHub("未找到任何发布版本".to_string()))
        }
    }
}
