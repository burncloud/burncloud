mod common;

use burncloud_database::sqlx;
use common::{setup_db, start_test_server};
use reqwest::Client;
use serde_json::json;
use uuid::Uuid;

#[tokio::test]
async fn test_expired_token_returns_401() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // Use unique token name for each test run
    let unique_token = format!("sk-expired-{}", Uuid::new_v4());

    // Create a token that expired 1 hour ago
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expired_time = now - 3600; // 1 hour ago

    // Delete any existing token with same name
    sqlx::query("DELETE FROM router_tokens WHERE token = ?")
        .bind(&unique_token)
        .execute(&pool)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota, expired_time)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&unique_token)
    .bind("test-user-expired")
    .bind("active")
    .bind(-1i64) // unlimited quota
    .bind(0i64)
    .bind(expired_time)
    .execute(&pool)
    .await?;

    let port = 3030;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    // Test with expired token
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", unique_token))
        .json(&json!({"model": "test", "messages": [{"role": "user", "content": "hello"}]}))
        .send()
        .await?;

    // Should return 401 with token_expired error
    assert_eq!(resp.status(), 401);
    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["error"]["code"], "token_expired");
    assert_eq!(body["error"]["message"], "Token has expired");

    Ok(())
}

#[tokio::test]
async fn test_valid_token_with_future_expiry_passes_auth() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // Use unique token name for each test run
    let unique_token = format!("sk-future-{}", Uuid::new_v4());

    // Create a token that expires 1 hour in the future
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;
    let expired_time = now + 3600; // 1 hour in the future

    // Delete any existing token with same name
    sqlx::query("DELETE FROM router_tokens WHERE token = ?")
        .bind(&unique_token)
        .execute(&pool)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota, expired_time)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&unique_token)
    .bind("test-user-future")
        .bind("active")
    .bind(-1i64) // unlimited quota
    .bind(0i64)
    .bind(expired_time)
    .execute(&pool)
    .await?;

    let port = 3031;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    // Test with valid token that has future expiry
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", unique_token))
        .json(&json!({"model": "test", "messages": [{"role": "user", "content": "hello"}]}))
        .send()
        .await?;

    // Token is valid (passed auth), but may fail due to routing/upstream
    // We just need to verify it's NOT 401 with "token_expired"
    let status = resp.status();
    // Should NOT be 401 with token_expired (that's what we're testing)
    // It can be any status other than 401 with token_expired
    if status == 401 {
        let body: serde_json::Value = resp.json().await?;
        // The error should NOT be token_expired
        assert_ne!(body["error"]["code"], "token_expired", "Token with future expiry should not be reported as expired");
    }
    // If status is not 401, the token passed auth validation

    Ok(())
}

#[tokio::test]
async fn test_token_with_never_expire_minus_one_passes_auth() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // Use unique token name for each test run
    let unique_token = format!("sk-never-{}", Uuid::new_v4());

    // Delete any existing token with same name
    sqlx::query("DELETE FROM router_tokens WHERE token = ?")
        .bind(&unique_token)
        .execute(&pool)
        .await?;

    // Create a token with expired_time = -1 (never expires)
    sqlx::query(
        r#"
        INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota, expired_time)
        VALUES (?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&unique_token)
    .bind("test-user-never")
    .bind("active")
    .bind(-1i64) // unlimited quota
    .bind(0i64)
    .bind(-1i64) // never expires
    .execute(&pool)
    .await?;

    let port = 3032;
    start_test_server(port).await;

    let client = Client::new();
    let url = format!("http://localhost:{}/v1/chat/completions", port);

    // Test with never-expire token
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", unique_token))
        .json(&json!({"model": "test", "messages": [{"role": "user", "content": "hello"}]}))
        .send()
        .await?;

    // Token is valid (passed auth), but may fail due to routing/upstream
    // We just need to verify it's NOT 401 with "token_expired"
    let status = resp.status();
    // Should NOT be 401 with token_expired
    if status == 401 {
        let body: serde_json::Value = resp.json().await?;
        // The error should NOT be token_expired
        assert_ne!(body["error"]["code"], "token_expired", "Token with never-expire (-1) should not be reported as expired");
    }
    // If status is not 401, the token passed auth validation

    Ok(())
}
