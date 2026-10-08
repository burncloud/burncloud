//! 自动更新器核心实现

use crate::{UpdateConfig, UpdateError, UpdateResult};
use self_update::backends::github;
use semver;
use tracing::{error, info};

/// Whether `release_version` is an upgrade over `current_version`.
///
/// Both version strings may carry a leading `v`, which is stripped before comparison because release tags often
/// do and `semver::Version::parse` rejects it.
///
/// **Parsed comparison when both parse, string inequality otherwise.** The two branches answer different
/// questions, and the second is the one to be careful about:
///
/// * both parse -- `latest > current` is a strict upgrade, so a release that is the same version or **older**
///   reports `false`. That matters because the caller's next step is to replace the running executable: a
///   string comparison would treat `1.0.5` against `1.0.4` as an update and **downgrade** the application,
///   which is how this crate behaved before this function existed.
/// * either fails to parse -- the two strings are compared for inequality. An unparseable version cannot be
///   ordered, so the best that can be said is whether it differs; the alternative would be to report "no
///   update" and silently never update.
///
/// ## A prerelease is not treated specially
///
/// `semver` orders `1.0.0-beta.1 < 1.0.0`, which is the standard rule, so a stable `1.0.0` is an upgrade over a
/// running `1.0.0-beta.1` and the reverse is not an upgrade. There is **no policy here for excluding
/// prereleases from the candidate set**: if the GitHub release list offers `2.0.0-rc.1` as its first entry, this
/// function will call it an upgrade over `1.9.0`. Whether prereleases should be offered at all depends on how
/// the release list is ordered and filtered, which is the caller's contract rather than this function's -- the
/// plan for this crate says the security rules follow "an explicit update contract", and this is the point where
/// that contract would be applied. Recorded rather than decided.
pub fn is_upgrade_over(release_version: &str, current_version: &str) -> bool {
    let release = release_version.trim_start_matches('v');
    let current = current_version.trim_start_matches('v');

    match (
        semver::Version::parse(release),
        semver::Version::parse(current),
    ) {
        (Ok(release), Ok(current)) => release > current,
        _ => release != current,
    }
}

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
            .repo_owner(self.config.github_owner.clone())
            .repo_name(self.config.github_repo.clone())
            .filter_target(target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.latest() {
            Ok(Some((
                latest_release.version().to_owned(),
                latest_release.name().to_owned(),
            )))
        } else {
            Ok(None)
        }
    }

    pub fn needs_update(&self) -> UpdateResult<bool> {
        info!("检查是否需要更新..");

        let target = self_update::get_target();
        let releases = github::ReleaseList::configure()
            .repo_owner(self.config.github_owner.clone())
            .repo_name(self.config.github_repo.clone())
            .filter_target(target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.latest() {
            let latest_version = latest_release.version().to_owned();
            // The same decision as `sync_check_for_updates`, from the same function. Before it existed the two
            // paths disagreed: this one parsed both versions, and the other compared the strings.
            Ok(is_upgrade_over(
                &latest_version,
                &self.config.current_version,
            ))
        } else {
            error!("未找到任何发布版本");
            Err(UpdateError::GitHub("未找到任何发布版本".to_string()))
        }
    }

    pub fn sync_update(&self) -> UpdateResult<()> {
        info!("开始同步更新应用程序..");

        let target = self_update::get_target();

        let update = github::Update::configure()
            .repo_owner(self.config.github_owner.clone())
            .repo_name(self.config.github_repo.clone())
            .target(target)
            .bin_name(self.config.bin_name.clone())
            .current_version(self.config.current_version.clone())
            .show_download_progress(false)
            .no_confirm(true)
            .build()
            .map_err(UpdateError::from)?;

        let status = update.update().map_err(UpdateError::from)?;
        if status.is_updated() {
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
            .repo_owner(self.config.github_owner.clone())
            .repo_name(self.config.github_repo.clone())
            .filter_target(target)
            .build()
            .map_err(UpdateError::from)?
            .fetch()
            .map_err(UpdateError::from)?;

        if let Some(latest_release) = releases.latest() {
            let release_version = latest_release.version().to_owned();
            // This compared the two strings for inequality until `is_upgrade_over` was extracted, which meant a
            // release **older** than the running version reported "update available" and the caller's next step
            // -- `sync_update`, which replaces the executable -- would have downgraded the application.
            Ok(is_upgrade_over(
                &release_version,
                &self.config.current_version,
            ))
        } else {
            error!("未找到任何发布版本");
            Err(UpdateError::GitHub("未找到任何发布版本".to_string()))
        }
    }
}
