//! Core installer implementation

use log::{info, warn};
use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use zip::ZipArchive;

use crate::error::{InstallerError, InstallerResult};
use crate::platform::Platform;
use crate::registry::{get_software, list_software};
use crate::software::{
    GitHubAsset, GitHubRelease, InstallMethod, InstallStatus, ShellType, Software,
};

/// Installer configuration
#[derive(Debug, Clone)]
pub struct InstallerConfig {
    /// Download directory
    pub download_dir: PathBuf,
    /// Install directory
    pub install_dir: PathBuf,
    /// Bundle directory for offline installation
    pub bundle_dir: Option<PathBuf>,
    /// HTTP client
    pub client: Client,
    /// Platform
    pub platform: Platform,
    /// Auto-install dependencies
    pub auto_deps: bool,
    /// Force reinstall
    pub force: bool,
}

impl Default for InstallerConfig {
    fn default() -> Self {
        let download_dir = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("./downloads"))
            .join("burncloud");

        let install_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("./apps"))
            .join("burncloud")
            .join("installed");

        Self {
            download_dir,
            install_dir,
            bundle_dir: None,
            client: Client::new(),
            platform: Platform::current(),
            auto_deps: false,
            force: false,
        }
    }
}

impl InstallerConfig {
    /// Create new configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set download directory
    pub fn with_download_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.download_dir = path.into();
        self
    }

    /// Set install directory
    pub fn with_install_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.install_dir = path.into();
        self
    }

    /// Enable auto-dependency installation
    pub fn with_auto_deps(mut self, auto_deps: bool) -> Self {
        self.auto_deps = auto_deps;
        self
    }

    /// Force reinstall
    pub fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Set bundle directory for offline installation
    pub fn with_bundle_dir(mut self, path: Option<PathBuf>) -> Self {
        self.bundle_dir = path;
        self
    }
}

/// Software installer
pub struct Installer {
    config: InstallerConfig,
}

impl Installer {
    /// Create new installer
    pub fn new(config: InstallerConfig) -> Self {
        Self { config }
    }

    /// Create installer with default configuration
    pub fn with_default_config() -> Self {
        Self::new(InstallerConfig::default())
    }

    /// List available software
    pub fn list_available(&self) -> Vec<&Software> {
        list_software()
    }

    /// Get software by ID
    pub fn get_software(&self, id: &str) -> Option<&Software> {
        get_software(id)
    }

    /// Check installation status
    pub async fn check_status(&self, software_id: &str) -> InstallerResult<InstallStatus> {
        // Check if software exists
        let software = get_software(software_id)
            .ok_or_else(|| InstallerError::SoftwareNotFound(software_id.to_string()))?;

        // Check platform support
        if !software.supports_platform(self.config.platform.os, self.config.platform.arch) {
            return Err(InstallerError::PlatformNotSupported(format!(
                "{} is not supported on {}",
                software.name, self.config.platform
            )));
        }

        // Check install directory for installed marker
        let install_marker = self.config.install_dir.join(software_id).join(".installed");

        if install_marker.exists() {
            Ok(InstallStatus::Installed)
        } else {
            Ok(InstallStatus::NotInstalled)
        }
    }

    /// Install software
    pub async fn install(&self, software_id: &str) -> InstallerResult<()> {
        let software = get_software(software_id)
            .ok_or_else(|| InstallerError::SoftwareNotFound(software_id.to_string()))?;

        info!("Installing {}...", software.name);

        // Check platform support
        if !software.supports_platform(self.config.platform.os, self.config.platform.arch) {
            return Err(InstallerError::PlatformNotSupported(format!(
                "{} is not supported on {}",
                software.name, self.config.platform
            )));
        }

        // Check dependencies
        self.check_dependencies(software).await?;

        // Create directories
        fs::create_dir_all(&self.config.download_dir).await?;
        fs::create_dir_all(&self.config.install_dir).await?;

        // Execute installation based on method
        match &software.install_method {
            InstallMethod::Script { url, shell } => {
                self.install_via_script(software, url, shell).await?;
            }
            InstallMethod::GitHubRelease {
                owner,
                repo,
                asset_patterns,
                extract_archive,
            } => {
                self.install_from_github(software, owner, repo, asset_patterns, *extract_archive)
                    .await?;
            }
            InstallMethod::DirectDownload { url, filename } => {
                self.install_from_url(software, url, filename.as_deref())
                    .await?;
            }
            InstallMethod::PackageManager {
                windows,
                macos,
                linux,
            } => {
                let cmd = match self.config.platform.os {
                    crate::platform::OS::Windows => windows.as_ref(),
                    crate::platform::OS::MacOS => macos.as_ref(),
                    crate::platform::OS::Linux => linux.as_ref(),
                    _ => None,
                };

                if let Some(cmd) = cmd {
                    self.install_via_package_manager(software, cmd).await?;
                } else {
                    return Err(InstallerError::PlatformNotSupported(
                        "No package manager command for this platform".to_string(),
                    ));
                }
            }
            InstallMethod::NpmPackage {
                package,
                version,
                global,
            } => {
                if let Some(bundle_dir) = &self.config.bundle_dir {
                    // Offline mode: install from bundle
                    self.install_npm_from_bundle(
                        software,
                        package,
                        version.as_deref(),
                        *global,
                        bundle_dir,
                    )
                    .await?;
                } else {
                    // Online mode: install via npm
                    self.install_via_npm(software, package, version.as_deref(), *global)
                        .await?;
                }
            }
            InstallMethod::GitRepo {
                url,
                branch,
                build_command,
                package_manager,
            } => {
                self.install_from_git(
                    software,
                    url,
                    branch.as_deref(),
                    build_command.as_deref(),
                    package_manager.as_deref(),
                )
                .await?;
            }
        }

        // Create install marker
        let install_marker = self.config.install_dir.join(software_id).join(".installed");
        if let Some(parent) = install_marker.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(&install_marker, chrono::Utc::now().to_rfc3339()).await?;

        info!("Successfully installed {}", software.name);
        Ok(())
    }

    /// Install software from local file (offline mode)
    pub async fn install_from_local(
        &self,
        software_id: &str,
        local_path: &Path,
    ) -> InstallerResult<()> {
        let software = get_software(software_id)
            .ok_or_else(|| InstallerError::SoftwareNotFound(software_id.to_string()))?;

        info!(
            "Installing {} from local file: {}",
            software.name,
            local_path.display()
        );

        // Check if local file exists
        if !local_path.exists() {
            return Err(InstallerError::FileSystem(format!(
                "Local file not found: {}",
                local_path.display()
            )));
        }

        // Check platform support
        if !software.supports_platform(self.config.platform.os, self.config.platform.arch) {
            return Err(InstallerError::PlatformNotSupported(format!(
                "{} is not supported on {}",
                software.name, self.config.platform
            )));
        }

        // Check dependencies (skip for offline mode if bundle_dir is set)
        if self.config.bundle_dir.is_none() {
            self.check_dependencies(software).await?;
        }

        // Create install directory
        let install_dir = self.config.install_dir.join(software_id);
        fs::create_dir_all(&install_dir).await?;

        // Determine file type and handle accordingly
        let extension = local_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match extension.as_str() {
            "zip" => {
                // Extract zip archive
                self.extract_zip(local_path, &install_dir)?;
                info!(
                    "Extracted {} to {}",
                    local_path.display(),
                    install_dir.display()
                );
            }
            "ps1" => {
                // PowerShell script - execute it
                info!("Executing PowerShell script: {}", local_path.display());
                let status = Command::new("powershell")
                    .args(["-ExecutionPolicy", "Bypass", "-File"])
                    .arg(local_path)
                    .status()
                    .map_err(|e| {
                        InstallerError::Script(format!(
                            "Failed to execute PowerShell script: {}",
                            e
                        ))
                    })?;

                if !status.success() {
                    return Err(InstallerError::Script(format!(
                        "PowerShell script exited with code: {}",
                        status.code().unwrap_or(-1)
                    )));
                }
            }
            "sh" => {
                // Bash/Shell script - execute it
                info!("Executing shell script: {}", local_path.display());
                let status = Command::new("bash").arg(local_path).status().map_err(|e| {
                    InstallerError::Script(format!("Failed to execute shell script: {}", e))
                })?;

                if !status.success() {
                    return Err(InstallerError::Script(format!(
                        "Shell script exited with code: {}",
                        status.code().unwrap_or(-1)
                    )));
                }
            }
            "exe" | "msi" => {
                // Installer executable - run it
                info!("Running installer: {}", local_path.display());
                let status = if extension == "msi" {
                    Command::new("msiexec")
                        .args(["/i", local_path.to_str().unwrap(), "/quiet", "/norestart"])
                        .status()
                        .map_err(|e| {
                            InstallerError::Script(format!("Failed to run MSI installer: {}", e))
                        })?
                } else {
                    // Try silent install flags, fall back to normal install
                    let result = Command::new(local_path)
                        .args(["/S", "/silent", "/quiet"])
                        .status();

                    match result {
                        Ok(s) => s,
                        Err(_) => Command::new(local_path).status().map_err(|e| {
                            InstallerError::Script(format!("Failed to run installer: {}", e))
                        })?,
                    }
                };

                if !status.success() {
                    warn!(
                        "Installer exited with code: {}",
                        status.code().unwrap_or(-1)
                    );
                }
            }
            _ => {
                // Copy single file
                let file_name = local_path
                    .file_name()
                    .ok_or_else(|| InstallerError::FileSystem("Invalid file name".to_string()))?;
                let dest_path = install_dir.join(file_name);
                fs::copy(local_path, &dest_path).await?;
                info!("Copied {} to {}", local_path.display(), dest_path.display());
            }
        }

        // Create install marker
        let install_marker = install_dir.join(".installed");
        fs::write(&install_marker, chrono::Utc::now().to_rfc3339()).await?;

        info!("Successfully installed {} from local file", software.name);
        Ok(())
    }

    /// Install dependency from bundle directory (offline mode)
    pub async fn install_dependency_from_bundle(&self, dep_name: &str) -> InstallerResult<()> {
        let bundle_dir =
            self.config.bundle_dir.as_ref().ok_or_else(|| {
                InstallerError::Configuration("Bundle directory not set".to_string())
            })?;

        // Look for the dependency in the bundle directory
        // Convention: bundle/<dep-name>/<platform>/<arch>/<file>
        let mut platform_dir = bundle_dir
            .join(dep_name)
            .join(self.config.platform.os.to_string().to_lowercase())
            .join(self.config.platform.arch.to_string().to_lowercase());

        if !platform_dir.exists() {
            // Try without arch distinction
            platform_dir = bundle_dir
                .join(dep_name)
                .join(self.config.platform.os.to_string().to_lowercase());

            if !platform_dir.exists() {
                return Err(InstallerError::DependencyNotFound(format!(
                    "Dependency '{}' not found in bundle: {}",
                    dep_name,
                    bundle_dir.display()
                )));
            }
        }

        // Find the installer file
        let entries: Vec<_> = std::fs::read_dir(&platform_dir)
            .map_err(|e| InstallerError::FileSystem(e.to_string()))?
            .filter_map(|e| e.ok())
            .collect();

        if entries.is_empty() {
            return Err(InstallerError::DependencyNotFound(format!(
                "No installer found for '{}' in {}",
                dep_name,
                platform_dir.display()
            )));
        }

        // Get the first installer file
        let installer_path = entries[0].path();

        // Run the installer
        info!(
            "Installing {} from bundle: {}",
            dep_name,
            installer_path.display()
        );

        let extension = installer_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if extension == "msi" {
            // MSI installer
            let status = Command::new("msiexec")
                .args([
                    "/i",
                    installer_path.to_str().unwrap(),
                    "/quiet",
                    "/norestart",
                ])
                .status()
                .map_err(|e| {
                    InstallerError::Script(format!("Failed to run MSI installer: {}", e))
                })?;

            if !status.success() {
                return Err(InstallerError::InstallationFailed(format!(
                    "MSI installer failed for {}",
                    dep_name
                )));
            }
        } else if extension == "exe" {
            // EXE installer (silent install)
            let status = Command::new(&installer_path)
                .args(["/S", "/silent", "/quiet"])
                .status()
                .unwrap_or_else(|_| {
                    // Try without args (some installers don't support silent)
                    Command::new(&installer_path).status().unwrap()
                });

            if !status.success() {
                return Err(InstallerError::InstallationFailed(format!(
                    "Installer failed for {}",
                    dep_name
                )));
            }
        } else if extension == "zip" {
            // Zip archive - extract to a temp location and run any installer inside
            let temp_dir = std::env::temp_dir().join(format!("burncloud-{}", dep_name));
            self.extract_zip(&installer_path, &temp_dir)?;

            // Look for an installer inside
            for entry in std::fs::read_dir(&temp_dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "exe" || ext == "msi" {
                        // Run the installer
                        let _ = Command::new(&path).status();
                        break;
                    }
                }
            }
        }

        info!("Successfully installed {} from bundle", dep_name);
        Ok(())
    }

    /// Check dependencies
    async fn check_dependencies(&self, software: &Software) -> InstallerResult<()> {
        for dep in &software.dependencies {
            info!("Checking dependency: {}", dep.name);

            if !self.check_dependency(dep).await? {
                warn!("Dependency {} not found", dep.name);

                if self.config.auto_deps {
                    if let Some(auto_install) = &dep.auto_install {
                        info!("Auto-installing dependency: {}", dep.name);
                        self.install_dependency(dep, auto_install).await?;
                    } else if let Some(hint) = &dep.install_hint {
                        return Err(InstallerError::DependencyNotFound(format!(
                            "{} not found. Install it from: {}",
                            dep.name, hint
                        )));
                    } else {
                        return Err(InstallerError::DependencyNotFound(dep.name.clone()));
                    }
                } else if let Some(hint) = &dep.install_hint {
                    return Err(InstallerError::DependencyNotFound(format!(
                        "{} not found. Install it from: {} (or use --auto-deps)",
                        dep.name, hint
                    )));
                } else {
                    return Err(InstallerError::DependencyNotFound(format!(
                        "{} not found. Use --auto-deps to install automatically",
                        dep.name
                    )));
                }
            }
        }
        Ok(())
    }

    /// Check if a dependency is installed
    async fn check_dependency(&self, dep: &crate::software::Dependency) -> InstallerResult<bool> {
        let output = if self.config.platform.is_windows() {
            Command::new("cmd")
                .args(["/C", &dep.check_command])
                .output()
        } else {
            Command::new("sh").args(["-c", &dep.check_command]).output()
        };

        match output {
            Ok(output) => {
                if !output.status.success() {
                    return Ok(false);
                }

                if let Some(expected) = &dep.expected_output {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    Ok(stdout.contains(expected))
                } else {
                    Ok(true)
                }
            }
            Err(_) => Ok(false),
        }
    }

    /// Install a dependency
    async fn install_dependency(
        &self,
        dep: &crate::software::Dependency,
        method: &InstallMethod,
    ) -> InstallerResult<()> {
        match method {
            InstallMethod::PackageManager {
                windows,
                macos,
                linux,
            } => {
                let cmd = match self.config.platform.os {
                    crate::platform::OS::Windows => windows.as_ref(),
                    crate::platform::OS::MacOS => macos.as_ref(),
                    crate::platform::OS::Linux => linux.as_ref(),
                    _ => None,
                };

                if let Some(cmd) = cmd {
                    self.install_via_package_manager_raw(cmd).await?;
                    info!("Successfully installed dependency: {}", dep.name);
                    Ok(())
                } else {
                    Err(InstallerError::PlatformNotSupported(
                        "No package manager command for this platform".to_string(),
                    ))
                }
            }
            _ => Err(InstallerError::InstallationFailed(format!(
                "Unsupported auto-install method for dependency: {}",
                dep.name
            ))),
        }
    }

    /// Install via script
    async fn install_via_script(
        &self,
        software: &Software,
        url: &str,
        shell: &ShellType,
    ) -> InstallerResult<()> {
        info!("Downloading installation script from {}", url);

        let script_content = self.config.client.get(url).send().await?.text().await?;

        let script_path = self.config.download_dir.join(format!(
            "install-{}{}",
            software.id,
            match shell {
                ShellType::PowerShell => ".ps1",
                ShellType::Bash => ".sh",
                ShellType::Sh => ".sh",
                ShellType::Cmd => ".bat",
            }
        ));

        fs::write(&script_path, &script_content).await?;

        info!("Executing installation script...");

        let result = match shell {
            ShellType::PowerShell => Command::new("powershell")
                .args(["-ExecutionPolicy", "Bypass", "-File"])
                .arg(&script_path)
                .status(),
            ShellType::Bash => Command::new("bash").arg(&script_path).status(),
            ShellType::Sh => Command::new("sh").arg(&script_path).status(),
            ShellType::Cmd => Command::new("cmd").args(["/C"]).arg(&script_path).status(),
        };

        match result {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(InstallerError::Script(format!(
                "Script exited with code: {}",
                status.code().unwrap_or(-1)
            ))),
            Err(e) => Err(InstallerError::Script(format!(
                "Failed to execute script: {}",
                e
            ))),
        }
    }

    /// Install from GitHub Releases
    async fn install_from_github(
        &self,
        software: &Software,
        owner: &str,
        repo: &str,
        asset_patterns: &HashMap<(crate::platform::OS, crate::platform::Arch), String>,
        extract_archive: Option<bool>,
    ) -> InstallerResult<()> {
        info!("Fetching latest release from {}/{}", owner, repo);

        let release = self.fetch_github_release(owner, repo).await?;

        info!("Latest release: {}", release.tag_name);

        // Find matching asset
        let pattern = asset_patterns
            .get(&(self.config.platform.os, self.config.platform.arch))
            .ok_or_else(|| {
                InstallerError::PlatformNotSupported(format!(
                    "No asset pattern for {}",
                    self.config.platform
                ))
            })?;

        let asset = self.find_matching_asset(&release.assets, pattern)?;

        info!("Downloading {}...", asset.name);

        let download_path = self.config.download_dir.join(&asset.name);
        self.download_file(&asset.browser_download_url, &download_path)
            .await?;

        info!("Downloaded to {}", download_path.display());

        // Handle archive extraction if needed
        if extract_archive.unwrap_or(false) {
            let software_install_dir = self.config.install_dir.join(&software.id);
            self.extract_zip(&download_path, &software_install_dir)?;
            info!("Extracted to {}", software_install_dir.display());
        } else {
            // Move file to install directory
            let software_install_dir = self.config.install_dir.join(&software.id);
            fs::create_dir_all(&software_install_dir).await?;
            let final_path = software_install_dir.join(&asset.name);
            fs::rename(&download_path, &final_path).await?;
            info!("Installed to {}", final_path.display());
        }

        Ok(())
    }

    /// Install from direct URL
    async fn install_from_url(
        &self,
        _software: &Software,
        url: &str,
        filename: Option<&str>,
    ) -> InstallerResult<()> {
        let filename = filename.unwrap_or_else(|| url.split('/').next_back().unwrap_or("download"));

        info!("Downloading {} from {}", filename, url);

        let download_path = self.config.download_dir.join(filename);
        self.download_file(url, &download_path).await?;

        info!("Downloaded to {}", download_path.display());

        Ok(())
    }

    /// Install via package manager
    async fn install_via_package_manager(
        &self,
        software: &Software,
        cmd: &str,
    ) -> InstallerResult<()> {
        info!("Installing {} via package manager...", software.name);
        self.install_via_package_manager_raw(cmd).await
    }

    /// Execute package manager command
    async fn install_via_package_manager_raw(&self, cmd: &str) -> InstallerResult<()> {
        let result = if self.config.platform.is_windows() {
            Command::new("cmd").args(["/C", cmd]).status()
        } else {
            Command::new("sh").args(["-c", cmd]).status()
        };

        match result {
            Ok(status) if status.success() => Ok(()),
            Ok(status) => Err(InstallerError::InstallationFailed(format!(
                "Package manager exited with code: {}",
                status.code().unwrap_or(-1)
            ))),
            Err(e) => Err(InstallerError::InstallationFailed(format!(
                "Failed to run package manager: {}",
                e
            ))),
        }
    }

    /// Fetch GitHub release information
    async fn fetch_github_release(
        &self,
        owner: &str,
        repo: &str,
    ) -> InstallerResult<GitHubRelease> {
        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            owner, repo
        );

        let response = self
            .config
            .client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("User-Agent", "BurnCloud-Installer")
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(InstallerError::GitHub(format!(
                "Failed to fetch release: {}",
                response.status()
            )));
        }

        let release: GitHubRelease = response.json().await?;
        Ok(release)
    }

    /// Find matching asset from pattern
    fn find_matching_asset(
        &self,
        assets: &[GitHubAsset],
        pattern: &str,
    ) -> InstallerResult<GitHubAsset> {
        // Convert glob-like pattern to regex
        let regex_pattern = pattern.replace(".", "\\.").replace("*", ".*");

        let regex = Regex::new(&format!("^{}$", regex_pattern))
            .map_err(|e| InstallerError::Configuration(format!("Invalid pattern: {}", e)))?;

        for asset in assets {
            if regex.is_match(&asset.name) {
                return Ok(asset.clone());
            }
        }

        Err(InstallerError::Download(format!(
            "No asset matching pattern: {}",
            pattern
        )))
    }

    /// Extract zip archive to target directory
    fn extract_zip(&self, zip_path: &Path, target_dir: &Path) -> InstallerResult<()> {
        info!(
            "Extracting {} to {}",
            zip_path.display(),
            target_dir.display()
        );

        let file = File::open(zip_path)
            .map_err(|e| InstallerError::FileSystem(format!("Failed to open zip file: {}", e)))?;
        let reader = BufReader::new(file);
        let mut archive = ZipArchive::new(reader).map_err(|e| {
            InstallerError::InstallationFailed(format!("Failed to read zip archive: {}", e))
        })?;

        // Create target directory
        std::fs::create_dir_all(target_dir).map_err(|e| {
            InstallerError::FileSystem(format!("Failed to create directory: {}", e))
        })?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                InstallerError::InstallationFailed(format!("Failed to read zip entry: {}", e))
            })?;

            let outpath = match file.enclosed_name() {
                Some(path) => target_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                // Directory
                std::fs::create_dir_all(&outpath).map_err(|e| {
                    InstallerError::FileSystem(format!("Failed to create directory: {}", e))
                })?;
            } else {
                // File
                if let Some(p) = outpath.parent() {
                    if !p.exists() {
                        std::fs::create_dir_all(p).map_err(|e| {
                            InstallerError::FileSystem(format!("Failed to create directory: {}", e))
                        })?;
                    }
                }
                let mut outfile = File::create(&outpath).map_err(|e| {
                    InstallerError::FileSystem(format!("Failed to create file: {}", e))
                })?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| {
                    InstallerError::FileSystem(format!("Failed to write file: {}", e))
                })?;
            }
        }

        info!("Extraction complete");
        Ok(())
    }

    /// Download file with progress
    async fn download_file(&self, url: &str, path: &Path) -> InstallerResult<()> {
        let response = self.config.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(InstallerError::Download(format!(
                "Download failed: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        fs::write(path, bytes).await?;

        Ok(())
    }

    /// Install via npm (online mode)
    async fn install_via_npm(
        &self,
        software: &Software,
        package: &str,
        version: Option<&str>,
        global: bool,
    ) -> InstallerResult<()> {
        info!("Installing {} via npm...", software.name);

        let package_spec = match version {
            Some(v) => format!("{}@{}", package, v),
            None => package.to_string(),
        };

        let mut args = vec!["install"];
        if global {
            args.push("-g");
        }
        args.push(&package_spec);

        let result = if self.config.platform.is_windows() {
            Command::new("cmd").args(["/C", "npm"]).args(&args).status()
        } else {
            Command::new("sh")
                .args(["-c", &format!("npm {}", args.join(" "))])
                .status()
        };

        match result {
            Ok(status) if status.success() => {
                info!("Successfully installed {} via npm", software.name);
                Ok(())
            }
            Ok(status) => Err(InstallerError::InstallationFailed(format!(
                "npm install exited with code: {}",
                status.code().unwrap_or(-1)
            ))),
            Err(e) => Err(InstallerError::InstallationFailed(format!(
                "Failed to run npm install: {}",
                e
            ))),
        }
    }

    /// Install npm package from bundle (offline mode)
    async fn install_npm_from_bundle(
        &self,
        software: &Software,
        _package: &str,
        _version: Option<&str>,
        global: bool,
        bundle_dir: &Path,
    ) -> InstallerResult<()> {
        info!("Installing {} from bundle...", software.name);

        // Look for the tarball in bundle/software/<id>/npm-package/
        let npm_dir = bundle_dir
            .join("software")
            .join(&software.id)
            .join("npm-package");

        if !npm_dir.exists() {
            return Err(InstallerError::FileSystem(format!(
                "npm package directory not found in bundle: {}",
                npm_dir.display()
            )));
        }

        // Find the tarball file
        let tarball = std::fs::read_dir(&npm_dir)
            .map_err(|e| InstallerError::FileSystem(e.to_string()))?
            .filter_map(|e| e.ok())
            .find(|e| {
                e.path()
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext == "tgz")
                    .unwrap_or(false)
            });

        let tarball_path = tarball
            .ok_or_else(|| {
                InstallerError::FileSystem(format!("No .tgz file found in {}", npm_dir.display()))
            })?
            .path();

        info!("Installing from tarball: {}", tarball_path.display());

        // Install from tarball
        let tarball_str = tarball_path.to_string_lossy();
        let mut args = vec!["install"];
        if global {
            args.push("-g");
        }
        args.push(&tarball_str);

        let result = if self.config.platform.is_windows() {
            Command::new("cmd").args(["/C", "npm"]).args(&args).status()
        } else {
            Command::new("sh")
                .args(["-c", &format!("npm {}", args.join(" "))])
                .status()
        };

        match result {
            Ok(status) if status.success() => {
                info!("Successfully installed {} from bundle", software.name);
                Ok(())
            }
            Ok(status) => Err(InstallerError::InstallationFailed(format!(
                "npm install from bundle exited with code: {}",
                status.code().unwrap_or(-1)
            ))),
            Err(e) => Err(InstallerError::InstallationFailed(format!(
                "Failed to run npm install from bundle: {}",
                e
            ))),
        }
    }

    /// Install from Git repository
    async fn install_from_git(
        &self,
        software: &Software,
        url: &str,
        branch: Option<&str>,
        build_command: Option<&str>,
        _package_manager: Option<&str>,
    ) -> InstallerResult<()> {
        info!("Installing {} from Git repository: {}", software.name, url);

        let clone_dir = self.config.install_dir.join(&software.id);

        // Clone the repository
        let mut clone_args = vec!["clone", url];
        if let Some(branch) = branch {
            clone_args.extend_from_slice(&["-b", branch]);
        }
        clone_args.push(clone_dir.to_str().unwrap());

        let clone_result = if self.config.platform.is_windows() {
            Command::new("cmd")
                .args(["/C", "git"])
                .args(&clone_args)
                .status()
        } else {
            Command::new("sh")
                .args(["-c", &format!("git {}", clone_args.join(" "))])
                .status()
        };

        match clone_result {
            Ok(status) if status.success() => {
                info!("Cloned {} successfully", software.name);
            }
            Ok(status) => {
                return Err(InstallerError::InstallationFailed(format!(
                    "git clone exited with code: {}",
                    status.code().unwrap_or(-1)
                )));
            }
            Err(e) => {
                return Err(InstallerError::InstallationFailed(format!(
                    "Failed to run git clone: {}",
                    e
                )));
            }
        }

        // Run build command if specified
        if let Some(build_cmd) = build_command {
            info!("Running build command: {}", build_cmd);

            let build_result = if self.config.platform.is_windows() {
                Command::new("cmd")
                    .args(["/C", build_cmd])
                    .current_dir(&clone_dir)
                    .status()
            } else {
                Command::new("sh")
                    .args(["-c", build_cmd])
                    .current_dir(&clone_dir)
                    .status()
            };

            match build_result {
                Ok(status) if status.success() => {
                    info!("Build completed successfully");
                }
                Ok(status) => {
                    warn!(
                        "Build command exited with code: {}",
                        status.code().unwrap_or(-1)
                    );
                }
                Err(e) => {
                    warn!("Failed to run build command: {}", e);
                }
            }
        }

        info!("Successfully installed {} from Git", software.name);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installer_config() {
        let config = InstallerConfig::new();
        assert!(config.download_dir.to_string_lossy().contains("burncloud"));
    }

    #[test]
    fn test_installer_list() {
        let installer = Installer::with_default_config();
        let list = installer.list_available();
        assert!(!list.is_empty());
    }

    #[test]
    fn test_get_software() {
        let installer = Installer::with_default_config();
        let software = installer.get_software("openclaw");
        assert!(software.is_some());
    }
}
