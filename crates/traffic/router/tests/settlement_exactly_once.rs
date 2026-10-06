#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]

mod common;

use common::{insert_router_token, insert_test_channel, setup_db};
use std::time::Duration;

const MODEL: &str = "settlement-exactly-once-model";

async fn seed_price(db: &burncloud_database::Database) -> anyhow::Result<()> {
    use burncloud_commerce_contracts::price_u64::dollars_to_nano;
    use burncloud_database_billing::{BillingPriceModel, PriceInput};

    BillingPriceModel::upsert(
        db,
        &PriceInput {
            model: MODEL.to_string(),
            currency: "USD".to_string(),
            input_price: dollars_to_nano(1.0),
            output_price: dollars_to_nano(1.0),
            cache_read_input_price: None,
            cache_creation_input_price: None,
            batch_input_price: None,
            batch_output_price: None,
            priority_input_price: None,
            priority_output_price: None,
            audio_input_price: None,
            audio_output_price: None,
            reasoning_price: None,
            embedding_price: None,
            image_price: None,
            video_price: None,
            music_price: None,
            source: Some("settlement-test".to_string()),
            region: None,
            context_window: None,
            max_output_tokens: None,
            supports_vision: None,
            supports_function_calling: None,
            voices_pricing: None,
            video_pricing: None,
            asr_pricing: None,
            realtime_pricing: None,
            model_type: None,
        },
    )
    .await?;
    Ok(())
}

async fn grant_quota(
    db: &burncloud_database::Database,
    token: &str,
    user_id: &str,
) -> anyhow::Result<()> {
    let pool = db.get_connection()?.pool();
    let updated = burncloud_database::sqlx::query(
        "UPDATE user_api_keys SET remain_quota = 1000000000, used_quota = 0 WHERE key = ?",
    )
    .bind(token)
    .execute(pool)
    .await?
    .rows_affected();
    assert_eq!(
        updated, 1,
        "the test credential must exist in user_api_keys"
    );

    burncloud_database::sqlx::query(
        "INSERT OR REPLACE INTO router_tokens \
         (token, user_id, status, quota_limit, used_quota, expired_time, accessed_time) \
         VALUES (?, ?, 'active', 1000000000, 0, -1, 0)",
    )
    .bind(token)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}

async fn current_quota(pool: &burncloud_database::sqlx::AnyPool, token: &str) -> i64 {
    burncloud_database::sqlx::query_scalar::<_, i64>(
        "SELECT used_quota FROM router_tokens WHERE token = ?",
    )
    .bind(token)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .unwrap_or(0)
}

async fn wait_for_positive_stable_quota(
    pool: &burncloud_database::sqlx::AnyPool,
    token: &str,
) -> i64 {
    let mut last = 0;
    let mut stable = 0;
    for _ in 0..200 {
        let value = current_quota(pool, token).await;
        if value > 0 && value == last {
            stable += 1;
            if stable >= 3 {
                return value;
            }
        } else {
            stable = 0;
            last = value;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    current_quota(pool, token).await
}

async fn wait_for_log_cost(pool: &burncloud_database::sqlx::AnyPool, user_id: &str) -> i64 {
    for _ in 0..100 {
        let cost = burncloud_database::sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(cost, 0) FROM router_logs WHERE user_id = ? AND model = ? ORDER BY id DESC LIMIT 1",
        )
        .bind(user_id)
        .bind(MODEL)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .unwrap_or(0);
        if cost > 0 {
            return cost;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    0
}

fn reserve_ephemeral_port() -> u16 {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap_or_else(|e| panic!("must be able to reserve an ephemeral port: {e}"));
    listener.local_addr().expect("local address").port()
}

#[tokio::test]
async fn a_served_request_is_debited_once_and_matches_the_billing_log() -> anyhow::Result<()> {
    let (db, pool, db_url) = setup_db().await?;
    seed_price(&db).await?;

    let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let upstream_port = upstream_listener.local_addr()?.port();
    tokio::spawn(common::start_mock_upstream(upstream_listener));

    insert_test_channel(
        &pool,
        1,
        "settlement-exactly-once-channel",
        &format!("http://127.0.0.1:{upstream_port}"),
        "sk-upstream-test-only",
        MODEL,
        "default",
    )
    .await?;

    let router_port = reserve_ephemeral_port();
    common::start_test_server(router_port, &db_url).await;

    let token = "sk-settlement-exactly-once";
    let user_id = "settlement-exactly-once-user";
    insert_router_token(&db, token, user_id, "default", Some("value"), None).await?;
    grant_quota(&db, token, user_id).await?;
    let before = current_quota(&pool, token).await;
    assert_eq!(before, 0);

    let response = reqwest::Client::new()
        .post(format!(
            "http://127.0.0.1:{router_port}/v1/chat/completions"
        ))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": MODEL,
            "messages": [{"role": "user", "content": "hello"}]
        }))
        .send()
        .await?;

    let status = response.status();
    let body = response.text().await?;
    assert!(
        status.is_success(),
        "request must reach the mock upstream, got {status}: {body}"
    );
    let parsed: serde_json::Value = serde_json::from_str(&body)?;
    assert_eq!(
        parsed["choices"][0]["message"]["content"], "mock answer",
        "success must be the upstream completion, not a local synthetic response"
    );

    let after = wait_for_positive_stable_quota(&pool, token).await;
    assert!(
        after > before,
        "served request must debit a non-zero amount"
    );

    let logged_cost = wait_for_log_cost(&pool, user_id).await;
    assert!(
        logged_cost > 0,
        "served request must produce a priced router log"
    );
    assert_eq!(
        after - before,
        logged_cost,
        "ledger debit and immutable billing log must agree exactly"
    );

    // Settlement is asynchronous, so give any accidental replay enough time to
    // show itself. A second debit would make this assertion fail.
    tokio::time::sleep(Duration::from_millis(500)).await;
    let after_delay = current_quota(&pool, token).await;
    assert_eq!(
        after_delay, after,
        "quota must stop moving after the single settlement; a replay would double-bill"
    );

    let log_count = burncloud_database::sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM router_logs WHERE user_id = ? AND model = ?",
    )
    .bind(user_id)
    .bind(MODEL)
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        log_count, 1,
        "one request must create exactly one billing log row"
    );

    Ok(())
}
