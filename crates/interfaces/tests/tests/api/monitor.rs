#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::let_unit_value,
    clippy::redundant_pattern,
    clippy::manual_is_multiple_of,
    clippy::let_and_return,
    clippy::to_string_trait_impl,
    clippy::to_string_in_format_args,
    clippy::redundant_pattern_matching,
    reason = "integration-test code: panicking on fixture failures is the intended signal, and fixtures use plain Value formatting and patterns"
)]
use crate::common::{admin_client, spawn_app};

#[tokio::test]
async fn test_get_system_metrics() -> anyhow::Result<()> {
    // 1. Start Server
    // Ensure we are using a fresh server instance (mock or spawned)
    let base_url = spawn_app().await;

    // /console/api/monitor is an administrator-only management-plane route.
    // Use the real Console JWT bootstrapped by the isolated test fixture; a
    // data-plane API key such as sk-burncloud-demo must not authenticate here.
    let client = admin_client(&base_url).await;

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
