//! Core installer implementation

use regex::Regex;
use reqwest::Client;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::fs;
use tracing::{info, warn};
use zip::ZipArchive;

use crate::error::{InstallerError, InstallerResult};
use crate::platform::Platform;
use crate::registry::{get_software, list_software};
use crate::software::{
    GitHubAsset, GitHubRelease, InstallMethod, InstallStatus, ShellType, Software,
};

const CHINA_NPM_MIRROR: &str = "https://registry.npmmirror.com";

/// A validated npm registry URL.
///
/// This type stays private so callers cannot bypass validation. It deliberately
/// does not implement `Debug` or `Display`, because registry URLs may contain
/// sensitive query parameters even though URL userinfo is forbidden.
struct NpmRegistry {
    url: reqwest::Url,
}

impl NpmRegistry {
    fn parse(value: &str) -> InstallerResult<Self> {
        let value = value.trim();
        if value.is_empty() {
            return Err(InstallerError::Configuration(
                "NPM_MIRROR is empty".to_string(),
            ));
        }

        // Direct process arguments already prevent shell interpretation. Reject
        // raw shell metacharacters as an additional boundary and require callers
        // to percent-encode any such character that is genuinely part of a URL.
        if value.chars().any(|character| {
            character.is_control()
                || character.is_whitespace()
                || matches!(
                    character,
                    ';' | '|' | '&' | '`' | '$' | '<' | '>' | '\'' | '"' | '\\'
                )
        }) {
            return Err(InstallerError::Configuration(
                "NPM_MIRROR contains unsupported characters".to_string(),
            ));
        }

        let url = reqwest::Url::parse(value).map_err(|_| {
            InstallerError::Configuration(
                "NPM_MIRROR must be a valid absolute HTTP(S) URL".to_string(),
            )
        })?;

        if !matches!(url.scheme(), "http" | "https") {
            return Err(InstallerError::Configuration(
                "NPM_MIRROR must use http or https".to_string(),
            ));
        }
        if url.host_str().is_none() {
            return Err(InstallerError::Configuration(
                "NPM_MIRROR must include a host".to_string(),
            ));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(InstallerError::Configuration(
                "NPM_MIRROR must not include URL userinfo".to_string(),
            ));
        }

        Ok(Self { url })
    }

    fn as_str(&self) -> &str {
        self.url.as_str()
    }

    fn safe_origin(&self) -> String {
        // A URL with a validated host always reaches this branch. Keep the
        // fallback generic rather than risk logging the original URL.
        self.url
            .host_str()
            .map(|host| format!("{}://{}", self.url.scheme(), host))
            .unwrap_or_else(|| "custom registry".to_string())
    }
}

fn resolve_npm_registry(
    custom_mirror: Option<&str>,
    in_china: bool,
) -> InstallerResult<Option<NpmRegistry>> {
    if let Some(value) = custom_mirror.filter(|value| !value.trim().is_empty()) {
        return NpmRegistry::parse(value).map(Some);
    }

    if in_china {
        NpmRegistry::parse(CHINA_NPM_MIRROR).map(Some)
    } else {
        Ok(None)
    }
}

fn npm_install_command(platform: &Platform, args: &[String], current_path: &str) -> Command {
    // npm is installed as npm.cmd on Windows. Invoking it directly avoids cmd
    // parsing while preserving the same argument boundaries as Unix.
    let executable = if platform.is_windows() {
        "npm.cmd"
    } else {
        "npm"
    };
    let mut command = Command::new(executable);
    command.args(args).env("PATH", current_path);
    command
}

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

        // Update system PATH environment variable for        #[cfg(target_os = "windows")]
        if let Err(e) = self.update_system_path() {
            warn!("Failed to update system PATH: {}", e);
        }

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
                        .args([
                            "/i",
                            local_path.to_str().ok_or_else(|| {
                                InstallerError::Script(
                                    "installer path is not valid UTF-8".to_string(),
                                )
                            })?,
                            "/quiet",
                            "/norestart",
                        ])
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
        // Convention: bundle/dependencies/<dep-name>/<platform>/<arch>/<file>
        // First try with "dependencies" subdirectory (bundle creation format)
        let mut platform_dir = bundle_dir
            .join("dependencies")
            .join(dep_name.to_lowercase())
            .join(self.config.platform.os.to_string().to_lowercase())
            .join(self.config.platform.arch.to_string().to_lowercase());

        if !platform_dir.exists() {
            // Try without arch distinction
            platform_dir = bundle_dir
                .join("dependencies")
                .join(dep_name.to_lowercase())
                .join(self.config.platform.os.to_string().to_lowercase());

            if !platform_dir.exists() {
                // Fallback: try without "dependencies" subdirectory (legacy format)
                platform_dir = bundle_dir
                    .join(dep_name.to_lowercase())
                    .join(self.config.platform.os.to_string().to_lowercase())
                    .join(self.config.platform.arch.to_string().to_lowercase());

                if !platform_dir.exists() {
                    // Try without arch distinction (legacy format)
                    platform_dir = bundle_dir
                        .join(dep_name.to_lowercase())
                        .join(self.config.platform.os.to_string().to_lowercase());

                    if !platform_dir.exists() {
                        return Err(InstallerError::DependencyNotFound(format!(
                            "Dependency '{}' not found in bundle: {}",
                            dep_name,
                            bundle_dir.display()
                        )));
                    }
                }
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
                    installer_path.to_str().ok_or_else(|| {
                        InstallerError::Script("installer path is not valid UTF-8".to_string())
                    })?,
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
            // Git for Windows uses Inno Setup which requires /SILENT or /VERYSILENT
            // NSIS installers use /S
            // Try different silent flags based on installer type
            let installer_name = installer_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();

            let status = if installer_name.contains("git") && installer_name.contains("64-bit") {
                // Git for Windows (Inno Setup) - use /SILENT
                info!("Detected Git for Windows installer, using /SILENT flag");
                Command::new(&installer_path)
                    .args(["/SILENT", "/NORESTART"])
                    .status()
            } else {
                // Try NSIS style first (/S), then Inno Setup style (/SILENT)
                Command::new(&installer_path)
                    .args(["/S"])
                    .status()
                    .or_else(|_| {
                        Command::new(&installer_path)
                            .args(["/SILENT", "/NORESTART"])
                            .status()
                    })
                    .or_else(|_| {
                        // Try without args (some installers don't support silent)
                        Command::new(&installer_path).status()
                    })
            };

            let status = status.map_err(|e| {
                InstallerError::Script(format!("Failed to run EXE installer: {}", e))
            })?;

            if !status.success() {
                return Err(InstallerError::InstallationFailed(format!(
                    "Installer failed for {}",
                    dep_name
                )));
            }

            // Special handling for Git: add to PATH for current session
            if dep_name.to_lowercase() == "git" {
                // Git for Windows installs to C:\Program Files\Git by default
                let git_cmd_dir = PathBuf::from("C:\\Program Files\\Git\\cmd");
                let git_bin_dir = PathBuf::from("C:\\Program Files\\Git\\bin");

                let current_path = std::env::var("PATH").unwrap_or_default();
                let mut new_path = current_path.clone();

                if git_cmd_dir.exists() {
                    new_path = if self.config.platform.is_windows() {
                        format!("{};{}", git_cmd_dir.display(), new_path)
                    } else {
                        format!("{}:{}", git_cmd_dir.display(), new_path)
                    };
                    info!("[git] Added Git cmd to PATH: {}", git_cmd_dir.display());
                }

                if git_bin_dir.exists() {
                    new_path = if self.config.platform.is_windows() {
                        format!("{};{}", git_bin_dir.display(), new_path)
                    } else {
                        format!("{}:{}", git_bin_dir.display(), new_path)
                    };
                    info!("[git] Added Git bin to PATH: {}", git_bin_dir.display());
                }

                std::env::set_var("PATH", &new_path);
                info!("[git] Updated PATH for current session");

                // Verify git is now accessible
                if let Ok(output) = Command::new("git").arg("--version").output() {
                    info!(
                        "[git] Git version: {}",
                        String::from_utf8_lossy(&output.stdout).trim()
                    );
                }
            }
        } else if extension == "zip" {
            // Zip archive - extract to a temp location and run any installer inside
            let temp_dir = std::env::temp_dir().join(format!("burncloud-{}", dep_name));
            self.extract_zip(&installer_path, &temp_dir)?;

            // Look for an installer inside
            for entry in std::fs::read_dir(&temp_dir)
                .map_err(|e| InstallerError::Script(format!("Failed to read temp dir: {}", e)))?
            {
                let entry = entry.map_err(|e| {
                    InstallerError::Script(format!("Failed to read dir entry: {}", e))
                })?;
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    if ext == "exe" || ext == "msi" {
                        // Run the installer
                        let _ = Command::new(&path).status();
                        break;
                    }
                }
            }

            // Special handling for Node.js dependency
            if dep_name.to_lowercase() == "node.js" {
                // Install Node.js directly from bundle
                info!(
                    "[bundle] Installing Node.js from bundle, platform_dir: {}",
                    platform_dir.display()
                );
                let installed = self.install_nodejs_from_bundle(&platform_dir)?;
                info!("[bundle] Node.js installation result: {}", installed);
                if !installed {
                    warn!("[bundle] Node.js was not installed from bundle, falling back may be needed");
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
                    if let Some(_auto_install) = &dep.auto_install {
                        info!("Auto-installing dependency: {}", dep.name);

                        // If bundle directory is set, install from bundle
                        if self.config.bundle_dir.is_some() {
                            info!("Installing {} from bundle...", dep.name);
                            self.install_dependency_from_bundle(&dep.name).await?;
                        } else {
                            // Otherwise use the auto_install method
                            self.install_dependency(dep, _auto_install).await?;
                        }
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
        use std::time::Instant;
        let start = Instant::now();
        info!("[npm] Installing {} via npm...", software.name);

        let package_spec = match version {
            Some(v) => format!("{}@{}", package, v),
            None => package.to_string(),
        };

        let custom_mirror = std::env::var("NPM_MIRROR").ok();
        let use_region_default = custom_mirror
            .as_deref()
            .map(str::trim)
            .map(str::is_empty)
            .unwrap_or(true);

        // Preserve the existing region selection exactly when NPM_MIRROR is
        // missing or blank. An invalid non-blank custom value fails closed.
        let in_china = use_region_default
            && (std::env::var("LANG")
                .map(|language| language.contains("zh") || language.contains("CN"))
                .unwrap_or(false)
                || std::env::var("LC_ALL")
                    .map(|language| language.contains("zh") || language.contains("CN"))
                    .unwrap_or(false));
        let registry = resolve_npm_registry(custom_mirror.as_deref(), in_china)?;

        if let Some(registry) = registry.as_ref() {
            info!("[npm] Custom registry enabled: {}", registry.safe_origin());
        }

        let mut args = vec!["install".to_string()];
        if global {
            args.push("-g".to_string());
        }
        args.push(package_spec);

        if let Some(registry) = registry.as_ref() {
            args.push(format!("--registry={}", registry.as_str()));
        }

        info!("[npm] Starting npm install");

        // Get current PATH to pass to child process (includes Node.js path if installed from bundle)
        let current_path = std::env::var("PATH").unwrap_or_default();
        let result = npm_install_command(&self.config.platform, &args, &current_path).status();

        let elapsed = start.elapsed();
        info!(
            "[npm] npm install completed in {:.2}s",
            elapsed.as_secs_f64()
        );

        match result {
            Ok(status) if status.success() => {
                info!("[npm] Successfully installed {} via npm", software.name);
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
        use std::time::Instant;
        let _start = Instant::now();
        info!("[bundle] Installing {} from bundle...", software.name);
        info!("[bundle] Bundle directory: {}", bundle_dir.display());

        // Look for the tarball in bundle/software/<id>/npm-package/
        let npm_dir = bundle_dir
            .join("software")
            .join(&software.id)
            .join("npm-package");

        info!("[bundle] Looking for npm package in: {}", npm_dir.display());

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

        info!(
            "[bundle] Installing from tarball: {}",
            tarball_path.display()
        );

        // Get tarball size for progress info
        if let Ok(metadata) = std::fs::metadata(&tarball_path) {
            info!(
                "[bundle] Tarball size: {} bytes ({:.2} MB)",
                metadata.len(),
                metadata.len() as f64 / 1024.0 / 1024.0
            );
        }

        // Install from tarball
        let tarball_str = tarball_path.to_string_lossy();
        let mut args = vec!["install"];
        if global {
            args.push("-g");
        }
        args.push(&tarball_str);

        info!("[bundle] Starting npm install from local bundle");

        let npm_start = Instant::now();

        // Get current PATH to pass to child process (includes Node.js path if installed from bundle)
        let current_path = std::env::var("PATH").unwrap_or_default();
        info!("[bundle] Current PATH: {}", current_path);

        // Check if npm is accessible without involving a command shell.
        let npm_check = if self.config.platform.is_windows() {
            Command::new("where").arg("npm.cmd").output()
        } else {
            Command::new("which").arg("npm").output()
        };

        match npm_check {
            Ok(output) => {
                if output.status.success() {
                    info!(
                        "[bundle] npm found at: {}",
                        String::from_utf8_lossy(&output.stdout).trim()
                    );
                } else {
                    warn!("[bundle] npm not found in PATH!");
                    warn!(
                        "[bundle] stderr: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                }
            }
            Err(e) => {
                warn!("[bundle] Failed to check npm location: {}", e);
            }
        }

        let args: Vec<String> = args.into_iter().map(str::to_string).collect();
        let result = npm_install_command(&self.config.platform, &args, &current_path).output();

        let npm_elapsed = npm_start.elapsed();
        info!(
            "[bundle] npm install completed in {:.2}s",
            npm_elapsed.as_secs_f64()
        );

        match result {
            Ok(output) if output.status.success() => {
                info!(
                    "[bundle] npm stdout: {}",
                    String::from_utf8_lossy(&output.stdout)
                );
                info!("Successfully installed {} from bundle", software.name);
                Ok(())
            }
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                warn!("[bundle] npm stdout: {}", stdout);
                warn!("[bundle] npm stderr: {}", stderr);
                Err(InstallerError::InstallationFailed(format!(
                    "npm install from bundle exited with code: {}\nstdout: {}\nstderr: {}",
                    output.status.code().unwrap_or(-1),
                    stdout,
                    stderr
                )))
            }
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
        clone_args.push(clone_dir.to_str().ok_or_else(|| {
            InstallerError::Script("clone dir path is not valid UTF-8".to_string())
        })?);

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

    /// Install Node.js directly from bundle (offline mode)
    /// Returns true if Node.js was installed from bundle, false if fallback needed
    fn install_nodejs_from_bundle(&self, platform_dir: &Path) -> InstallerResult<bool> {
        use std::time::Instant;
        let start = Instant::now();
        info!(
            "[nodejs] Looking for offline Node.js package in: {}",
            platform_dir.display()
        );
        info!(
            "[nodejs] Platform directory exists: {}",
            platform_dir.exists()
        );

        // List all files in the directory for debugging
        if let Ok(entries) = std::fs::read_dir(platform_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                info!(
                    "[nodejs]   Found file: {}",
                    entry.file_name().to_string_lossy()
                );
            }
        }

        // Look for Node.js official package (node-v*.zip or node-v*.tar.gz)
        let nodejs_package = std::fs::read_dir(platform_dir)
            .map_err(|e| InstallerError::FileSystem(e.to_string()))?
            .filter_map(|e| e.ok())
            .find(|e| {
                let name = e.file_name().to_string_lossy().to_lowercase();
                let matches = name.starts_with("node-v")
                    && (name.ends_with(".zip") || name.ends_with(".tar.gz"));
                info!("[nodejs]   Checking file: {} -> matches: {}", name, matches);
                matches
            });

        let Some(nodejs_entry) = nodejs_package else {
            warn!("[nodejs] No offline Node.js package found in bundle");
            return Ok(false);
        };

        let nodejs_path = nodejs_entry.path();

        // Get file size
        if let Ok(metadata) = std::fs::metadata(&nodejs_path) {
            info!(
                "[nodejs] Found offline Node.js package: {} ({:.2} MB)",
                nodejs_path.display(),
                metadata.len() as f64 / 1024.0 / 1024.0
            );
        } else {
            info!(
                "[nodejs] Found offline Node.js package: {}",
                nodejs_path.display()
            );
        }

        // Determine installation directory
        // Use %LOCALAPPDATA%\burncloud\nodejs on Windows
        let nodejs_install_dir = if self.config.platform.is_windows() {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("./apps"))
                .join("burncloud")
                .join("nodejs")
        } else {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("./apps"))
                .join("burncloud")
                .join("nodejs")
        };

        info!(
            "[nodejs] Installation directory: {}",
            nodejs_install_dir.display()
        );

        // Create installation directory
        std::fs::create_dir_all(&nodejs_install_dir).map_err(|e| {
            InstallerError::FileSystem(format!("Failed to create Node.js directory: {}", e))
        })?;

        // Extract Node.js package
        let extract_start = Instant::now();
        let extension = nodejs_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        info!(
            "[nodejs] Extracting Node.js package (format: {})...",
            extension
        );

        if extension == "zip" {
            self.extract_zip(&nodejs_path, &nodejs_install_dir)?;
        } else if extension == "gz" {
            // tar.gz extraction
            let status = Command::new("tar")
                .args([
                    "-xzf",
                    nodejs_path.to_str().ok_or_else(|| {
                        InstallerError::Script("nodejs path is not valid UTF-8".to_string())
                    })?,
                    "-C",
                    nodejs_install_dir.to_str().ok_or_else(|| {
                        InstallerError::Script(
                            "nodejs install dir path is not valid UTF-8".to_string(),
                        )
                    })?,
                ])
                .status()
                .map_err(|e| InstallerError::Script(format!("Failed to extract tar.gz: {}", e)))?;

            if !status.success() {
                return Err(InstallerError::InstallationFailed(
                    "Failed to extract Node.js tar.gz".to_string(),
                ));
            }
        }

        let extract_elapsed = extract_start.elapsed();
        info!(
            "[nodejs] Extraction completed in {:.2}s",
            extract_elapsed.as_secs_f64()
        );

        // Find the extracted Node.js directory (usually node-v22.14.0-win-x64)
        let extracted_dir = std::fs::read_dir(&nodejs_install_dir)
            .map_err(|e| InstallerError::FileSystem(e.to_string()))?
            .filter_map(|e| e.ok())
            .find(|e| e.file_name().to_string_lossy().starts_with("node-v"));

        if let Some(extracted) = extracted_dir {
            let extracted_path = extracted.path();
            info!(
                "[nodejs] Node.js extracted to: {}",
                extracted_path.display()
            );

            // Add to system PATH for current session
            let nodejs_bin = extracted_path.clone();
            let current_path = std::env::var("PATH").unwrap_or_default();
            let new_path = if self.config.platform.is_windows() {
                format!("{};{}", nodejs_bin.display(), current_path)
            } else {
                format!("{}:{}", nodejs_bin.display(), current_path)
            };
            std::env::set_var("PATH", &new_path);
            info!("Added Node.js to PATH: {}", nodejs_bin.display());

            // Verify installation
            let node_exe = if self.config.platform.is_windows() {
                nodejs_bin.join("node.exe")
            } else {
                nodejs_bin.join("bin").join("node")
            };

            if node_exe.exists() {
                info!("Node.js installation verified: {}", node_exe.display());

                // Test node --version
                if let Ok(output) = Command::new(&node_exe).arg("--version").output() {
                    let version = String::from_utf8_lossy(&output.stdout);
                    info!("[nodejs] Node.js version: {}", version.trim());
                }
            } else {
                warn!(
                    "[nodejs] Node.js executable not found at expected location: {}",
                    node_exe.display()
                );
            }
        }

        let total_elapsed = start.elapsed();
        info!(
            "[nodejs] Node.js installed successfully from bundle in {:.2}s",
            total_elapsed.as_secs_f64()
        );
        Ok(true)
    }

    /// Update system PATH environment variable to include installed tools
    ///
    /// On Windows, this uses `setx` to permanently add paths to user's PATH.
    /// This allows Node.js, Git, and npm global packages (like openclaw) to be
    /// accessible from any new command prompt.
    #[cfg(target_os = "windows")]
    fn update_system_path(&self) -> InstallerResult<()> {
        use std::process::Command;

        let mut paths_to_add = Vec::new();

        // Node.js path - installed to %LOCALAPPDATA%\burncloud\nodejs\node-v24.14.0-win-x64
        let nodejs_base = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("./apps"))
            .join("burncloud")
            .join("nodejs");

        info!("[env] Looking for Node.js in: {}", nodejs_base.display());

        // List all directories in nodejs_base for debugging
        if nodejs_base.exists() {
            if let Ok(entries) = std::fs::read_dir(&nodejs_base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    info!("[env]   Found in nodejs dir: {}", path.display());
                }
            }
        }

        if nodejs_base.exists() {
            // Find the actual Node.js directory (e.g., node-v24.14.0-win-x64)
            if let Ok(entries) = std::fs::read_dir(&nodejs_base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let name = path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    info!("[env]   Checking: {} (is_dir: {})", name, path.is_dir());
                    if path.is_dir() && name.starts_with("node-v") {
                        paths_to_add.push(path.to_string_lossy().to_string());
                        info!("[env] Adding Node.js to PATH: {}", path.display());
                        break;
                    }
                }
            }
        } else {
            warn!(
                "[env] Node.js directory does not exist: {}",
                nodejs_base.display()
            );
        }

        // Git paths - installed to C:\Program Files\Git by Git for Windows installer
        let git_install_dir = PathBuf::from("C:\\Program Files\\Git");
        info!("[env] Looking for Git in: {}", git_install_dir.display());
        if git_install_dir.exists() {
            let git_cmd = git_install_dir.join("cmd");
            let git_bin = git_install_dir.join("bin");

            if git_cmd.exists() {
                paths_to_add.push(git_cmd.to_string_lossy().to_string());
                info!("[env] Adding Git/cmd to PATH: {}", git_cmd.display());
            }
            if git_bin.exists() {
                paths_to_add.push(git_bin.to_string_lossy().to_string());
                info!("[env] Adding Git/bin to PATH: {}", git_bin.display());
            }
        } else {
            warn!(
                "[env] Git installation directory does not exist: {}",
                git_install_dir.display()
            );
        }

        // npm global bin path (for openclaw and other global packages)
        // npm stores global packages in %APPDATA%\npm on Windows
        if let Some(appdata) = dirs::config_dir() {
            let npm_global = appdata.join("npm");
            // Always add npm global path, even if it doesn't exist yet
            // It will be created when npm installs global packages
            paths_to_add.push(npm_global.to_string_lossy().to_string());
            info!(
                "[env] Adding npm global to PATH: {} (exists: {})",
                npm_global.display(),
                npm_global.exists()
            );
        }

        if paths_to_add.is_empty() {
            warn!("[env] No paths to add to PATH");
            return Ok(());
        }

        // Get current user PATH
        let current_path = std::env::var("PATH").unwrap_or_default();
        info!("[env] Current PATH length: {} chars", current_path.len());

        // Check which paths are not already in PATH
        let current_path_lower = current_path.to_lowercase();
        let new_paths: Vec<String> = paths_to_add
            .into_iter()
            .filter(|p| {
                // Check if path is not already in PATH (case-insensitive on Windows)
                let already_exists = current_path_lower.contains(&p.to_lowercase());
                if already_exists {
                    info!("[env] Path already in PATH: {}", p);
                }
                !already_exists
            })
            .collect();

        if new_paths.is_empty() {
            info!("[env] All paths already in PATH");
            return Ok(());
        }

        info!("[env] New paths to add: {:?}", new_paths);

        // Use setx to permanently add paths to user PATH
        // We need to get the current user PATH from registry, add new paths, and set it back
        let new_path_value = if current_path.is_empty() {
            new_paths.join(";")
        } else {
            format!("{};{}", current_path, new_paths.join(";"))
        };

        info!(
            "[env] Setting new PATH with {} additional entries",
            new_paths.len()
        );

        // Use setx to set the user PATH permanently
        let output = Command::new("setx")
            .args(["PATH", &new_path_value])
            .output()
            .map_err(|e| {
                InstallerError::InstallationFailed(format!("Failed to run setx: {}", e))
            })?;

        if output.status.success() {
            info!("[env] Successfully updated system PATH");
            info!("[env] Please restart your terminal for changes to take effect");
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!("[env] setx failed: {}", stderr);
            Err(InstallerError::InstallationFailed(format!(
                "Failed to update PATH: {}",
                stderr
            )))
        }
    }

    #[cfg(not(target_os = "windows"))]
    fn update_system_path(&self) -> InstallerResult<()> {
        // On non-Windows, users typically manage PATH themselves or via shell config
        info!("[env] PATH update not implemented for this platform");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{Arch, OS};

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

    #[test]
    fn npm_mirror_missing_and_blank_preserve_region_fallback() {
        assert!(resolve_npm_registry(None, false)
            .unwrap_or_else(|error| panic!("missing mirror must be accepted: {error}"))
            .is_none());
        assert!(resolve_npm_registry(Some("  \t "), false)
            .unwrap_or_else(|error| panic!("blank mirror must be accepted: {error}"))
            .is_none());

        let registry = resolve_npm_registry(Some(""), true)
            .unwrap_or_else(|error| panic!("China fallback must be valid: {error}"))
            .unwrap_or_else(|| panic!("China fallback must select a registry"));
        assert_eq!(registry.as_str(), "https://registry.npmmirror.com/");
    }

    #[test]
    fn npm_mirror_accepts_valid_https_url_and_redacts_log_label() {
        let registry = NpmRegistry::parse("https://registry.example.com/npm?token=test-marker")
            .unwrap_or_else(|error| panic!("valid HTTPS mirror rejected: {error}"));
        assert!(NpmRegistry::parse("http://localhost:4873/npm").is_ok());

        assert_eq!(
            registry.as_str(),
            "https://registry.example.com/npm?token=test-marker"
        );
        assert_eq!(registry.safe_origin(), "https://registry.example.com");
        assert!(!registry.safe_origin().contains("test-marker"));
    }

    #[test]
    fn npm_mirror_rejects_shell_metacharacters_without_echoing_input() {
        let marker = "must-not-appear";
        for metacharacter in [';', '|', '&', '`', '$', '<', '>'] {
            let value = format!("https://registry.example.com/{marker}{metacharacter}echo");
            let error = NpmRegistry::parse(&value)
                .err()
                .unwrap_or_else(|| panic!("shell metacharacter must be rejected"));

            let rendered = error.to_string();
            assert!(!rendered.contains(marker));
            assert!(!rendered.contains(&value));
        }
    }

    #[test]
    fn npm_mirror_rejects_userinfo_without_echoing_credentials() {
        let password = "must-not-appear";
        let value = format!("https://user:{password}@registry.example.com");
        let error = NpmRegistry::parse(&value)
            .err()
            .unwrap_or_else(|| panic!("URL userinfo must be rejected"));

        assert!(!error.to_string().contains(password));
    }

    #[test]
    fn npm_mirror_rejects_empty_host_and_unapproved_scheme() {
        for value in [
            "https://",
            "file:///tmp/registry",
            "ftp://registry.example.com",
        ] {
            assert!(NpmRegistry::parse(value).is_err(), "accepted {value}");
        }
    }

    #[test]
    fn npm_command_never_uses_a_shell_and_keeps_registry_as_one_argument() {
        let registry = NpmRegistry::parse("https://registry.example.com/npm")
            .unwrap_or_else(|error| panic!("test mirror must be valid: {error}"));
        let args = vec![
            "install".to_string(),
            "-g".to_string(),
            "example-package@1.0.0".to_string(),
            format!("--registry={}", registry.as_str()),
        ];

        for (os, expected_program) in [(OS::Linux, "npm"), (OS::Windows, "npm.cmd")] {
            let platform = Platform {
                os,
                arch: Arch::X64,
            };
            let command = npm_install_command(&platform, &args, "test-path");
            let actual_args: Vec<String> = command
                .get_args()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect();

            assert_eq!(command.get_program(), expected_program);
            assert_eq!(actual_args, args);
            assert!(!actual_args.iter().any(|arg| arg == "-c" || arg == "/C"));
        }
    }

    // ---------------------------------------------------------------------------------------
    // archive extraction: path traversal, corrupt input, and what comes out
    // ---------------------------------------------------------------------------------------
    //
    // The plan lists "损坏压缩包/路径穿越" for this crate, and none of the existing tests cover either. These
    // live here rather than in `tests/` because `extract_zip` is private and takes `&self`; the crate's other
    // tests are inline for the same reason.
    //
    // **Nothing here touches a real system directory.** The only directory ever passed to `extract_zip` is a
    // temporary one created by the test, which the plan requires ("不得安装到开发者真实系统目录").

    /// Build **raw** zip bytes from an explicit list of entry names.
    ///
    /// `zip::write::ZipWriter` cannot express what these tests need: it normalises a name on the way in, so
    /// `../escape.txt` is stored as `escape.txt` and the property under test never reaches the extractor. This
    /// writes the local file header, the data and the central directory by hand so the stored name is
    /// byte-for-byte what the test asks for.
    ///
    /// Only stored (method 0) entries of known length are produced, which keeps the records small and removes
    /// any dependency on a compressor.
    fn raw_zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();

        for (name, data) in entries {
            let name_bytes = name.as_bytes();
            let crc = crc32(data);
            let offset = out.len() as u32;
            let size = data.len() as u32;

            // Local file header
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&20u16.to_le_bytes()); // version needed
            out.extend_from_slice(&0u16.to_le_bytes()); // flags
            out.extend_from_slice(&0u16.to_le_bytes()); // method: stored
            out.extend_from_slice(&0u16.to_le_bytes()); // mod time
            out.extend_from_slice(&0u16.to_le_bytes()); // mod date
            out.extend_from_slice(&crc.to_le_bytes());
            out.extend_from_slice(&size.to_le_bytes()); // compressed
            out.extend_from_slice(&size.to_le_bytes()); // uncompressed
            out.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            out.extend_from_slice(&0u16.to_le_bytes()); // extra length
            out.extend_from_slice(name_bytes);
            out.extend_from_slice(data);

            // Central directory record
            central.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            central.extend_from_slice(&20u16.to_le_bytes()); // version made by
            central.extend_from_slice(&20u16.to_le_bytes()); // version needed
            central.extend_from_slice(&0u16.to_le_bytes()); // flags
            central.extend_from_slice(&0u16.to_le_bytes()); // method
            central.extend_from_slice(&0u16.to_le_bytes()); // mod time
            central.extend_from_slice(&0u16.to_le_bytes()); // mod date
            central.extend_from_slice(&crc.to_le_bytes());
            central.extend_from_slice(&size.to_le_bytes());
            central.extend_from_slice(&size.to_le_bytes());
            central.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
            central.extend_from_slice(&0u16.to_le_bytes()); // extra
            central.extend_from_slice(&0u16.to_le_bytes()); // comment
            central.extend_from_slice(&0u16.to_le_bytes()); // disk
            central.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            central.extend_from_slice(&0u32.to_le_bytes()); // external attrs
            central.extend_from_slice(&offset.to_le_bytes());
            central.extend_from_slice(name_bytes);
        }

        let central_offset = out.len() as u32;
        let central_size = central.len() as u32;
        out.extend_from_slice(&central);

        // End of central directory
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // disk
        out.extend_from_slice(&0u16.to_le_bytes()); // disk with central
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&central_size.to_le_bytes());
        out.extend_from_slice(&central_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment length
        out
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc = 0xFFFF_FFFFu32;
        for byte in data {
            crc ^= *byte as u32;
            for _ in 0..8 {
                let mask = (crc & 1).wrapping_neg();
                crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            }
        }
        !crc
    }

    /// A unique temporary directory for one test, removed when the returned guard is dropped.
    ///
    /// **The cleanup is a `Drop`, not a line at the end of the test.** An earlier version called
    /// `std::fs::remove_dir_all` as the last statement, and the first attempt at measuring whether the zip-slip
    /// guard was load-bearing left a `bc_installer_zip_slip_*` directory behind: removing the guard makes the
    /// assertion panic, which skips everything after it. That is the same residue pattern seen repeatedly
    /// elsewhere in this suite, and a guard is the only form of cleanup that survives a panic.
    struct TempRoot(PathBuf);

    impl TempRoot {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "bc_installer_{}_{}_{}",
                tag,
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .expect("the clock is after the epoch")
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).expect("the temporary directory must be creatable");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempRoot {
        fn drop(&mut self) {
            // Nothing is asserted here: a `Drop` that panics during unwinding aborts the process. A failure to
            // remove is reported and ignored, so one leftover directory cannot crash the run.
            if let Err(e) = std::fs::remove_dir_all(&self.0) {
                if self.0.exists() {
                    eprintln!("could not remove {}: {e}", self.0.display());
                }
            }
        }
    }

    /// An installer whose directories all point inside `root`, so nothing can reach a real location.
    fn installer_in(root: &Path) -> Installer {
        let config = InstallerConfig::new()
            .with_download_dir(root.join("downloads"))
            .with_install_dir(root.join("install"))
            .with_bundle_dir(None);
        Installer::new(config)
    }

    fn write_zip(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).expect("the archive must be writable");
        path
    }

    #[test]
    fn a_traversing_zip_entry_is_skipped_and_never_escapes_the_target() {
        // **The zip-slip case.** An entry named `../escaped.txt` must not be written outside the target
        // directory. `extract_zip` reaches this through `file.enclosed_name()`, which returns `None` for a name
        // that would escape, and the loop `continue`s on `None` -- so the guard is a **library behaviour plus an
        // explicit skip**, and this test pins both: the file is not created, and the archive still extracts its
        // legitimate entries rather than failing outright.
        let root = TempRoot::new("zip_slip");
        let target = root.path().join("install").join("app");
        std::fs::create_dir_all(&target).expect("the target directory must be creatable");
        let installer = installer_in(root.path());

        let bytes = raw_zip(&[
            ("../escaped.txt", b"should never be written"),
            ("../../also-escaped.txt", b"nor this"),
            ("/absolute.txt", b"nor an absolute name"),
            ("safe.txt", b"this one is fine"),
            ("nested/deep.txt", b"and this one"),
        ]);
        let zip_path = write_zip(root.path(), "hostile.zip", &bytes);

        installer
            .extract_zip(&zip_path, &target)
            .expect("a hostile entry is skipped, not fatal -- the archive still extracts");

        // The escape targets, checked by name at every level above the target.
        let escaped = root.path().join("escaped.txt");
        let escaped_two = root.path().join("also-escaped.txt");
        let absolute = PathBuf::from("/absolute.txt");
        println!(
            "escaped.txt exists: {}, two levels up: {}, absolute: {}",
            escaped.exists(),
            escaped_two.exists(),
            absolute.exists()
        );
        assert!(!escaped.exists(), "`../escaped.txt` must not be written");
        assert!(
            !escaped_two.exists(),
            "`../../also-escaped.txt` must not be written"
        );
        assert!(
            !absolute.exists() || !absolute.starts_with("/absolute.txt"),
            "an absolute entry name must not be honoured"
        );

        // And the harmless entries are there, so the skip did not become "extract nothing".
        assert!(
            target.join("safe.txt").exists(),
            "a plain entry is extracted"
        );
        assert!(
            target.join("nested").join("deep.txt").exists(),
            "and a nested one"
        );
    }

    #[test]
    fn a_traversing_entry_does_not_create_directories_outside_the_target() {
        // The directory half of the same problem: an entry named `../dir/` must not create `dir` outside the
        // target. A traversal guard that only covers files would miss this, because the directory branch calls
        // `create_dir_all` before any file is written.
        let root = TempRoot::new("zip_slip_dir");
        let target = root.path().join("install").join("app");
        std::fs::create_dir_all(&target).expect("the target directory must be creatable");
        let installer = installer_in(root.path());

        let bytes = raw_zip(&[
            ("../outside-dir/", b""),
            (
                "../outside-dir/inside.txt",
                b"written through the directory entry",
            ),
            ("inside.txt", b"fine"),
        ]);
        let zip_path = write_zip(root.path(), "hostile-dir.zip", &bytes);

        installer
            .extract_zip(&zip_path, &target)
            .expect("the archive extracts, skipping the escaping entry");

        let outside = root.path().join("outside-dir");
        println!("outside-dir exists: {}", outside.exists());
        assert!(
            !outside.exists(),
            "a directory entry with `..` must not create a directory outside the target"
        );
        assert!(
            target.join("inside.txt").exists(),
            "and the safe entry is written"
        );
    }

    #[test]
    fn a_corrupt_archive_is_an_error_rather_than_a_partial_extraction() {
        // "损坏压缩包". A file that is not a zip at all must be reported, not silently treated as an empty
        // archive -- a caller that carried on would report a successful install with nothing installed.
        let root = TempRoot::new("corrupt");
        let target = root.path().join("install").join("app");
        let installer = installer_in(root.path());

        for (label, bytes) in [
            ("empty", Vec::new()),
            ("plain text", b"this is not a zip file at all".to_vec()),
            ("truncated header", vec![0x50, 0x4b, 0x03, 0x04]),
        ] {
            let zip_path = write_zip(
                root.path(),
                &format!("{}.zip", label.replace(' ', "_")),
                &bytes,
            );
            let result = installer.extract_zip(&zip_path, &target);
            match &result {
                Ok(()) => panic!("a {label} file was accepted as an archive"),
                Err(e) => println!("{label} -> {e}"),
            }
            assert!(result.is_err(), "a {label} file must be rejected");
        }

        // A file that does not exist is also an error, and is a different one from a corrupt archive.
        let missing = installer.extract_zip(&root.path().join("no-such.zip"), &target);
        assert!(missing.is_err(), "a missing archive must be rejected");

        // Nothing was created by any of the failures, so a failed extraction leaves no partial tree behind.
        let leftovers: Vec<_> = std::fs::read_dir(&target)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name())
                    .collect()
            })
            .unwrap_or_default();
        println!("leftovers in the target after three failures: {leftovers:?}");
        assert!(
            leftovers.is_empty(),
            "a rejected archive must not leave files in the target directory"
        );
    }

    #[test]
    fn extraction_preserves_file_contents_and_creates_the_target() {
        // The successful path, which the negative tests above need as a control: without it, "the file is not
        // there" would pass on an extractor that never writes anything.
        let root = TempRoot::new("extract_ok");
        let target = root.path().join("install").join("app");
        // The target deliberately does not exist yet, so this also covers its creation.
        assert!(!target.exists(), "the target starts absent");

        let installer = installer_in(root.path());
        let payload = b"node --version\n";
        let zip_path = write_zip(root.path(), "ok.zip", &raw_zip(&[("bin/tool.sh", payload)]));

        installer
            .extract_zip(&zip_path, &target)
            .expect("a well-formed archive extracts");

        assert!(target.exists(), "the target directory was created");
        let written =
            std::fs::read(target.join("bin").join("tool.sh")).expect("the file is readable");
        assert_eq!(
            written, payload,
            "and its contents are byte-for-byte what was stored"
        );
    }

    #[tokio::test]
    async fn installation_status_is_read_from_the_install_directory_alone() {
        // "已安装版本处理". `check_status` reports `Installed` when `<install_dir>/<id>/.installed` exists and
        // `NotInstalled` otherwise, so the question is whether the answer follows the configured directory rather
        // than the real system. This points the installer at a temporary root, which is the property the plan
        // cares about.
        let root = TempRoot::new("status");
        let installer = installer_in(root.path());

        let before = installer
            .check_status("openclaw")
            .await
            .expect("a known software reports a status");
        println!("before the marker: {before:?}");
        assert_eq!(
            before,
            InstallStatus::NotInstalled,
            "nothing is installed in a fresh root"
        );

        // The marker is the whole signal.
        let marker_dir = root.path().join("install").join("openclaw");
        std::fs::create_dir_all(&marker_dir).expect("the marker directory must be creatable");
        std::fs::write(marker_dir.join(".installed"), b"").expect("the marker must be writable");

        let after = installer
            .check_status("openclaw")
            .await
            .expect("the status is readable");
        println!("after the marker: {after:?}");
        assert_eq!(
            after,
            InstallStatus::Installed,
            "the marker alone decides, and it is read from the configured directory"
        );

        // Idempotence: asking again gives the same answer, so the check does not consume or move anything.
        let again = installer.check_status("openclaw").await.expect("readable");
        assert_eq!(
            again,
            InstallStatus::Installed,
            "and asking twice changes nothing"
        );

        // An unknown id is an error rather than a status, which is a different answer from NotInstalled and is
        // worth pinning because the two would otherwise look alike to a caller.
        assert!(
            installer.check_status("no-such-software").await.is_err(),
            "an unknown software id must be an error, not NotInstalled"
        );
    }

    #[test]
    fn the_nodejs_url_names_the_platform_the_architecture_and_the_version() {
        // "OS/架构选择" and "版本与路径解析". The URL is assembled from a version and a platform pair, and the
        // extension differs by OS, so a mistake here downloads a binary for the wrong system -- which fails
        // later and confusingly.
        for (os, arch, expected) in [
            (
                OS::Windows,
                Arch::X64,
                "https://nodejs.org/dist/v22.14.0/node-v22.14.0-win-x64.zip",
            ),
            (
                OS::Windows,
                Arch::ARM64,
                "https://nodejs.org/dist/v22.14.0/node-v22.14.0-win-arm64.zip",
            ),
            (
                OS::MacOS,
                Arch::ARM64,
                "https://nodejs.org/dist/v22.14.0/node-v22.14.0-darwin-arm64.tar.gz",
            ),
            (
                OS::Linux,
                Arch::X64,
                "https://nodejs.org/dist/v22.14.0/node-v22.14.0-linux-x64.tar.gz",
            ),
        ] {
            let url = crate::bundle::get_nodejs_download_url("22.14.0", os, arch);
            println!("{os:?}/{arch:?} -> {url:?}");
            assert_eq!(
                url.as_deref(),
                Some(expected),
                "the URL must name the platform, the architecture and the version"
            );
        }

        // The version appears twice -- in the path and in the file name -- so a version that is only
        // substituted once would produce a URL that 404s.
        let url = crate::bundle::get_nodejs_download_url("1.2.3", OS::Linux, Arch::X64)
            .expect("linux/x64 is supported");
        assert_eq!(
            url.matches("1.2.3").count(),
            2,
            "the version appears in both positions: {url}"
        );

        // A platform pair the template has no mapping for returns `None` rather than a guessed URL. macOS on
        // 32-bit x86 is such a pair; the template covers ARM64 and x64 only.
        assert_eq!(
            crate::bundle::get_nodejs_download_url("22.14.0", OS::MacOS, Arch::X86),
            None,
            "an unmapped platform pair must be None rather than a URL that cannot exist"
        );
    }
}
