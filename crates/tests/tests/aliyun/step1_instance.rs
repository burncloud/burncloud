//! Step 1: ECS Instance Creation and Management Tests
//!
//! Test creating, listing, and deleting Aliyun ECS instances
//!
//! Run: cargo test -p burncloud-tests --test aliyun_e2e_test -- --ignored 01_instance

use super::*;

/// List all instances in the region
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

/// Create a new test instance
/// Output: instance_id (save for subsequent tests)
#[test]
#[ignore]
fn test_create_instance() {
    let ecs = AliyunECS::from_config_file_with_region("cn-shenzhen")
        .expect("Failed to load Aliyun config");

    let instance_id = ecs
        .create_windows_instance(
            "Burncloud@Test123",
            Some("burncloud-test-01"),
            Some("ecs.g7.large"),
        )
        .expect("Failed to create instance");

    println!("=== INSTANCE CREATED ===");
    println!("Instance ID: {}", instance_id);
    println!("Next step: Run test_wait_for_instance with this ID");
}

/// Wait for instance to be ready and get public IP
/// Set env INSTANCE_ID before running
#[test]
#[ignore]
fn test_wait_for_instance() {
    let instance_id = std::env::var("INSTANCE_ID").expect("Set INSTANCE_ID environment variable");

    let ecs = AliyunECS::from_config_file_with_region("cn-shenzhen")
        .expect("Failed to load Aliyun config");

    let public_ip = ecs
        .wait_for_instance_ready(&instance_id, 600)
        .expect("Failed to wait for instance");

    println!("=== INSTANCE READY ===");
    println!("Instance ID: {}", instance_id);
    println!("Public IP: {}", public_ip);
    println!("Next step: Run test_install_ssh");
}

/// Delete a specific instance by ID
/// Set env INSTANCE_ID before running
#[test]
#[ignore]
fn test_delete_instance() {
    let instance_id = std::env::var("INSTANCE_ID").expect("Set INSTANCE_ID environment variable");

    let ecs = AliyunECS::from_config_file_with_region("cn-shenzhen")
        .expect("Failed to load Aliyun config");

    ecs.delete_instance(&instance_id, true)
        .expect("Failed to delete instance");

    println!("=== INSTANCE DELETED ===");
    println!("Instance ID: {}", instance_id);
}

/// Cleanup all test instances (with "burncloud" prefix)
#[test]
#[ignore]
fn test_cleanup_all_test_instances() {
    let ecs = AliyunECS::from_config_file_with_region("cn-shenzhen")
        .expect("Failed to load Aliyun config");

    let deleted = ecs
        .delete_instances_by_prefix("burncloud", true)
        .expect("Failed to delete instances");

    println!("=== CLEANUP COMPLETE ===");
    println!("Deleted {} instances", deleted);
}
