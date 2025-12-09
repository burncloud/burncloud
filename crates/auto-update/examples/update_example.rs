//! Example demonstrating the correct usage of self_update crate v0.4.5
//!
//! This example shows how to:
//! 1. Check for available updates without performing them
//! 2. Get latest release information
//! 3. Properly use the github::Update struct

use burncloud_auto_update::{AutoUpdater, UpdateConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    env_logger::init();

    // Create updater with custom config
    let config = UpdateConfig {
        current_version: "0.1.0".to_string(),
        github_owner: "burncloud".to_string(),
        github_repo: "burncloud".to_string(),
        bin_name: "burncloud".to_string(),
        ..UpdateConfig::default()
    };

    let updater = AutoUpdater::new(config);

    // Example 1: Check if updates are available (simple check)
    println!("=== Example 1: Basic Update Check ===");
    match updater.sync_check_for_updates() {
        Ok(has_update) => {
            if has_update {
                println!("✅ Update available!");
            } else {
                println!("✅ Already up to date");
            }
        }
        Err(e) => println!("❌ Check failed: {}", e),
    }

    // Example 2: Get latest release information
    println!("\n=== Example 2: Get Release Information ===");
    match updater.get_latest_release_info() {
        Ok(Some((tag, name))) => {
            println!("Latest release:");
            println!("  Tag: {}", tag);
            println!("  Name: {}", name);
        }
        Ok(None) => println!("No releases found"),
        Err(e) => println!("❌ Failed to get release info: {}", e),
    }

    // Example 3: Smart version comparison
    println!("\n=== Example 3: Smart Version Comparison ===");
    match updater.needs_update() {
        Ok(needs_update) => {
            if needs_update {
                println!("✅ Newer version available (using semver comparison)");
            } else {
                println!("✅ Current version is up to date");
            }
        }
        Err(e) => println!("❌ Version check failed: {}", e),
    }

    // Example 4: Get download links for manual update
    println!("\n=== Example 4: Manual Download Links ===");
    let (github_url, gitee_url) = updater.get_download_links();
    println!("GitHub releases: {}", github_url);
    println!("Gitee releases: {}", gitee_url);

    // Example 5: Perform actual update (commented out for safety)
    /*
    println!("\n=== Example 5: Perform Update ===");
    match updater.sync_update() {
        Ok(_) => println!("✅ Update completed successfully"),
        Err(e) => println!("❌ Update failed: {}", e),
    }
    */

    Ok(())
}
