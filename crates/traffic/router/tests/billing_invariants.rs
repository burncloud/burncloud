#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

mod common;

use burncloud_database::sqlx;
use burncloud_database_router::{RouterDatabase, RouterLog, RouterTokenModel};
use common::{insert_router_token, setup_db};

fn test_log(user_id: &str) -> RouterLog {
    RouterLog {
        id: 0,
        request_id: uuid::Uuid::new_v4().to_string(),
        user_id: Some(user_id.to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("1".to_string()),
        status_code: 200,
        latency_ms: 10,
        prompt_tokens: 100,
        completion_tokens: 50,
        cost: 25_000_000,
        model: Some("test-model".to_string()),
        cache_read_tokens: 0,
        reasoning_tokens: 0,
        pricing_region: None,
        video_tokens: 0,
        cache_write_tokens: 0,
        audio_input_tokens: 0,
        audio_output_tokens: 0,
        image_tokens: 0,
        embedding_tokens: 0,
        input_cost: 10_000_000,
        output_cost: 15_000_000,
        cache_read_cost: 0,
        cache_write_cost: 0,
        audio_cost: 0,
        image_cost: 0,
        video_cost: 0,
        reasoning_cost: 0,
        embedding_cost: 0,
        layer_decision: None,
        traffic_color: None,
        cost_status: Some("ok".to_string()),
        error_type: None,
        created_at: None,
    }
}

#[tokio::test]
async fn router_log_insert_never_mutates_spend_quota() -> anyhow::Result<()> {
    let (db, pool, _db_url) = setup_db().await?;

    for (token, used) in [("bc_live_log_a", 100_i64), ("bc_live_log_b", 200_i64)] {
        sqlx::query(
            "INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota) VALUES (?, 'same-user', 'active', 1000000000, ?)",
        )
        .bind(token)
        .bind(used)
        .execute(&pool)
        .await?;
    }

    RouterDatabase::insert_log(&db, &test_log("same-user")).await?;

    let a: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("bc_live_log_a")
        .fetch_one(&pool)
        .await?;
    let b: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("bc_live_log_b")
        .fetch_one(&pool)
        .await?;

    assert_eq!(a, 100, "usage logging must not settle spend for token A");
    assert_eq!(b, 200, "usage logging must not settle spend for token B");
    Ok(())
}

#[tokio::test]
async fn spend_settlement_is_scoped_to_the_presented_router_token() -> anyhow::Result<()> {
    let (db, pool, _db_url) = setup_db().await?;

    for token in ["bc_live_scope_a", "bc_live_scope_b"] {
        sqlx::query(
            "INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota) VALUES (?, 'same-user', 'active', 100, 0)",
        )
        .bind(token)
        .execute(&pool)
        .await?;
    }

    assert!(RouterDatabase::deduct_quota(&db, "same-user", "bc_live_scope_a", 40).await?);

    let a: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("bc_live_scope_a")
        .fetch_one(&pool)
        .await?;
    let b: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("bc_live_scope_b")
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        (a, b),
        (40, 0),
        "one key must never consume another key's quota"
    );

    assert!(!RouterDatabase::deduct_quota(&db, "same-user", "bc_live_scope_a", 70).await?);
    let a: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind("bc_live_scope_a")
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        a, 110,
        "actual over-cap spend must still be durably settled"
    );

    Ok(())
}

#[tokio::test]
async fn user_api_key_settlement_is_key_scoped() -> anyhow::Result<()> {
    let (db, pool, _db_url) = setup_db().await?;

    insert_router_token(&db, "sk-legacy-a", "legacy-user", "default", None, None).await?;
    insert_router_token(&db, "sk-legacy-b", "legacy-user", "default", None, None).await?;

    sqlx::query("UPDATE user_api_keys SET remain_quota = 100, used_quota = 0 WHERE user_id = ?")
        .bind("legacy-user")
        .execute(&pool)
        .await?;

    assert!(RouterDatabase::deduct_quota(&db, "legacy-user", "sk-legacy-a", 25).await?);

    let a: i64 = sqlx::query_scalar("SELECT used_quota FROM user_api_keys WHERE key = ?")
        .bind("sk-legacy-a")
        .fetch_one(&pool)
        .await?;
    let b: i64 = sqlx::query_scalar("SELECT used_quota FROM user_api_keys WHERE key = ?")
        .bind("sk-legacy-b")
        .fetch_one(&pool)
        .await?;
    assert_eq!((a, b), (25, 0));

    Ok(())
}

#[tokio::test]
async fn rotated_old_key_settles_against_the_current_key() -> anyhow::Result<()> {
    let (db, pool, _db_url) = setup_db().await?;

    sqlx::query(
        "INSERT INTO router_tokens (token, user_id, status, quota_limit, used_quota, key_version, key_prefix) VALUES (?, 'rotate-user', 'active', 100, 0, 1, 'bc_live_')",
    )
    .bind("bc_live_old")
    .execute(&pool)
    .await?;

    let rotation = RouterTokenModel::rotate(&db, "bc_live_old", 1, false).await?;
    assert!(RouterDatabase::deduct_quota(&db, "rotate-user", "bc_live_old", 30).await?);

    let used: i64 = sqlx::query_scalar("SELECT used_quota FROM router_tokens WHERE token = ?")
        .bind(&rotation.new_token)
        .fetch_one(&pool)
        .await?;
    assert_eq!(
        used, 30,
        "transition keys must not create an unmetered billing path"
    );

    Ok(())
}
