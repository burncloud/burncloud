#![allow(clippy::expect_used, clippy::disallowed_types)]
//! Aliyun ECS Bundle Installation E2E Test
//!
//! Run with: cargo test -p burncloud-tests --test aliyun_e2e_test -- --ignored
//!
//! Requirements:
//! - Aliyun credentials in ~/.aliyun/config.json
//! - burncloud.exe at target/release/burncloud.exe
//! - openclaw-bundle at target/release/openclaw-bundle
//!
//! ## Instance Reuse
//!
//! The test saves instance info to .env.test file for reuse. If the instance is still running,
//! it will be reused instead of creating a new one.
//!
//! ## Full E2E Test
//!
//! ```bash
//! cargo test -p burncloud-tests --test aliyun_e2e_test test_e2e_bundle_installation -- --ignored --nocapture
//! ```
//!
//! ## Manual Cleanup
//!
//! ```bash
//! cargo test -p burncloud-tests --test aliyun_e2e_test test_cleanup_test_instances -- --ignored
//! ```

mod aliyun;

// Re-export for convenience in step modules
pub use aliyun::{AliyunECS, BundleE2ETest};

// Full E2E test (runs all steps automatically)
#[test]
#[ignore]
fn test_e2e_bundle_installation() {
    let mut test = BundleE2ETest::from_config_file_with_region("cn-shenzhen")
        .expect("Failed to create test runner");

    // Setup server (will reuse existing if available)
    test.setup().expect("Server setup failed");
    println!("Server IP: {}", test.public_ip().unwrap());

    // Upload files - use CARGO_MANIFEST_DIR to get project root
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let project_root = std::path::Path::new(&manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("Failed to get project root");

    let cli_path = project_root.join("target/release/burncloud.exe");
    let bundle_path = project_root.join("target/release/openclaw-bundle");

    test.upload_files(cli_path.to_str().unwrap(), bundle_path.to_str().unwrap())
        .expect("File upload failed");

    // Run installation
    test.run_installation().expect("Installation failed");

    // Verify
    let success = test.verify_installation().expect("Verification failed");
    assert!(success, "Installation verification failed");

    println!("=== E2E Test PASSED ===");
    println!("Instance info saved to .env.test for reuse");
    // NOTE: Instance is NOT cleaned up - preserved for reuse
}

// Convenience test for listing instances
#[test]
#[ignore]
fn test_list_instances() {
    let ecs = AliyunECS::from_config_file_with_region("cn-shenzhen")
        .expect("Failed to load Aliyun config");

    let instances = ecs.list_instances().expect("Failed to list instances");

    println!("Found {} instances:", instances.len());
    for inst in &instances {
        let ips = inst
            .public_ip_address
            .as_ref()
            .map(|p| p.ip_address.first().map(|s| s.as_str()).unwrap_or("N/A"))
            .unwrap_or("N/A");
        println!(
            "  {} - {} - {} - {}",
            inst.instance_id, inst.status, ips, inst.instance_name
        );
    }
}

// Show saved test instance info
#[test]
#[ignore]
fn test_show_saved_instance() {
    if let Some(info) = BundleE2ETest::load_saved_instance() {
        println!("Saved test instance info:");
        println!("  Instance ID: {}", info.instance_id);
        println!("  Public IP: {}", info.public_ip);
        println!("  Password: {}", info.password);
    } else {
        println!("No saved instance info found in .env.test");
    }
}

// Cleanup all test instances (manual cleanup)
#[test]
#[ignore]
fn test_cleanup_test_instances() {
    // Clear saved instance info
    BundleE2ETest::clear_saved_instance();

    let ecs = AliyunECS::from_config_file_with_region("cn-shenzhen")
        .expect("Failed to load Aliyun config");

    // Delete all instances with "burncloud" prefix
    let deleted = ecs
        .delete_instances_by_prefix("burncloud", true)
        .expect("Failed to delete instances");
    println!("Deleted {} test instances", deleted);
}
