//! MN-01: System Monitor API Integration Tests (P2)
//!
//! Tests for verifying system monitoring API endpoints.
//!
//! Key Scenarios:
//! - System metrics API returns proper structure
//! - CPU info is accurate
//! - Memory info is accurate
//! - Disk info is accurate

use crate::common::spawn_app;
use burncloud_tests::TestClient;

/// Test: MN-01 - Get system metrics returns proper structure
#[tokio::test]
async fn test_get_system_metrics() -> anyhow::Result<()> {
    // 1. Start Server
    // Ensure we are using a fresh server instance (mock or spawned)
    let base_url = spawn_app().await;

    let client = TestClient::new(&base_url);

    // We'll try to fetch. If it's 401, we know we need auth.
    // But since it FAILED with 401, let's fix it by assuming we need one.
    // However, burncloud's default setup usually has a demo user.
    // Let's try to add a bearer token.
    // Note: In a real test env, we should create a user/token first, but for now let's try to see if just adding *any* token works if the auth middleware just checks for presence, or if we need a specific one.
    // Actually, looking at `burncloud_database_router::RouterDatabase::init`, it inserts `sk-burncloud-demo`.

    let client = client.with_token("sk-burncloud-demo");

    // 2. GET /console/api/monitor
    let json = client.get("/console/api/monitor").await?;

    // 3. Verify Structure
    assert!(json["success"].as_bool().unwrap_or(false));

    let data = &json["data"];
    assert!(data.is_object());

    // Check CPU info
    let cpu = &data["cpu"];
    assert!(cpu["core_count"].as_u64().unwrap() > 0);
    // usage_percent might be 0.0 initially, so just checking type
    assert!(cpu["usage_percent"].is_number());

    // Check Memory info
    let memory = &data["memory"];
    assert!(memory["total"].as_u64().unwrap() > 0);
    assert!(memory["used"].as_u64().unwrap() > 0);

    // Check Disk info
    let disks = data["disks"].as_array().expect("disks should be an array");
    // CI environments might not return disks in some cases, but usually they do.
    // We'll assert it's an array, and if not empty, check fields.
    if !disks.is_empty() {
        let disk = &disks[0];
        assert!(disk["total"].as_u64().unwrap() > 0);
        assert!(disk["mount_point"].is_string());
    }

    Ok(())
}

/// Test: MN-01 - CPU info structure
#[tokio::test]
async fn test_cpu_info_structure() {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url).with_token("sk-burncloud-demo");

    let json = client.get("/console/api/monitor").await.expect("Failed to get metrics");

    let cpu = &json["data"]["cpu"];

    // Verify CPU fields
    assert!(cpu["usage_percent"].is_number(), "CPU should have 'usage_percent'");
    assert!(cpu["core_count"].is_number(), "CPU should have 'core_count'");
    assert!(cpu["frequency"].is_number(), "CPU should have 'frequency'");
    assert!(cpu["brand"].is_string(), "CPU should have 'brand'");

    // Verify ranges
    let usage = cpu["usage_percent"].as_f64().unwrap_or(-1.0);
    assert!(usage >= 0.0 && usage <= 100.0, "CPU usage should be 0-100%");

    let cores = cpu["core_count"].as_u64().unwrap_or(0);
    assert!(cores > 0, "CPU should have at least 1 core");

    let freq = cpu["frequency"].as_u64().unwrap_or(0);
    // Frequency in MHz - should be reasonable
    assert!(freq > 0 && freq < 10000, "CPU frequency should be reasonable");
}

/// Test: MN-01 - Memory info structure
#[tokio::test]
async fn test_memory_info_structure() {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url).with_token("sk-burncloud-demo");

    let json = client.get("/console/api/monitor").await.expect("Failed to get metrics");

    let memory = &json["data"]["memory"];

    // Verify memory fields
    assert!(memory["total"].is_number(), "Memory should have 'total'");
    assert!(memory["used"].is_number(), "Memory should have 'used'");
    assert!(memory["available"].is_number(), "Memory should have 'available'");
    assert!(memory["usage_percent"].is_number(), "Memory should have 'usage_percent'");

    // Verify values
    let total = memory["total"].as_u64().unwrap_or(0);
    let used = memory["used"].as_u64().unwrap_or(0);
    let available = memory["available"].as_u64().unwrap_or(0);
    let usage_percent = memory["usage_percent"].as_f64().unwrap_or(-1.0);

    assert!(total > 0, "Total memory should be > 0");
    assert!(used <= total, "Used memory should be <= total");
    assert!(available <= total, "Available memory should be <= total");
    assert!(usage_percent >= 0.0 && usage_percent <= 100.0, "Usage percent should be 0-100%");

    // Used + Available should approximately equal total (may have small differences)
    let sum = used + available;
    assert!(sum <= total + 1000, "Used + Available should be approximately equal to total");
}

/// Test: MN-01 - Disk info structure
#[tokio::test]
async fn test_disk_info_structure() {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url).with_token("sk-burncloud-demo");

    let json = client.get("/console/api/monitor").await.expect("Failed to get metrics");

    let disks = json["data"]["disks"].as_array().expect("Disks should be an array");

    // In CI, we might have disks, but it's not guaranteed
    if !disks.is_empty() {
        for disk in disks {
            // Verify disk fields
            assert!(disk["total"].is_number(), "Disk should have 'total'");
            assert!(disk["used"].is_number(), "Disk should have 'used'");
            assert!(disk["available"].is_number(), "Disk should have 'available'");
            assert!(disk["usage_percent"].is_number(), "Disk should have 'usage_percent'");
            assert!(disk["mount_point"].is_string(), "Disk should have 'mount_point'");

            // Verify values
            let total = disk["total"].as_u64().unwrap_or(0);
            let used = disk["used"].as_u64().unwrap_or(0);
            let available = disk["available"].as_u64().unwrap_or(0);
            let usage_percent = disk["usage_percent"].as_f64().unwrap_or(-1.0);

            assert!(total > 0, "Total disk space should be > 0");
            assert!(used <= total, "Used disk space should be <= total");
            assert!(available <= total, "Available disk space should be <= total");
            assert!(usage_percent >= 0.0 && usage_percent <= 100.0, "Usage percent should be 0-100%");
        }
    }
}

/// Test: MN-01 - Timestamp is recent
#[tokio::test]
async fn test_metrics_timestamp() {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url).with_token("sk-burncloud-demo");

    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let json = client.get("/console/api/monitor").await.expect("Failed to get metrics");

    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let timestamp = json["data"]["timestamp"].as_u64().unwrap_or(0);

    // Timestamp should be between before and after
    assert!(timestamp >= before - 1, "Timestamp should be recent");
    assert!(timestamp <= after + 1, "Timestamp should be recent");
}

/// Test: MN-01 - Response success flag
#[tokio::test]
async fn test_monitor_response_success() {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url).with_token("sk-burncloud-demo");

    let json = client.get("/console/api/monitor").await.expect("Failed to get metrics");

    // Response should have success: true
    assert_eq!(json["success"].as_bool().unwrap_or(false), true);
}

/// Test: MN-01 - Multiple requests return updated metrics
#[tokio::test]
async fn test_metrics_update() {
    let base_url = spawn_app().await;
    let client = TestClient::new(&base_url).with_token("sk-burncloud-demo");

    // Get metrics twice
    let json1 = client.get("/console/api/monitor").await.expect("Failed to get metrics");

    // Small delay to allow for potential CPU usage changes
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    let json2 = client.get("/console/api/monitor").await.expect("Failed to get metrics");

    // Both should succeed
    assert!(json1["success"].as_bool().unwrap_or(false));
    assert!(json2["success"].as_bool().unwrap_or(false));

    // Timestamps should be different or same (if very fast)
    let ts1 = json1["data"]["timestamp"].as_u64().unwrap_or(0);
    let ts2 = json2["data"]["timestamp"].as_u64().unwrap_or(0);
    assert!(ts2 >= ts1, "Second timestamp should be >= first");
}
