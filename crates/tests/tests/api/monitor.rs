use crate::common::spawn_app;
use burncloud_tests::TestClient;

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
