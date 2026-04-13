//! Bundle creation and verification for offline installation

use chrono::Utc;
use log::{info, warn};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use crate::error::{InstallerError, InstallerResult};
use crate::npm::NpmInstaller;
use crate::platform::{Arch, Platform, OS};
use crate::registry::get_software;
use crate::software::{GitHubRelease, InstallMethod};

/// Node.js version to bundle (LTS)
pub const NODEJS_LTS_VERSION: &str = "22.14.0";

/// Node.js Current version (latest)
pub const NODEJS_CURRENT_VERSION: &str = "24.14.0";

/// Node.js download URL template
/// Format: https://nodejs.org/dist/v{version}/node-v{version}-{platform}-{arch}.{ext}
pub fn get_nodejs_download_url(version: &str, os: OS, arch: Arch) -> Option<String> {
    let (platform, arch_str, ext) = match (os, arch) {
        (OS::Windows, Arch::X64) => ("win", "x64", "zip"),
        (OS::Windows, Arch::ARM64) => ("win", "arm64", "zip"),
        (OS::Windows, Arch::X86) => ("win", "x86", "zip"),
        (OS::MacOS, Arch::X64) => ("darwin", "x64", "tar.gz"),
        (OS::MacOS, Arch::ARM64) => ("darwin", "arm64", "tar.gz"),
        (OS::Linux, Arch::X64) => ("linux", "x64", "tar.gz"),
        (OS::Linux, Arch::ARM64) => ("linux", "arm64", "tar.gz"),
        _ => return None,
    };

    Some(format!(
        "https://nodejs.org/dist/v{}/node-v{}-{}-{}.{}",
        version, version, platform, arch_str, ext
    ))
}

/// Bundle version
pub const BUNDLE_VERSION: &str = "1.0.0";

/// Platform information for bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
}

impl From<Platform> for PlatformInfo {
    fn from(platform: Platform) -> Self {
        Self {
            os: platform.os.to_string(),
            arch: platform.arch.to_string(),
        }
    }
}

/// Software information in bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareInfo {
    pub id: String,
    pub name: String,
    pub version: Option<String>,
}

/// Dependency information in bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyInfo {
    pub name: String,
    pub path: String,
}

/// File information in bundle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub sha256: String,
    pub size: u64,
}

/// Bundle manifest
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleManifest {
    /// Bundle format version
    pub version: String,
    /// Creation timestamp
    pub created_at: String,
    /// Target platform
    pub platform: PlatformInfo,
    /// Software being bundled
    pub software: SoftwareInfo,
    /// Included dependencies
    pub dependencies: Vec<DependencyInfo>,
    /// All files with checksums
    pub files: Vec<FileInfo>,
}

impl BundleManifest {
    /// Load manifest from a bundle directory
    pub fn load(bundle_dir: &Path) -> InstallerResult<Self> {
        let manifest_path = bundle_dir.join("manifest.json");
        if !manifest_path.exists() {
            return Err(InstallerError::FileSystem(format!(
                "Manifest not found: {}",
                manifest_path.display()
            )));
        }

        let content = std::fs::read_to_string(&manifest_path)?;
        let manifest: BundleManifest = serde_json::from_str(&content)?;

        Ok(manifest)
    }

    /// Save manifest to a bundle directory
    pub fn save(&self, bundle_dir: &Path) -> InstallerResult<()> {
        let manifest_path = bundle_dir.join("manifest.json");
        let content = serde_json::to_string_pretty(self)?;

        std::fs::write(&manifest_path, content)?;

        Ok(())
    }
}

/// Bundle creator
pub struct BundleCreator {
    client: Client,
    platform: Platform,
    output_dir: PathBuf,
}

impl BundleCreator {
    /// Create a new bundle creator
    pub fn new(output_dir: PathBuf) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(300)) // 5 minute timeout
            .user_agent("BurnCloud-Installer/1.0")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            platform: Platform::current(),
            output_dir,
        }
    }

    /// Create a bundle for the specified software
    pub async fn create(&self, software_id: &str) -> InstallerResult<PathBuf> {
        let software = get_software(software_id)
            .ok_or_else(|| InstallerError::SoftwareNotFound(software_id.to_string()))?;

        info!("Creating bundle for {}...", software.name);

        // Create bundle directory structure
        let bundle_dir = self.output_dir.join(format!("{}-bundle", software_id));
        let software_dir = bundle_dir.join("software").join(software_id);
        let deps_dir = bundle_dir.join("dependencies");

        // Clean and create directories
        if bundle_dir.exists() {
            std::fs::remove_dir_all(&bundle_dir)?;
        }
        std::fs::create_dir_all(&software_dir)?;
        std::fs::create_dir_all(&deps_dir)?;

        let mut files: Vec<FileInfo> = Vec::new();
        let mut dependencies: Vec<DependencyInfo> = Vec::new();

        // Process software installation method
        match &software.install_method {
            InstallMethod::NpmPackage {
                package,
                version,
                global: _,
            } => {
                let npm_dir = software_dir.join("npm-package");
                std::fs::create_dir_all(&npm_dir)?;

                // Download npm tarball
                let tarball =
                    NpmInstaller::download_tarball(package, version.as_deref(), &npm_dir)?;

                let file_info = self.add_file_info(&tarball, &bundle_dir)?;
                files.push(file_info);
            }
            InstallMethod::GitHubRelease {
                owner,
                repo,
                asset_patterns,
                extract_archive: _,
            } => {
                let release_dir = software_dir.join("release");
                std::fs::create_dir_all(&release_dir)?;

                // Fetch release info
                let release = self.fetch_github_release(owner, repo).await?;

                // Find matching asset
                let pattern = asset_patterns
                    .get(&(self.platform.os, self.platform.arch))
                    .ok_or_else(|| {
                        InstallerError::PlatformNotSupported(format!(
                            "No asset pattern for {}",
                            self.platform
                        ))
                    })?;

                let asset = self.find_matching_asset(&release.assets, pattern)?;

                // Download asset
                let asset_path = release_dir.join(&asset.name);
                self.download_file(&asset.browser_download_url, &asset_path)
                    .await?;

                let file_info = self.add_file_info(&asset_path, &bundle_dir)?;
                files.push(file_info);
            }
            InstallMethod::GitRepo { .. } => {
                // For GitRepo, we'll download the npm tarball if it's an npm project
                // or clone the repo as a zip
                let git_dir = software_dir.join("git-repo");
                std::fs::create_dir_all(&git_dir)?;

                // Try to download as npm package first (for openclaw)
                if software_id == "openclaw" {
                    let tarball = NpmInstaller::download_tarball("openclaw", None, &git_dir)?;
                    let file_info = self.add_file_info(&tarball, &bundle_dir)?;
                    files.push(file_info);
                } else {
                    // For other repos, we would clone or download archive
                    return Err(InstallerError::Configuration(
                        "GitRepo bundling not fully implemented for this software".to_string(),
                    ));
                }
            }
            InstallMethod::DirectDownload { .. } => {
                // Special handling for Node.js - download from nodejs.org
                if software_id == "nodejs" {
                    let nodejs_dir = software_dir.join("nodejs");
                    std::fs::create_dir_all(&nodejs_dir)?;

                    // Get the version from software definition or use default
                    let version = software.version.as_deref().unwrap_or(NODEJS_LTS_VERSION);

                    // Download Node.js official package
                    if let Some(nodejs_url) =
                        get_nodejs_download_url(version, self.platform.os, self.platform.arch)
                    {
                        let nodejs_filename = nodejs_url.split('/').next_back().unwrap_or("nodejs.zip");
                        let nodejs_path = nodejs_dir.join(nodejs_filename);

                        info!("Downloading Node.js {} from: {}", version, nodejs_url);
                        self.download_file(&nodejs_url, &nodejs_path).await?;

                        info!("Downloaded Node.js {} for offline installation", version);

                        let file_info = self.add_file_info(&nodejs_path, &bundle_dir)?;
                        files.push(file_info);
                    } else {
                        return Err(InstallerError::PlatformNotSupported(format!(
                            "No Node.js download available for {}",
                            self.platform
                        )));
                    }
                } else {
                    return Err(InstallerError::Configuration(format!(
                        "DirectDownload bundling not implemented for: {}",
                        software_id
                    )));
                }
            }
            _ => {
                return Err(InstallerError::Configuration(format!(
                    "Bundling not supported for this installation method: {:?}",
                    software.install_method
                )));
            }
        }

        // Process dependencies
        for dep in &software.dependencies {
            if let Some(auto_install) = &dep.auto_install {
                let dep_path = self
                    .bundle_dependency(&dep.name, auto_install, &deps_dir)
                    .await?;
                if let Some(dep_path) = dep_path {
                    dependencies.push(DependencyInfo {
                        name: dep.name.clone(),
                        path: dep_path
                            .strip_prefix(&bundle_dir)?
                            .to_string_lossy()
                            .to_string(),
                    });

                    // Add dependency files to file list
                    self.add_directory_files(&dep_path, &bundle_dir, &mut files)?;
                }
            }
        }

        // Create manifest
        let manifest = BundleManifest {
            version: BUNDLE_VERSION.to_string(),
            created_at: Utc::now().to_rfc3339(),
            platform: PlatformInfo::from(self.platform),
            software: SoftwareInfo {
                id: software.id.clone(),
                name: software.name.clone(),
                version: software.version.clone(),
            },
            dependencies,
            files,
        };

        // Save manifest
        manifest.save(&bundle_dir)?;

        // Create checksums file
        self.create_checksums_file(&bundle_dir)?;

        info!("Bundle created at: {}", bundle_dir.display());
        Ok(bundle_dir)
    }

    /// Bundle a dependency
    async fn bundle_dependency(
        &self,
        dep_name: &str,
        method: &InstallMethod,
        deps_dir: &Path,
    ) -> InstallerResult<Option<PathBuf>> {
        match method {
            InstallMethod::GitHubRelease {
                owner,
                repo,
                asset_patterns,
                ..
            } => {
                // Create platform-specific directory
                let platform_dir = deps_dir
                    .join(dep_name.to_lowercase())
                    .join(self.platform.os.to_string().to_lowercase())
                    .join(self.platform.arch.to_string().to_lowercase());

                std::fs::create_dir_all(&platform_dir)?;

                // Fetch release
                let release = self.fetch_github_release(owner, repo).await?;

                // Find matching asset
                let pattern = asset_patterns
                    .get(&(self.platform.os, self.platform.arch))
                    .ok_or_else(|| {
                        InstallerError::PlatformNotSupported(format!(
                            "No asset pattern for dependency {} on {}",
                            dep_name, self.platform
                        ))
                    })?;

                let asset = self.find_matching_asset(&release.assets, pattern)?;

                // Download asset
                let asset_path = platform_dir.join(&asset.name);
                self.download_file(&asset.browser_download_url, &asset_path)
                    .await?;

                info!("Downloaded dependency {}: {}", dep_name, asset.name);

                // Special handling for Node.js: also download Node.js official package
                // This enables offline installation without network access
                if dep_name.to_lowercase() == "node.js" {
                    info!("Downloading Node.js offline package for offline installation...");
                    if let Some(nodejs_url) = get_nodejs_download_url(
                        NODEJS_CURRENT_VERSION,
                        self.platform.os,
                        self.platform.arch,
                    ) {
                        // Determine filename from URL
                        let nodejs_filename = nodejs_url.split('/').next_back().unwrap_or("nodejs.zip");
                        let nodejs_path = platform_dir.join(nodejs_filename);

                        info!("Downloading Node.js from: {}", nodejs_url);
                        match self.download_file(&nodejs_url, &nodejs_path).await {
                            Ok(()) => {
                                info!(
                                    "Downloaded Node.js {} for offline installation",
                                    NODEJS_CURRENT_VERSION
                                );
                            }
                            Err(e) => {
                                warn!("Failed to download Node.js offline package: {}", e);
                                warn!(
                                    "Installation will require network access to download Node.js"
                                );
                            }
                        }
                    }
                }

                Ok(Some(platform_dir))
            }
            InstallMethod::DirectDownload {
                url: _,
                filename: _,
            } => {
                // Special handling for Node.js: download from nodejs.org
                if dep_name.to_lowercase() == "node.js" {
                    info!("Downloading Node.js directly from nodejs.org...");

                    // Create platform-specific directory
                    let platform_dir = deps_dir
                        .join(dep_name.to_lowercase())
                        .join(self.platform.os.to_string().to_lowercase())
                        .join(self.platform.arch.to_string().to_lowercase());

                    std::fs::create_dir_all(&platform_dir)?;

                    // Get Node.js download URL
                    let nodejs_url = get_nodejs_download_url(
                        NODEJS_CURRENT_VERSION,
                        self.platform.os,
                        self.platform.arch,
                    )
                    .ok_or_else(|| {
                        InstallerError::PlatformNotSupported(format!(
                            "No Node.js download available for {}",
                            self.platform
                        ))
                    })?;

                    // Determine filename from URL
                    let nodejs_filename = nodejs_url.split('/').next_back().unwrap_or("nodejs.zip");
                    let nodejs_path = platform_dir.join(nodejs_filename);

                    info!("Downloading Node.js from: {}", nodejs_url);
                    self.download_file(&nodejs_url, &nodejs_path).await?;

                    info!(
                        "Downloaded Node.js {} for offline installation",
                        NODEJS_CURRENT_VERSION
                    );

                    Ok(Some(platform_dir))
                } else {
                    info!(
                        "Skipping dependency {} (DirectDownload not implemented for this dependency)",
                        dep_name
                    );
                    Ok(None)
                }
            }
            _ => {
                info!(
                    "Skipping dependency {} (auto-install not supported for this method)",
                    dep_name
                );
                Ok(None)
            }
        }
    }

    /// Add file info with SHA256 checksum
    fn add_file_info(&self, file_path: &Path, bundle_dir: &Path) -> InstallerResult<FileInfo> {
        let mut file = File::open(file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        hasher.update(&buffer);

        let checksum = format!("{:x}", hasher.finalize());
        let size = file_path.metadata()?.len();
        let relative_path = file_path
            .strip_prefix(bundle_dir)?
            .to_string_lossy()
            .to_string();

        Ok(FileInfo {
            path: relative_path,
            sha256: checksum,
            size,
        })
    }

    /// Add all files from a directory
    fn add_directory_files(
        &self,
        dir: &Path,
        bundle_dir: &Path,
        files: &mut Vec<FileInfo>,
    ) -> InstallerResult<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_info = self.add_file_info(&path, bundle_dir)?;
                files.push(file_info);
            } else if path.is_dir() {
                self.add_directory_files(&path, bundle_dir, files)?;
            }
        }

        Ok(())
    }

    /// Create checksums.sha256 file
    fn create_checksums_file(&self, bundle_dir: &Path) -> InstallerResult<()> {
        let checksums_path = bundle_dir.join("checksums.sha256");
        let mut file = File::create(&checksums_path)?;

        let manifest = BundleManifest::load(bundle_dir)?;
        for fi in &manifest.files {
            writeln!(file, "{}  {}", fi.sha256, fi.path)?;
        }

        Ok(())
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
        assets: &[crate::software::GitHubAsset],
        pattern: &str,
    ) -> InstallerResult<crate::software::GitHubAsset> {
        // Convert glob-like pattern to regex
        let regex_pattern = pattern.replace(".", "\\.").replace("*", ".*");

        let regex = regex::Regex::new(&format!("^{}$", regex_pattern))
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

    /// Download file with retry logic
    async fn download_file(&self, url: &str, path: &Path) -> InstallerResult<()> {
        let max_retries = 3;
        let mut last_error = None;

        for attempt in 1..=max_retries {
            match self.download_file_inner(url, path).await {
                Ok(()) => {
                    if attempt > 1 {
                        info!("Download succeeded on attempt {}", attempt);
                    }
                    return Ok(());
                }
                Err(e) => {
                    last_error = Some(e);
                    if attempt < max_retries {
                        let delay = std::time::Duration::from_secs(2 * attempt as u64);
                        warn!(
                            "Download attempt {}/{} failed, retrying in {:?}...",
                            attempt, max_retries, delay
                        );
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| {
            InstallerError::Download("Download failed after all retries".to_string())
        }))
    }

    /// Download file (internal implementation)
    async fn download_file_inner(&self, url: &str, path: &Path) -> InstallerResult<()> {
        info!("Downloading: {}", url);

        let response = self.client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(InstallerError::Download(format!(
                "Download failed: {}",
                response.status()
            )));
        }

        let bytes = response.bytes().await?;
        std::fs::write(path, bytes)?;

        info!(
            "Downloaded: {} ({} bytes)",
            path.display(),
            path.metadata().map(|m| m.len()).unwrap_or(0)
        );

        Ok(())
    }
}

/// Bundle verifier
pub struct BundleVerifier;

impl BundleVerifier {
    /// Verify a bundle directory
    pub fn verify(bundle_dir: &Path) -> InstallerResult<BundleManifest> {
        if !bundle_dir.exists() {
            return Err(InstallerError::FileSystem(format!(
                "Bundle directory not found: {}",
                bundle_dir.display()
            )));
        }

        // Load and validate manifest
        let manifest = BundleManifest::load(bundle_dir)?;

        // Verify platform compatibility
        let current_platform = Platform::current();
        let bundle_os = manifest.platform.os.to_lowercase();
        let _bundle_arch = manifest.platform.arch.to_lowercase();

        if bundle_os != current_platform.os.to_string().to_lowercase() {
            return Err(InstallerError::PlatformNotSupported(format!(
                "Bundle is for {}, but current platform is {}",
                manifest.platform.os, current_platform.os
            )));
        }

        // Verify files and checksums
        let checksums_path = bundle_dir.join("checksums.sha256");
        if checksums_path.exists() {
            Self::verify_checksums(bundle_dir, &checksums_path)?;
        } else {
            // Verify each file in manifest
            for fi in &manifest.files {
                let file_path = bundle_dir.join(&fi.path);
                if !file_path.exists() {
                    return Err(InstallerError::FileSystem(format!(
                        "File missing from bundle: {}",
                        fi.path
                    )));
                }

                // Verify checksum
                let mut file = File::open(&file_path)?;
                let mut hasher = Sha256::new();
                let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;
                hasher.update(&buffer);

                let checksum = format!("{:x}", hasher.finalize());
                if checksum != fi.sha256 {
                    return Err(InstallerError::FileSystem(format!(
                        "Checksum mismatch for {}: expected {}, got {}",
                        fi.path, fi.sha256, checksum
                    )));
                }
            }
        }

        info!("Bundle verified successfully");
        Ok(manifest)
    }

    /// Verify checksums from checksums.sha256 file
    fn verify_checksums(bundle_dir: &Path, checksums_path: &Path) -> InstallerResult<()> {
        let file = File::open(checksums_path)?;
        let reader = BufReader::new(file);

        for line in reader.lines() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            // Parse format: "checksum  path" (two spaces)
            let parts: Vec<&str> = line.splitn(2, "  ").collect();
            if parts.len() != 2 {
                continue;
            }

            let expected_checksum = parts[0];
            let file_path = bundle_dir.join(parts[1]);

            if !file_path.exists() {
                return Err(InstallerError::FileSystem(format!(
                    "File missing: {}",
                    parts[1]
                )));
            }

            // Calculate checksum
            let mut file = File::open(&file_path)?;
            let mut hasher = Sha256::new();
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            hasher.update(&buffer);

            let actual_checksum = format!("{:x}", hasher.finalize());
            if actual_checksum != expected_checksum {
                return Err(InstallerError::FileSystem(format!(
                    "Checksum mismatch for {}: expected {}, got {}",
                    parts[1], expected_checksum, actual_checksum
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_info_from_platform() {
        let platform = Platform::current();
        let info = PlatformInfo::from(platform);

        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }

    #[test]
    fn test_bundle_manifest_serialization() {
        let manifest = BundleManifest {
            version: BUNDLE_VERSION.to_string(),
            created_at: Utc::now().to_rfc3339(),
            platform: PlatformInfo {
                os: "Windows".to_string(),
                arch: "x64".to_string(),
            },
            software: SoftwareInfo {
                id: "test".to_string(),
                name: "Test Software".to_string(),
                version: Some("1.0.0".to_string()),
            },
            dependencies: vec![],
            files: vec![],
        };

        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: BundleManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(manifest.version, deserialized.version);
        assert_eq!(manifest.software.id, deserialized.software.id);
    }
}
