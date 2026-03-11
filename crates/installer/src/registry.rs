//! Software registry

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::platform::{Arch, OS};
use crate::software::{Dependency, InstallMethod, Software};

/// Global software registry
static SOFTWARE_REGISTRY: OnceLock<HashMap<&'static str, Software>> = OnceLock::new();

/// Get the software registry
pub fn get_registry() -> &'static HashMap<&'static str, Software> {
    SOFTWARE_REGISTRY.get_or_init(|| {
        let mut registry = HashMap::new();

        // OpenClaw
        let openclaw = create_openclaw();
        registry.insert("openclaw", openclaw);

        // Cherry Studio
        let cherry_studio = create_cherry_studio();
        registry.insert("cherry-studio", cherry_studio);

        // fnm (Fast Node Manager)
        let fnm = create_fnm();
        registry.insert("fnm", fnm);

        // Git for Windows
        let git = create_git();
        registry.insert("git", git);

        registry
    })
}

/// Get a software by ID
pub fn get_software(id: &str) -> Option<&'static Software> {
    get_registry().get(id)
}

/// List all available software
pub fn list_software() -> Vec<&'static Software> {
    get_registry().values().collect()
}

/// Create OpenClaw software definition
fn create_openclaw() -> Software {
    Software::new(
        "openclaw",
        "OpenClaw",
        "Open source personal AI assistant that runs 24/7. Supports WhatsApp, Telegram, Discord, and more.",
        // Primary installation method: npm package
        InstallMethod::NpmPackage {
            package: "openclaw".to_string(),
            version: Some("latest".to_string()),
            global: true,
        },
    )
    .with_homepage("https://openclaw.ai")
    .with_category("AI Tools")
    .with_tag("ai")
    .with_tag("assistant")
    .with_tag("automation")
    .with_tag("npm")
    // Git dependency - required for git-based installation and general usage
    .with_dependency(
        Dependency::new("Git", "git --version")
            .with_expected_output("git version")
            .with_install_hint("https://git-scm.com/downloads")
            .with_auto_install(InstallMethod::PackageManager {
                windows: Some("winget install Git.Git".to_string()),
                macos: Some("brew install git".to_string()),
                linux: Some("apt install git -y".to_string()),
            }),
    )
    // Node.js dependency - required for OpenClaw (needs v22+)
    .with_dependency(
        Dependency::new("Node.js", "node --version")
            .with_expected_output("v2")  // Expects v22+
            .with_install_hint("Use 'burncloud install fnm' to install Node.js via fnm, then 'fnm install 22'")
            .with_auto_install(InstallMethod::PackageManager {
                windows: Some("fnm install 22".to_string()),
                macos: Some("fnm install 22".to_string()),
                linux: Some("fnm install 22".to_string()),
            }),
    )
    .with_platforms(vec![
        (OS::Windows, Arch::X64),
        (OS::MacOS, Arch::ARM64),
        (OS::MacOS, Arch::X64),
        (OS::Linux, Arch::X64),
    ])
}

/// OpenClaw GitHub repository info for offline installation
pub const OPENCLAW_GIT_REPO: &str = "https://github.com/openclaw/openclaw.git";

/// Get OpenClaw GitRepo installation method for offline installation
pub fn get_openclaw_git_method() -> InstallMethod {
    InstallMethod::GitRepo {
        url: OPENCLAW_GIT_REPO.to_string(),
        branch: Some("main".to_string()),
        build_command: Some("pnpm install && pnpm build".to_string()),
        package_manager: Some("pnpm".to_string()),
    }
}

/// Create Cherry Studio software definition
fn create_cherry_studio() -> Software {
    let mut asset_patterns = HashMap::new();
    asset_patterns.insert(
        (OS::Windows, Arch::X64),
        "Cherry-Studio-*-x64-setup.exe".to_string(),
    );
    asset_patterns.insert(
        (OS::Windows, Arch::ARM64),
        "Cherry-Studio-*-arm64-setup.exe".to_string(),
    );
    asset_patterns.insert(
        (OS::MacOS, Arch::ARM64),
        "Cherry-Studio-*-arm64.dmg".to_string(),
    );
    asset_patterns.insert(
        (OS::MacOS, Arch::X64),
        "Cherry-Studio-*-x64.dmg".to_string(),
    );
    asset_patterns.insert(
        (OS::Linux, Arch::X64),
        "Cherry-Studio-*-x86_64.AppImage".to_string(),
    );

    Software::new(
        "cherry-studio",
        "Cherry Studio",
        "AI productivity tool with 41K+ GitHub stars. A powerful desktop application for AI-assisted work.",
        InstallMethod::GitHubRelease {
            owner: "CherryHQ".to_string(),
            repo: "cherry-studio".to_string(),
            asset_patterns,
            extract_archive: Some(false),
        },
    )
    .with_homepage("https://github.com/CherryHQ/cherry-studio")
    .with_category("AI Tools")
    .with_tag("ai")
    .with_tag("productivity")
    .with_tag("desktop")
    .with_platforms(vec![
        (OS::Windows, Arch::X64),
        (OS::Windows, Arch::ARM64),
        (OS::MacOS, Arch::ARM64),
        (OS::MacOS, Arch::X64),
        (OS::Linux, Arch::X64),
    ])
}

/// Create fnm (Fast Node Manager) software definition
fn create_fnm() -> Software {
    let mut asset_patterns = HashMap::new();
    // fnm release asset naming: fnm-{platform}.zip (simpler naming)
    // Note: fnm uses unified binaries per platform
    asset_patterns.insert((OS::Windows, Arch::X64), "fnm-windows.zip".to_string());
    asset_patterns.insert(
        (OS::Windows, Arch::ARM64),
        "fnm-arm64.zip".to_string(), // Universal ARM64
    );
    asset_patterns.insert((OS::MacOS, Arch::X64), "fnm-macos.zip".to_string());
    asset_patterns.insert(
        (OS::MacOS, Arch::ARM64),
        "fnm-macos.zip".to_string(), // Same as x64 (universal binary)
    );
    asset_patterns.insert((OS::Linux, Arch::X64), "fnm-linux.zip".to_string());
    asset_patterns.insert(
        (OS::Linux, Arch::ARM64),
        "fnm-arm64.zip".to_string(), // Universal ARM64
    );

    Software::new(
        "fnm",
        "fnm (Fast Node Manager)",
        "Fast and simple Node.js version manager built in Rust. Cross-platform, speedy, and supports .nvmrc.",
        InstallMethod::GitHubRelease {
            owner: "Schniz".to_string(),
            repo: "fnm".to_string(),
            asset_patterns,
            extract_archive: Some(true),
        },
    )
    .with_homepage("https://github.com/Schniz/fnm")
    .with_version("latest")
    .with_category("Development")
    .with_tag("nodejs")
    .with_tag("javascript")
    .with_tag("version-manager")
    .with_tag("rust")
    .with_platforms(vec![
        (OS::Windows, Arch::X64),
        (OS::Windows, Arch::ARM64),
        (OS::MacOS, Arch::X64),
        (OS::MacOS, Arch::ARM64),
        (OS::Linux, Arch::X64),
        (OS::Linux, Arch::ARM64),
    ])
}

/// Create Git for Windows software definition
fn create_git() -> Software {
    let mut asset_patterns = HashMap::new();
    // Git for Windows release naming: Git-<version>-<arch>-bit.exe
    // Examples: Git-2.47.0-64-bit.exe, Git-2.47.0-32-bit.exe
    asset_patterns.insert((OS::Windows, Arch::X64), "Git-*-64-bit.exe".to_string());
    asset_patterns.insert((OS::Windows, Arch::X86), "Git-*-32-bit.exe".to_string());
    // Note: Git for Windows ARM64 uses the same 64-bit installer via emulation
    asset_patterns.insert((OS::Windows, Arch::ARM64), "Git-*-64-bit.exe".to_string());

    Software::new(
        "git",
        "Git",
        "Distributed version control system. Essential for cloning repositories and version control.",
        InstallMethod::GitHubRelease {
            owner: "git-for-windows".to_string(),
            repo: "git".to_string(),
            asset_patterns,
            extract_archive: Some(false), // It's an installer exe
        },
    )
    .with_homepage("https://git-scm.com")
    .with_category("Development")
    .with_tag("vcs")
    .with_tag("version-control")
    .with_tag("scm")
    .with_platforms(vec![
        (OS::Windows, Arch::X64),
        (OS::Windows, Arch::X86),
        (OS::Windows, Arch::ARM64),
    ])
}

/// Check if a software ID is valid
pub fn is_valid_software(id: &str) -> bool {
    get_registry().contains_key(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_contains_software() {
        let registry = get_registry();
        assert!(registry.contains_key("openclaw"));
        assert!(registry.contains_key("cherry-studio"));
        assert!(registry.contains_key("fnm"));
        assert!(registry.contains_key("git"));
    }

    #[test]
    fn test_get_software() {
        let software = get_software("openclaw");
        assert!(software.is_some());
        assert_eq!(software.unwrap().id, "openclaw");

        let software = get_software("cherry-studio");
        assert!(software.is_some());
        assert_eq!(software.unwrap().id, "cherry-studio");

        let software = get_software("fnm");
        assert!(software.is_some());
        assert_eq!(software.unwrap().id, "fnm");

        let software = get_software("git");
        assert!(software.is_some());
        assert_eq!(software.unwrap().id, "git");
    }

    #[test]
    fn test_list_software() {
        let list = list_software();
        assert!(list.len() >= 4);
    }

    #[test]
    fn test_openclaw_definition() {
        let software = get_software("openclaw").unwrap();
        assert_eq!(software.name, "OpenClaw");
        assert!(!software.dependencies.is_empty());
        assert!(software.homepage.is_some());
        // OpenClaw should have Git and Node.js dependencies
        assert!(software.dependencies.len() >= 2);
    }

    #[test]
    fn test_cherry_studio_definition() {
        let software = get_software("cherry-studio").unwrap();
        assert_eq!(software.name, "Cherry Studio");
        assert!(software.homepage.is_some());

        // Check GitHub release method
        if let InstallMethod::GitHubRelease { owner, repo, .. } = &software.install_method {
            assert_eq!(owner, "CherryHQ");
            assert_eq!(repo, "cherry-studio");
        } else {
            panic!("Expected GitHubRelease install method");
        }
    }

    #[test]
    fn test_fnm_definition() {
        let software = get_software("fnm").unwrap();
        assert_eq!(software.name, "fnm (Fast Node Manager)");
        assert!(software.homepage.is_some());

        // Check GitHub release method
        if let InstallMethod::GitHubRelease {
            owner,
            repo,
            extract_archive,
            ..
        } = &software.install_method
        {
            assert_eq!(owner, "Schniz");
            assert_eq!(repo, "fnm");
            assert_eq!(*extract_archive, Some(true));
        } else {
            panic!("Expected GitHubRelease install method");
        }

        // Check it has nodejs tag
        assert!(software.tags.contains(&"nodejs".to_string()));
    }

    #[test]
    fn test_git_definition() {
        let software = get_software("git").unwrap();
        assert_eq!(software.name, "Git");
        assert!(software.homepage.is_some());

        // Check GitHub release method
        if let InstallMethod::GitHubRelease {
            owner,
            repo,
            extract_archive,
            ..
        } = &software.install_method
        {
            assert_eq!(owner, "git-for-windows");
            assert_eq!(repo, "git");
            assert_eq!(*extract_archive, Some(false)); // It's an installer exe
        } else {
            panic!("Expected GitHubRelease install method");
        }

        // Check it has vcs tag
        assert!(software.tags.contains(&"vcs".to_string()));
    }
}
