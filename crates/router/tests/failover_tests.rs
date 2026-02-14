mod common;

use burncloud_database::sqlx;
use common::{setup_db, start_mock_upstream, start_test_server};
use reqwest::Client;

#[tokio::test]
async fn test_failover() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // Start Mock Upstream for Alive Node
    let mock_port = 3023;
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", mock_port))
        .await
        .unwrap();
    tokio::spawn(async move {
        start_mock_upstream(listener).await;
    });

    // 1. Create Upstreams
    let dead_id = "dead";
    let dead_url = "http://dead-node.burncloud.test"; // Should resolve to nothing or fail connect

    let alive_id = "alive";
    let alive_url = format!("http://127.0.0.1:{}/anything", mock_port);

    sqlx::query(
        r#"
        INSERT INTO router_upstreams (id, name, base_url, api_key, match_path, auth_type)
        VALUES 
        (?, 'Dead Node', ?, 'k1', '/dead', 'Bearer'),
        (?, 'Alive Node', ?, 'k2', '/alive', 'Bearer')
        ON CONFLICT(id) DO UPDATE SET base_url=excluded.base_url, name=excluded.name, api_key=excluded.api_key, match_path=excluded.match_path, auth_type=excluded.auth_type
        "#,
    )
    .bind(dead_id)
    .bind(dead_url)
    .bind(alive_id)
    .bind(alive_url)
    .execute(&pool)
    .await?;

    // 2. Create Group
    let group_id = "failover_group";
    let match_path = "/failover-test";

    sqlx::query(
        "INSERT INTO router_groups (id, name, strategy, match_path) VALUES (?, 'Failover Group', 'round_robin', ?) ON CONFLICT(id) DO UPDATE SET name=excluded.name, strategy=excluded.strategy, match_path=excluded.match_path"
    )
    .bind(group_id).bind(match_path)
    .execute(&pool).await?;

    // 3. Bind
    sqlx::query("DELETE FROM router_group_members WHERE group_id = ?")
        .bind(group_id)
        .execute(&pool)
        .await?;
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

    // 5. Send Requests
    // We expect some requests to hit Dead Node first (Round Robin), fail, and then hit Alive Node.
    // Some will hit Alive Node directly.
    // All requests should eventually succeed (200 OK).

    for i in 1..=4 {
        println!("Request {}", i);
        let resp = client
            .get(&url)
            .header("Authorization", "Bearer sk-burncloud-demo")
            // Need valid JSON body for ProxyLogic!
            .json(&serde_json::json!({"test": "failover"}))
            .send()
            .await?;

        assert_eq!(resp.status(), 200, "Request {} failed to failover", i);
    }

    Ok(())
}
