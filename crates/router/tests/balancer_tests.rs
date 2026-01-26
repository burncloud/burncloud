mod common;

use burncloud_database::sqlx;
use common::{setup_db, start_test_server, start_mock_upstream};
use reqwest::Client;
use serde_json::Value;

#[tokio::test]
async fn test_round_robin_balancer() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // Start Mock Upstream
    let mock_port = 3022;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", mock_port))
        .await
        .unwrap();
    tokio::spawn(async move {
        start_mock_upstream(listener).await;
    });

    // 1. Create Upstreams
    let u1_id = "u1";
    let u1_url = format!("http://127.0.0.1:{}/anything/u1", mock_port);

    let u2_id = "u2";
    let u2_url = format!("http://127.0.0.1:{}/anything/u2", mock_port);

    // Insert Upstreams
    // Note: We set match_path to something that won't match directly to ensure they are only reached via group
    // Or we can set them to distinct paths.
    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES 
        (?, 'Upstream 1', ?, 'key1', '/u1-direct', 'Bearer'),
        (?, 'Upstream 2', ?, 'key2', '/u2-direct', 'Bearer')
        ON CONFLICT(id) DO UPDATE SET 
            base_url = excluded.base_url,
            name = excluded.name,
            api_key = excluded.api_key,
            match_path = excluded.match_path,
            auth_type = excluded.auth_type
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
        "INSERT INTO router_groups (id, name, strategy, match_path) VALUES (?, 'Test Group', 'round_robin', ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, strategy=excluded.strategy, match_path=excluded.match_path"
    )
    .bind(group_id).bind(match_path)
    .execute(&pool).await?;

    // 3. Bind Upstreams to Group
    // For many-to-many, we might need to delete old ones or use upsert if ID exists?
    // router_group_members usually has (group_id, upstream_id) as PK or unique?
    // Let's assume we can delete first or ignore.
    // Or just Try Insert.
    // Let's first delete to be safe if we are reusing DB.
    sqlx::query("DELETE FROM router_group_members WHERE group_id = ?").bind(group_id).execute(&pool).await?;

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
        // Must send JSON body because ProxyLogic expects it, or at least handles it nicely
        // But ProxyLogic only fails if body is invalid JSON *AND* it needs to parse it?
        // Actually, previous debugging showed it returned 502 with "Invalid JSON body".
        // So we MUST send valid JSON.
        let resp = client
            .get(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            .json(&serde_json::json!({"test": "data"})) 
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
