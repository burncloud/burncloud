mod common;

use burncloud_database::sqlx;
use common::{setup_db, start_test_server};
use reqwest::Client;
use serde_json::Value;

#[tokio::test]
async fn test_round_robin_balancer() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // 1. Create Upstreams
    let u1_id = "u1";
    let u1_url = "https://httpbin.org/anything/u1";

    let u2_id = "u2";
    let u2_url = "https://httpbin.org/anything/u2";

    // Insert Upstreams
    // Note: We set match_path to something that won't match directly to ensure they are only reached via group
    // Or we can set them to distinct paths.
    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES 
        (?, 'Upstream 1', ?, 'key1', '/u1-direct', 'Bearer'),
        (?, 'Upstream 2', ?, 'key2', '/u2-direct', 'Bearer')
        "#,
    )
    .bind(u1_id)
    .bind(u1_url)
    .bind(u2_id)
    .bind(u2_url)
    .execute(&pool)
    .await?;

    // 2. Create Group
    let group_id = "g1";
    let match_path = "/group-test";

    sqlx::query(
        "INSERT INTO router_groups (id, name, strategy, match_path) VALUES (?, 'Test Group', 'round_robin', ?)"
    )
    .bind(group_id).bind(match_path)
    .execute(&pool).await?;

    // 3. Bind Upstreams to Group
    sqlx::query(
        "INSERT INTO router_group_members (group_id, upstream_id, weight) VALUES (?, ?, 1), (?, ?, 1)"
    )
    .bind(group_id).bind(u1_id)
    .bind(group_id).bind(u2_id)
    .execute(&pool).await?;

    // 4. Start Server
    let port = 3014;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}{}", port, match_path);

    // 5. Send Requests
    let mut hits_u1 = 0;
    let mut hits_u2 = 0;

    for i in 0..4 {
        let resp = client
            .get(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            .send()
            .await?;

        assert_eq!(resp.status(), 200);
        let json: Value = resp.json().await?;
        let target_url = json["url"].as_str().unwrap();

        println!("Request {} hit: {}", i, target_url);

        if target_url.contains("/u1") {
            hits_u1 += 1;
        } else if target_url.contains("/u2") {
            hits_u2 += 1;
        }
    }

    // 6. Verify Distribution
    // Round Robin should be exactly equal for 4 requests with 2 members
    assert_eq!(hits_u1, 2, "Should hit Upstream 1 twice");
    assert_eq!(hits_u2, 2, "Should hit Upstream 2 twice");

    Ok(())
}
