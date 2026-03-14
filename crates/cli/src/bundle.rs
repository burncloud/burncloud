//! Bundle subcommand handler

use anyhow::Result;
use burncloud_installer::{BundleCreator, BundleVerifier};
use clap::ArgMatches;
use log::error;
use std::path::PathBuf;

/// Handle bundle subcommand
pub async fn handle_bundle_command(matches: &ArgMatches) -> Result<()> {
    match matches.subcommand() {
        Some(("create", create_matches)) => handle_create(create_matches).await,
        Some(("verify", verify_matches)) => handle_verify(verify_matches).await,
        _ => {
            show_bundle_help();
            Ok(())
        }
    }
}

/// Handle bundle create subcommand
async fn handle_create(matches: &ArgMatches) -> Result<()> {
    let software_id = matches
        .get_one::<String>("software")
        .ok_or_else(|| anyhow::anyhow!("Software ID is required"))?;

    let output_dir = matches
        .get_one::<String>("output")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap().join("bundles"));

    println!("Creating offline bundle for '{}'...", software_id);
    println!("Output directory: {}\n", output_dir.display());

    // Create output directory if it doesn't exist
    std::fs::create_dir_all(&output_dir)?;

    let creator = BundleCreator::new(output_dir);

    match creator.create(software_id).await {
        Ok(bundle_path) => {
            println!("\nBundle created successfully!");
            println!("Bundle location: {}", bundle_path.display());
            println!("\nTo install from this bundle (offline):");
            println!(
                "  burncloud install {} --bundle {}",
                software_id,
                bundle_path.display()
            );
        }
        Err(e) => {
            error!("Failed to create bundle: {}", e);
            println!("\nFailed to create bundle: {}", e);
            return Err(anyhow::anyhow!("Bundle creation failed: {}", e));
        }
    }

    Ok(())
}

/// Handle bundle verify subcommand
async fn handle_verify(matches: &ArgMatches) -> Result<()> {
    let bundle_path = matches
        .get_one::<String>("bundle")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow::anyhow!("Bundle path is required"))?;

    println!("Verifying bundle: {}\n", bundle_path.display());

    match BundleVerifier::verify(&bundle_path) {
        Ok(manifest) => {
            println!("Bundle verification: PASSED\n");
            println!("Bundle Information:");
            println!("  Version: {}", manifest.version);
            println!("  Created: {}", manifest.created_at);
            println!(
                "  Platform: {} {}",
                manifest.platform.os, manifest.platform.arch
            );
            println!("\nSoftware:");
            println!("  ID: {}", manifest.software.id);
            println!("  Name: {}", manifest.software.name);
            if let Some(version) = manifest.software.version {
                println!("  Version: {}", version);
            }

            if !manifest.dependencies.is_empty() {
                println!("\nIncluded Dependencies:");
                for dep in &manifest.dependencies {
                    println!("  - {} ({})", dep.name, dep.path);
                }
            }

            println!("\nFiles: {} files included", manifest.files.len());

            // Calculate total size
            let total_size: u64 = manifest.files.iter().map(|f| f.size).sum();
            println!("Total size: {}", format_size(total_size));
        }
        Err(e) => {
            error!("Bundle verification failed: {}", e);
            println!("\nBundle verification: FAILED");
            println!("Error: {}", e);
            return Err(anyhow::anyhow!("Bundle verification failed: {}", e));
        }
    }

    Ok(())
}

/// Show bundle help
fn show_bundle_help() {
    println!("BurnCloud Bundle Manager");
    println!();
    println!(
        "Manage offline installation bundles for software deployment in air-gapped environments."
    );
    println!();
    println!("Usage:");
    println!("  burncloud bundle create <software>   Create an offline bundle");
    println!("  burncloud bundle verify <bundle>     Verify bundle integrity");
    println!();
    println!("Commands:");
    println!("  create    Create an offline installation bundle");
    println!("  verify    Verify bundle integrity and show information");
    println!();
    println!("Create Options:");
    println!("  <software>    Software ID to bundle (e.g., 'openclaw')");
    println!("  -o, --output  Output directory for the bundle (default: ./bundles)");
    println!();
    println!("Verify Options:");
    println!("  <bundle>    Path to the bundle directory to verify");
    println!();
    println!("Examples:");
    println!("  # Create a bundle for OpenClaw");
    println!("  burncloud bundle create openclaw -o ./offline-bundle");
    println!();
    println!("  # Verify a bundle");
    println!("  burncloud bundle verify ./offline-bundle/openclaw-bundle");
    println!();
    println!("  # Install from bundle (offline mode)");
    println!("  burncloud install openclaw --bundle ./offline-bundle/openclaw-bundle");
}

/// Format file size in human-readable format
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}
