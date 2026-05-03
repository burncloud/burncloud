//! Install subcommand handler

use anyhow::Result;
use burncloud_database_sys::InstallerDB;
use burncloud_installer::{Installer, InstallerConfig};
use clap::ArgMatches;
use tracing::error;
use std::path::PathBuf;

/// Handle install subcommand
pub async fn handle_install_command(matches: &ArgMatches) -> Result<()> {
    let list = matches.get_flag("list");
    let status = matches.get_flag("status");
    let auto_deps = matches.get_flag("auto-deps");
    let software_id = matches.get_one::<String>("software");
    let local_path = matches.get_one::<String>("local").map(PathBuf::from);
    let bundle_dir = matches.get_one::<String>("bundle").map(PathBuf::from);

    let config = InstallerConfig::new()
        .with_auto_deps(auto_deps)
        .with_bundle_dir(bundle_dir);

    let installer = Installer::new(config);

    if list {
        return handle_list(&installer).await;
    }

    if let Some(software_id) = software_id {
        if status {
            return handle_status(&installer, software_id).await;
        }

        // Check if local installation
        if let Some(local_path) = local_path {
            return handle_install_local(&installer, software_id, &local_path).await;
        }

        return handle_install(&installer, software_id).await;
    }

    // No specific action, show help
    show_install_help();
    Ok(())
}

/// List available software
async fn handle_list(installer: &Installer) -> Result<()> {
    println!("Available software to install:\n");

    let software_list = installer.list_available();

    for software in software_list {
        println!("  {} ({})", software.name, software.id);
        println!("    {}", software.description);
        if let Some(homepage) = &software.homepage {
            println!("    Homepage: {}", homepage);
        }
        if let Some(category) = &software.category {
            println!("    Category: {}", category);
        }
        if !software.tags.is_empty() {
            println!("    Tags: {}", software.tags.join(", "));
        }
        println!();
    }

    println!("Usage:");
    println!("  burncloud install <software-id>  - Install software");
    println!("  burncloud install <software-id> --status  - Check installation status");
    println!("  burncloud install <software-id> --auto-deps  - Auto-install dependencies");

    Ok(())
}

/// Check installation status
async fn handle_status(installer: &Installer, software_id: &str) -> Result<()> {
    match installer.get_software(software_id) {
        Some(software) => {
            println!("Software: {} ({})", software.name, software.id);
            println!("Description: {}", software.description);

            match installer.check_status(software_id).await {
                Ok(status) => {
                    println!("Status: {}", status);
                }
                Err(e) => {
                    println!("Status check error: {}", e);
                }
            }

            // Check database for installation record
            match InstallerDB::new().await {
                Ok(db) => {
                    if let Ok(Some(record)) = db.get(software_id).await {
                        if let Some(version) = record.version {
                            println!("Installed version: {}", version);
                        }
                        if let Some(install_dir) = record.install_dir {
                            println!("Install directory: {}", install_dir);
                        }
                        if let Some(installed_at) = record.installed_at {
                            println!("Installed at: {}", installed_at);
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to access installation database: {}", e);
                }
            }
        }
        None => {
            println!("Software '{}' not found", software_id);
            println!("Run 'burncloud install --list' to see available software");
        }
    }

    Ok(())
}

/// Install software
async fn handle_install(installer: &Installer, software_id: &str) -> Result<()> {
    match installer.get_software(software_id) {
        Some(software) => {
            println!("Installing {}...\n", software.name);

            // Initialize database for tracking
            let db = InstallerDB::new().await?;

            // Mark as installing
            db.mark_installing(software_id, &software.name).await?;

            match installer.install(software_id).await {
                Ok(()) => {
                    println!("\nSuccessfully installed {}!", software.name);

                    // Mark as installed in database
                    db.mark_installed(
                        software_id,
                        &software.name,
                        software.version.as_deref(),
                        None, // TODO: Get actual install dir
                        Some(&format!("{:?}", software.install_method)),
                    )
                    .await?;

                    if let Some(homepage) = &software.homepage {
                        println!("Visit {} to get started", homepage);
                    }
                }
                Err(e) => {
                    error!("Installation failed: {}", e);
                    println!("\nInstallation failed: {}", e);

                    // Mark as failed in database
                    db.mark_failed(software_id, &format!("{}", e)).await?;

                    return Err(anyhow::anyhow!("Installation failed: {}", e));
                }
            }
        }
        None => {
            println!("Software '{}' not found", software_id);
            println!("Run 'burncloud install --list' to see available software");
        }
    }

    Ok(())
}

/// Install software from local file
async fn handle_install_local(
    installer: &Installer,
    software_id: &str,
    local_path: &std::path::Path,
) -> Result<()> {
    match installer.get_software(software_id) {
        Some(software) => {
            println!(
                "Installing {} from local file: {}...\n",
                software.name,
                local_path.display()
            );

            // Initialize database for tracking
            let db = InstallerDB::new().await?;

            // Mark as installing
            db.mark_installing(software_id, &software.name).await?;

            match installer.install_from_local(software_id, local_path).await {
                Ok(()) => {
                    println!("\nSuccessfully installed {}!", software.name);

                    // Mark as installed in database
                    db.mark_installed(
                        software_id,
                        &software.name,
                        software.version.as_deref(),
                        Some(&local_path.to_string_lossy()),
                        Some("local"),
                    )
                    .await?;

                    if let Some(homepage) = &software.homepage {
                        println!("Visit {} to get started", homepage);
                    }
                }
                Err(e) => {
                    error!("Installation failed: {}", e);
                    println!("\nInstallation failed: {}", e);

                    // Mark as failed in database
                    db.mark_failed(software_id, &format!("{}", e)).await?;

                    return Err(anyhow::anyhow!("Installation failed: {}", e));
                }
            }
        }
        None => {
            println!("Software '{}' not found", software_id);
            println!("Run 'burncloud install --list' to see available software");
        }
    }

    Ok(())
}

/// Show install help
fn show_install_help() {
    println!("BurnCloud Software Installer");
    println!();
    println!("Usage:");
    println!("  burncloud install --list                      List available software");
    println!("  burncloud install <software>                  Install software");
    println!("  burncloud install <software> --status         Check installation status");
    println!("  burncloud install <software> --auto-deps      Auto-install dependencies");
    println!("  burncloud install <software> --local <path>   Install from local file");
    println!("  burncloud install <software> --bundle <dir>   Use local bundle for deps (offline)");
    println!();
    println!("Examples:");
    println!("  burncloud install --list");
    println!("  burncloud install openclaw");
    println!("  burncloud install cherry-studio --auto-deps");
    println!("  burncloud install openclaw --status");
    println!("  burncloud install fnm --local ./fnm-windows.zip");
    println!("  burncloud install openclaw --bundle ./offline-bundle/");
}
