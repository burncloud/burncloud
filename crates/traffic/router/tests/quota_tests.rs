#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::unnecessary_cast,
    clippy::let_and_return,
    clippy::redundant_pattern_matching
)]

mod common;

use burncloud_database::sqlx;
use burncloud_database_router::RouterDatabase;
use common::setup_db;
use std::time::Duration;

/// Spend quota is measured in nanodollars and actual cost is always settled.
#[tokio::test]
async fn test_quota_deduction() -> anyhow::Result<()> {
    let (db, pool, _db_url) = setup_db().await?;

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

    tokio::time::sleep(Duration::from_millis(50)).await;

    let result =
        RouterDatabase::deduct_quota(&db, "test-quota-user", "sk-test-quota-token", 50).await?;
    assert!(
        result,
        "Settlement within the spend cap should report available"
    );

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-quota-token")
        .fetch_one(&pool)
        .await?;
    assert_eq!(used, 50, "used_quota is settled nanodollar spend");

    let result =
        RouterDatabase::deduct_quota(&db, "test-quota-user", "sk-test-quota-token", 50).await?;
    assert!(
        result,
        "Settlement reaching the cap should still report available"
    );

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-quota-token")
        .fetch_one(&pool)
        .await?;
    assert_eq!(used, 100);

    // The final request may cross the cap because the exact cost is only known
    // after the upstream response. Its real cost must still be recorded, while
    // the return value tells the caller the credential is now exhausted.
    let result =
        RouterDatabase::deduct_quota(&db, "test-quota-user", "sk-test-quota-token", 1).await?;
    assert!(!result, "Over-cap settlement should report quota exhausted");

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-quota-token")
        .fetch_one(&pool)
        .await?;
    assert_eq!(used, 101, "actual spend must never be silently dropped");

    let result = RouterDatabase::check_quota(&db, "sk-test-quota-token", 1).await?;
    assert!(
        !result,
        "An exhausted credential must fail the next pre-check"
    );

    Ok(())
}

/// Unlimited spend quota remains unlimited but settlement is still recorded.
#[tokio::test]
async fn test_unlimited_quota() -> anyhow::Result<()> {
    let (db, pool, _db_url) = setup_db().await?;

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

    let result = RouterDatabase::deduct_quota(
        &db,
        "test-unlimited-user",
        "sk-test-unlimited-token",
        1_000_000,
    )
    .await?;
    assert!(
        result,
        "Unlimited token should always report quota available"
    );

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-unlimited-token")
        .fetch_one(&pool)
        .await?;
    assert_eq!(used, 1_000_000, "unlimited does not mean unmetered");

    Ok(())
}

/// Quota checks are read-only and missing credentials fail closed.
#[tokio::test]
async fn test_quota_check() -> anyhow::Result<()> {
    let (db, pool, _db_url) = setup_db().await?;

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

    let result = RouterDatabase::check_quota(&db, "sk-test-check-token", 40).await?;
    assert!(result, "Should have enough spend quota");

    let result = RouterDatabase::check_quota(&db, "sk-test-check-token", 60).await?;
    assert!(!result, "Should not have enough spend quota");

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("sk-test-check-token")
        .fetch_one(&pool)
        .await?;
    assert_eq!(used, 50, "quota check must not mutate settlement state");

    assert!(
        !RouterDatabase::check_quota(&db, "missing-token", 1).await?,
        "unknown credentials must fail closed"
    );
    assert!(
        !RouterDatabase::deduct_quota(&db, "test-check-user", "missing-token", 1).await?,
        "unknown credentials must not report a successful settlement"
    );

    Ok(())
}
