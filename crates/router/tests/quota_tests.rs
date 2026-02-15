mod common;

use burncloud_database::sqlx;
use burncloud_database_router::RouterDatabase;
use common::setup_db;
use std::time::Duration;

/// Test quota deduction
#[tokio::test]
async fn test_quota_deduction() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // Create a test token with limited quota
    sqlx::query(
        r#"
        INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota)
        VALUES (?, 'test-quota-user', 'active', 100, 0)
        ON CONFLICT(token) DO UPDATE SET quota_limit = 100, used_quota = 0
        "#,
    )
    .bind("sk-test-quota-token")
    .execute(&pool)
    .await?;

    // Small delay to ensure SQLite transaction completes
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Deduct 50 quota
    let result =
        RouterDatabase::deduct_quota(&_db, "test-quota-user", "sk-test-quota-token", 50.0).await?;
    assert!(result, "Deduction should succeed");

    // Small delay
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Check used_quota
    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-quota-token")
        .fetch_one(&pool)
        .await?;

    assert_eq!(used, 50, "used_quota should be 50");

    // Deduct another 50 (should succeed, total 100)
    let result =
        RouterDatabase::deduct_quota(&_db, "test-quota-user", "sk-test-quota-token", 50.0).await?;
    assert!(result, "Second deduction should succeed");

    tokio::time::sleep(Duration::from_millis(50)).await;

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-quota-token")
        .fetch_one(&pool)
        .await?;

    assert_eq!(used, 100, "used_quota should be 100");

    // Try to deduct 1 more (should fail - quota exceeded)
    let result =
        RouterDatabase::deduct_quota(&_db, "test-quota-user", "sk-test-quota-token", 1.0).await?;
    assert!(!result, "Deduction should fail - quota exceeded");

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Verify used_quota didn't change
    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-quota-token")
        .fetch_one(&pool)
        .await?;

    assert_eq!(used, 100, "used_quota should still be 100");

    println!("✓ Quota deduction test passed");

    Ok(())
}

/// Test unlimited quota token
#[tokio::test]
async fn test_unlimited_quota() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // Create a token with unlimited quota
    sqlx::query(
        r#"
        INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota)
        VALUES (?, 'test-unlimited-user', 'active', -1, 0)
        ON CONFLICT(token) DO UPDATE SET quota_limit = -1, used_quota = 0
        "#,
    )
    .bind("sk-test-unlimited-token")
    .execute(&pool)
    .await?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Deduct a large amount
    let result = RouterDatabase::deduct_quota(
        &_db,
        "test-unlimited-user",
        "sk-test-unlimited-token",
        1000000.0,
    )
    .await?;
    assert!(result, "Unlimited token should always succeed");

    println!("✓ Unlimited quota test passed");

    Ok(())
}

/// Test quota check without deduction
#[tokio::test]
async fn test_quota_check() -> anyhow::Result<()> {
    let (_db, pool) = setup_db().await?;

    // Create a token with quota 100
    sqlx::query(
        r#"
        INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota)
        VALUES (?, 'test-check-user', 'active', 100, 50)
        ON CONFLICT(token) DO UPDATE SET quota_limit = 100, used_quota = 50
        "#,
    )
    .bind("sk-test-check-token")
    .execute(&pool)
    .await?;

    tokio::time::sleep(Duration::from_millis(50)).await;

    // Check if 40 quota is available (should pass: 50 used + 40 = 90 < 100)
    let result = RouterDatabase::check_quota(&_db, "sk-test-check-token", 40.0).await?;
    assert!(result, "Should have enough quota");

    // Check if 60 quota is available (should fail: 50 used + 60 = 110 > 100)
    let result = RouterDatabase::check_quota(&_db, "sk-test-check-token", 60.0).await?;
    assert!(!result, "Should not have enough quota");

    // Verify used_quota didn't change
    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-check-token")
        .fetch_one(&pool)
        .await?;

    assert_eq!(used, 50, "used_quota should still be 50");

    println!("✓ Quota check test passed");

    Ok(())
}
