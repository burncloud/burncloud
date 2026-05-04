//! npm package installation support

use tracing::info;
use std::path::Path;
use std::process::Command;

use crate::error::{InstallerError, InstallerResult};
use crate::platform::Platform;

/// npm package installer
pub struct NpmInstaller;

impl NpmInstaller {
    /// Check if npm is available
    pub fn is_available() -> bool {
        let result = if Platform::current().is_windows() {
            Command::new("cmd").args(["/C", "npm --version"]).output()
        } else {
            Command::new("sh").args(["-c", "npm --version"]).output()
        };

        match result {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    /// Install npm package from registry (online mode)
    pub fn install(package: &str, version: Option<&str>, global: bool) -> InstallerResult<()> {
        let package_spec = match version {
            Some(v) => format!("{}@{}", package, v),
            None => package.to_string(),
        };

        let mut args = vec!["install"];
        if global {
            args.push("-g");
        }
        args.push(&package_spec);

        info!(
            "Installing npm package: {} (global: {})",
            package_spec, global
        );

        let result = if Platform::current().is_windows() {
            Command::new("cmd").args(["/C", "npm"]).args(&args).status()
        } else {
            Command::new("sh")
                .args(["-c", &format!("npm {}", args.join(" "))])
                .status()
        };

        match result {
            Ok(status) if status.success() => {
                info!("Successfully installed npm package: {}", package_spec);
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

    /// Install npm package from tarball (offline mode)
    pub fn install_from_tarball(tarball_path: &Path, global: bool) -> InstallerResult<()> {
        if !tarball_path.exists() {
            return Err(InstallerError::FileSystem(format!(
                "Tarball not found: {}",
                tarball_path.display()
            )));
        }

        let tarball_str = tarball_path.to_string_lossy();
        let mut args = vec!["install"];
        if global {
            args.push("-g");
        }
        args.push(&tarball_str);

        info!(
            "Installing npm package from tarball: {} (global: {})",
            tarball_path.display(),
            global
        );

        let result = if Platform::current().is_windows() {
            Command::new("cmd").args(["/C", "npm"]).args(&args).status()
        } else {
            Command::new("sh")
                .args(["-c", &format!("npm {}", args.join(" "))])
                .status()
        };

        match result {
            Ok(status) if status.success() => {
                info!(
                    "Successfully installed npm package from tarball: {}",
                    tarball_path.display()
                );
                Ok(())
            }
            Ok(status) => Err(InstallerError::InstallationFailed(format!(
                "npm install from tarball exited with code: {}",
                status.code().unwrap_or(-1)
            ))),
            Err(e) => Err(InstallerError::InstallationFailed(format!(
                "Failed to run npm install from tarball: {}",
                e
            ))),
        }
    }

    /// Pack an npm package to a tarball (for bundle creation)
    pub fn pack(package_dir: &Path, output_dir: &Path) -> InstallerResult<std::path::PathBuf> {
        if !package_dir.exists() {
            return Err(InstallerError::FileSystem(format!(
                "Package directory not found: {}",
                package_dir.display()
            )));
        }

        // Run npm pack in the package directory
        let result = if Platform::current().is_windows() {
            Command::new("cmd")
                .args(["/C", "npm", "pack"])
                .current_dir(package_dir)
                .output()
        } else {
            Command::new("sh")
                .args(["-c", "npm pack"])
                .current_dir(package_dir)
                .output()
        };

        match result {
            Ok(output) if output.status.success() => {
                // Get the tarball filename from stdout
                let tarball_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if tarball_name.is_empty() {
                    return Err(InstallerError::InstallationFailed(
                        "npm pack did not produce output".to_string(),
                    ));
                }

                let tarball_path = package_dir.join(&tarball_name);
                let dest_path = output_dir.join(&tarball_name);

                // Move tarball to output directory
                if tarball_path.exists() {
                    std::fs::copy(&tarball_path, &dest_path).map_err(|e| {
                        InstallerError::FileSystem(format!("Failed to copy tarball: {}", e))
                    })?;
                    std::fs::remove_file(&tarball_path).ok();

                    info!("Created npm tarball: {}", dest_path.display());
                    Ok(dest_path)
                } else {
                    Err(InstallerError::FileSystem(format!(
                        "Tarball not created: {}",
                        tarball_path.display()
                    )))
                }
            }
            Ok(output) => Err(InstallerError::InstallationFailed(format!(
                "npm pack failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ))),
            Err(e) => Err(InstallerError::InstallationFailed(format!(
                "Failed to run npm pack: {}",
                e
            ))),
        }
    }

    /// Download an npm package tarball without installing (for bundle creation)
    pub fn download_tarball(
        package: &str,
        version: Option<&str>,
        output_dir: &Path,
    ) -> InstallerResult<std::path::PathBuf> {
        let package_spec = match version {
            Some(v) => format!("{}@{}", package, v),
            None => format!("{}@latest", package),
        };

        // Use npm pack to download without installing
        // npm pack <package>@<version> will download the tarball
        let pack_arg = package_spec.clone();

        let result = if Platform::current().is_windows() {
            Command::new("cmd")
                .args(["/C", "npm", "pack", &pack_arg])
                .current_dir(output_dir)
                .output()
        } else {
            Command::new("sh")
                .args(["-c", &format!("npm pack {}", pack_arg)])
                .current_dir(output_dir)
                .output()
        };

        match result {
            Ok(output) if output.status.success() => {
                // Get the tarball filename from stdout
                let tarball_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if tarball_name.is_empty() {
                    return Err(InstallerError::InstallationFailed(
                        "npm pack did not produce output".to_string(),
                    ));
                }

                let tarball_path = output_dir.join(&tarball_name);

                if tarball_path.exists() {
                    info!(
                        "Downloaded npm tarball: {} -> {}",
                        package_spec,
                        tarball_path.display()
                    );
                    Ok(tarball_path)
                } else {
                    Err(InstallerError::FileSystem(format!(
                        "Tarball not created: {}",
                        tarball_path.display()
                    )))
                }
            }
            Ok(output) => Err(InstallerError::InstallationFailed(format!(
                "npm pack failed for {}: {}",
                package_spec,
                String::from_utf8_lossy(&output.stderr)
            ))),
            Err(e) => Err(InstallerError::InstallationFailed(format!(
                "Failed to run npm pack for {}: {}",
                package_spec, e
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available() {
        // This test will pass if npm is installed on the system
        // It's informational, not a hard requirement
        let _available = NpmInstaller::is_available();
    }
}
