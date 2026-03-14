//! Software definition types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::platform::{Arch, OS};

/// Shell type for script execution
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShellType {
    PowerShell,
    Bash,
    Sh,
    Cmd,
}

/// Installation method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstallMethod {
    /// Install via script (e.g., PowerShell, Bash)
    Script { url: String, shell: ShellType },
    /// Install from GitHub Releases
    GitHubRelease {
        owner: String,
        repo: String,
        /// Asset pattern mapping: (OS, Arch) -> pattern
        /// Use * as wildcard in patterns (e.g., "*-windows-*.exe")
        asset_patterns: HashMap<(OS, Arch), String>,
        /// Whether to extract as archive (auto-detect if None)
        extract_archive: Option<bool>,
    },
    /// Install via package manager
    PackageManager {
        /// Windows: winget, chocolatey, scoop
        /// macOS: brew
        /// Linux: apt, dnf, pacman, etc.
        windows: Option<String>,
        macos: Option<String>,
        linux: Option<String>,
    },
    /// Direct download URL
    DirectDownload {
        url: String,
        filename: Option<String>,
    },
    /// Install from Git repository
    GitRepo {
        /// Git repository URL (e.g., "https://github.com/user/repo.git")
        url: String,
        /// Branch or tag to checkout (default: main/master)
        branch: Option<String>,
        /// Build command after clone (e.g., "pnpm install && pnpm build")
        build_command: Option<String>,
        /// Package manager to use for building (npm, pnpm, yarn)
        package_manager: Option<String>,
    },
    /// Install via npm package
    NpmPackage {
        /// Package name (e.g., "openclaw")
        package: String,
        /// Version or tag (e.g., "latest", "1.0.0")
        version: Option<String>,
        /// Install globally
        global: bool,
    },
}

/// Software dependency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// Dependency name (e.g., "Git", "Node.js")
    pub name: String,
    /// Command to check if dependency is installed (e.g., "git --version")
    pub check_command: String,
    /// Expected output pattern (regex optional, if None just check command succeeds)
    pub expected_output: Option<String>,
    /// Installation hint or URL
    pub install_hint: Option<String>,
    /// Auto-install method (optional)
    pub auto_install: Option<InstallMethod>,
}

impl Dependency {
    /// Create a new dependency
    pub fn new(name: impl Into<String>, check_command: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            check_command: check_command.into(),
            expected_output: None,
            install_hint: None,
            auto_install: None,
        }
    }

    /// Add expected output pattern
    pub fn with_expected_output(mut self, pattern: impl Into<String>) -> Self {
        self.expected_output = Some(pattern.into());
        self
    }

    /// Add installation hint
    pub fn with_install_hint(mut self, hint: impl Into<String>) -> Self {
        self.install_hint = Some(hint.into());
        self
    }

    /// Add auto-install method
    pub fn with_auto_install(mut self, method: InstallMethod) -> Self {
        self.auto_install = Some(method);
        self
    }
}

/// Installation status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InstallStatus {
    NotInstalled,
    Installing,
    Installed,
    Failed,
    UpdateAvailable,
}

impl std::fmt::Display for InstallStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallStatus::NotInstalled => write!(f, "Not Installed"),
            InstallStatus::Installing => write!(f, "Installing"),
            InstallStatus::Installed => write!(f, "Installed"),
            InstallStatus::Failed => write!(f, "Failed"),
            InstallStatus::UpdateAvailable => write!(f, "Update Available"),
        }
    }
}

/// Software definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Software {
    /// Unique identifier (e.g., "openclaw", "cherry-studio")
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Homepage URL
    pub homepage: Option<String>,
    /// Icon URL or path
    pub icon: Option<String>,
    /// Version (if known)
    pub version: Option<String>,
    /// Installation method
    pub install_method: InstallMethod,
    /// Dependencies
    pub dependencies: Vec<Dependency>,
    /// Supported platforms (None = all platforms)
    pub supported_platforms: Option<Vec<(OS, Arch)>>,
    /// Category (e.g., "AI Tools", "Development")
    pub category: Option<String>,
    /// Tags
    pub tags: Vec<String>,
}

impl Software {
    /// Create a new software definition
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        description: impl Into<String>,
        install_method: InstallMethod,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: description.into(),
            homepage: None,
            icon: None,
            version: None,
            install_method,
            dependencies: Vec::new(),
            supported_platforms: None,
            category: None,
            tags: Vec::new(),
        }
    }

    /// Add homepage URL
    pub fn with_homepage(mut self, url: impl Into<String>) -> Self {
        self.homepage = Some(url.into());
        self
    }

    /// Add icon
    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Add version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Add dependency
    pub fn with_dependency(mut self, dep: Dependency) -> Self {
        self.dependencies.push(dep);
        self
    }

    /// Set supported platforms
    pub fn with_platforms(mut self, platforms: Vec<(OS, Arch)>) -> Self {
        self.supported_platforms = Some(platforms);
        self
    }

    /// Set category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Add tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Check if platform is supported
    pub fn supports_platform(&self, os: OS, arch: Arch) -> bool {
        match &self.supported_platforms {
            Some(platforms) => platforms.contains(&(os, arch)),
            None => true,
        }
    }
}

/// GitHub release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubRelease {
    pub tag_name: String,
    pub name: String,
    pub body: Option<String>,
    pub assets: Vec<GitHubAsset>,
    pub prerelease: bool,
    pub draft: bool,
}

/// GitHub release asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubAsset {
    pub name: String,
    pub browser_download_url: String,
    pub content_type: String,
    pub size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dependency_builder() {
        let dep = Dependency::new("Git", "git --version")
            .with_expected_output("git version")
            .with_install_hint("https://git-scm.com/downloads");

        assert_eq!(dep.name, "Git");
        assert_eq!(dep.check_command, "git --version");
        assert_eq!(dep.expected_output, Some("git version".to_string()));
    }

    #[test]
    fn test_software_builder() {
        let software = Software::new(
            "test-app",
            "Test App",
            "A test application",
            InstallMethod::DirectDownload {
                url: "https://example.com/app.exe".to_string(),
                filename: None,
            },
        )
        .with_homepage("https://example.com")
        .with_version("1.0.0");

        assert_eq!(software.id, "test-app");
        assert_eq!(software.homepage, Some("https://example.com".to_string()));
        assert_eq!(software.version, Some("1.0.0".to_string()));
    }
}
