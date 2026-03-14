//! Platform detection module

use serde::{Deserialize, Serialize};
use std::env;

/// Operating system type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OS {
    Windows,
    MacOS,
    Linux,
    Other,
}

/// CPU architecture type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Arch {
    X64,
    X86,
    ARM64,
    ARM,
    Other,
}

/// Platform information
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Platform {
    pub os: OS,
    pub arch: Arch,
}

impl Platform {
    /// Detect current platform
    pub fn current() -> Self {
        let os = match env::consts::OS {
            "windows" => OS::Windows,
            "macos" => OS::MacOS,
            "linux" => OS::Linux,
            _ => OS::Other,
        };

        let arch = match env::consts::ARCH {
            "x86_64" | "amd64" => Arch::X64,
            "x86" | "i386" | "i686" => Arch::X86,
            "aarch64" | "arm64" => Arch::ARM64,
            "arm" => Arch::ARM,
            _ => Arch::Other,
        };

        Self { os, arch }
    }

    /// Check if platform is Windows
    pub fn is_windows(&self) -> bool {
        self.os == OS::Windows
    }

    /// Check if platform is macOS
    pub fn is_macos(&self) -> bool {
        self.os == OS::MacOS
    }

    /// Check if platform is Linux
    pub fn is_linux(&self) -> bool {
        self.os == OS::Linux
    }

    /// Get platform string for GitHub releases
    pub fn to_github_target(&self) -> String {
        match (&self.os, &self.arch) {
            (OS::Windows, Arch::X64) => "win32-x64".to_string(),
            (OS::Windows, Arch::ARM64) => "win32-arm64".to_string(),
            (OS::MacOS, Arch::X64) => "darwin-x64".to_string(),
            (OS::MacOS, Arch::ARM64) => "darwin-arm64".to_string(),
            (OS::Linux, Arch::X64) => "linux-x64".to_string(),
            (OS::Linux, Arch::ARM64) => "linux-arm64".to_string(),
            _ => format!("{:?}-{:?}", self.os, self.arch).to_lowercase(),
        }
    }

    /// Get executable extension
    pub fn exe_extension(&self) -> &'static str {
        if self.is_windows() {
            ".exe"
        } else {
            ""
        }
    }
}

impl Default for Platform {
    fn default() -> Self {
        Self::current()
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}-{:?}", self.os, self.arch)
    }
}

impl std::fmt::Display for OS {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OS::Windows => write!(f, "Windows"),
            OS::MacOS => write!(f, "macOS"),
            OS::Linux => write!(f, "Linux"),
            OS::Other => write!(f, "Other"),
        }
    }
}

impl std::fmt::Display for Arch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Arch::X64 => write!(f, "x64"),
            Arch::X86 => write!(f, "x86"),
            Arch::ARM64 => write!(f, "arm64"),
            Arch::ARM => write!(f, "arm"),
            Arch::Other => write!(f, "other"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_platform() {
        let platform = Platform::current();
        // Just ensure it doesn't panic
        assert!(!platform.to_string().is_empty());
    }

    #[test]
    fn test_github_target() {
        let platform = Platform {
            os: OS::Windows,
            arch: Arch::X64,
        };
        assert_eq!(platform.to_github_target(), "win32-x64");

        let platform = Platform {
            os: OS::MacOS,
            arch: Arch::ARM64,
        };
        assert_eq!(platform.to_github_target(), "darwin-arm64");
    }

    #[test]
    fn test_exe_extension() {
        let windows = Platform {
            os: OS::Windows,
            arch: Arch::X64,
        };
        assert_eq!(windows.exe_extension(), ".exe");

        let linux = Platform {
            os: OS::Linux,
            arch: Arch::X64,
        };
        assert_eq!(linux.exe_extension(), "");
    }
}
