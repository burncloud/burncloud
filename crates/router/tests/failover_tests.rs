mod common;

use common::{setup_db, start_test_server};
use burncloud_database::sqlx;
use reqwest::Client;
use serde_json::Value;

#[tokio::test]
async fn test_failover() -> anyhow::Result<()> {
    // Scenario: Group "failover-group" has 2 members:
    // 1. "dead-node" -> Returns 500 (Mocked by HttpBin /status/500)
    // 2. "alive-node" -> Returns 200 (Mocked by HttpBin /anything)
    //
    // We expect the router to try "dead-node", fail, and then successfully route to "alive-node".
    
    let (_db, pool) = setup_db().await?;

    // 1. Insert Upstreams
    let dead_id = "dead";
    // Use a non-existent domain to trigger Connection Error (which triggers failover)
    let dead_url = "http://dead-node.burncloud.test"; 
    
    let alive_id = "alive";
    let alive_url = "https://httpbin.org/anything"; // Always returns 200

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES 
        (?, 'Dead Node', ?, 'k1', '/failover', 'Bearer'),
        (?, 'Alive Node', ?, 'k2', '/failover', 'Bearer')
        ON CONFLICT(id) DO UPDATE SET base_url = excluded.base_url
        "#
    )
    .bind(dead_id).bind(dead_url)
    .bind(alive_id).bind(alive_url)
    .execute(&pool).await?;

    // 2. Create Group
    let group_id = "g-fail";
    let match_path = "/failover-test";
    
    sqlx::query(
        "INSERT INTO router_groups (id, name, strategy, match_path) VALUES (?, 'Failover Group', 'round_robin', ?) ON CONFLICT(id) DO NOTHING"
    )
    .bind(group_id).bind(match_path)
    .execute(&pool).await?;

    // 3. Bind Members
    // Clean up old members first to avoid duplicates
    sqlx::query("DELETE FROM router_group_members WHERE group_id = ?")
        .bind(group_id)
        .execute(&pool).await?;

    sqlx::query(
        "INSERT INTO router_group_members (group_id, upstream_id, weight) VALUES (?, ?, 1), (?, ?, 1)"
    )
    .bind(group_id).bind(dead_id)
    .bind(group_id).bind(alive_id)
    .execute(&pool).await?;

    // 4. Start Server
    let port = 3015;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}{}", port, match_path);

    // 5. Send Request 1
    // Expected: It might hit Dead first, fail, then hit Alive. Result: 200.
    // Or hit Alive first. Result: 200.
    //
    // To verify failover *actually happened*, we'd need to check logs, but for integration test, 
    // ensuring we get 200 OK is the primary goal (High Availability).
    //
    // We can try sending multiple requests. Since RR rotates start index.
    // Req 1: Start [Dead, Alive] -> Dead(500) -> Alive(200). Success.
    // Req 2: Start [Alive, Dead] -> Alive(200). Success.
    // Req 3: Start [Dead, Alive] -> ...
    
    for i in 1..=3 {
        println!("Request {}", i);
        let resp = client.get(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            .send().await?;
        
        assert_eq!(resp.status(), 200, "Request {} failed to failover", i);
        
        // Verify it actually hit the alive node (HttpBin /anything returns json)
        let json: Value = resp.json().await?;
        let target_url = json["url"].as_str().unwrap();
        assert!(target_url.contains("/anything"), "Should eventually hit the alive node");
    }

    Ok(())
}
