#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::panic_in_result_fn,
    reason = "integration tests: a failed assertion is the intended failure signal"
)]

mod common;

use axum::http::StatusCode;
use burncloud_database::sqlx;
use common::{insert_router_token, insert_test_channel, setup_db, start_test_server};
use std::time::Duration;

async fn seed_price(db: &burncloud_database::Database, model: &str) -> anyhow::Result<()> {
    use burncloud_commerce_contracts::price_u64::dollars_to_nano;
    use burncloud_database_billing::{BillingPriceModel, PriceInput};

    let input = PriceInput {
        model: model.to_string(),
        input_price: dollars_to_nano(1.0),
        output_price: dollars_to_nano(1.0),
        currency: "USD".to_string(),
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
        source: None,
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
    };
    BillingPriceModel::upsert(db, &input).await?;
    assert!(
        BillingPriceModel::get(db, model, "USD", None).await?.is_some(),
        "seeded price must be readable through the production lookup"
    );
    Ok(())
}

async fn grant_unlimited_quota(pool: &sqlx::AnyPool, key: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE user_api_keys SET remain_quota = 1000000000 WHERE key = ?")
        .bind(key)
        .execute(pool)
        .await?;
    Ok(())
}

async fn post_chat(
    port: u16,
    token: &str,
    model: &str,
) -> anyhow::Result<reqwest::Response> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    Ok(client
        .post(format!("http://127.0.0.1:{port}/v1/chat/completions"))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": 32,
            "messages": [{"role": "user", "content": "reason probe"}],
        }))
        .send()
        .await?)
}

fn error_message(body: &serde_json::Value) -> &str {
    body.get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .expect("error.message must be present")
}

/// #662: a real connection failure must not fall through to
/// `All upstreams failed. Last error: ` with an empty suffix.
///
/// Mutation evidence: deleting the network-path `last_error = ...` assignment
/// makes this assertion fail because the required category/channel text
/// disappears from the final 502.
#[tokio::test]
async fn all_upstreams_failed_reports_network_reason_and_candidate() -> anyhow::Result<()> {
    let (db, pool, db_url) = setup_db().await?;
    let model = "reason-network-model";
    let token = "tok-reason-network";
    let channel_id = 9_662;

    seed_price(&db, model).await?;
    insert_test_channel(
        &pool,
        channel_id,
        "reason-network",
        "http://127.0.0.1:1",
        "k-network",
        model,
        "default",
    )
    .await?;
    insert_router_token(&db, token, "u-reason-network", "default", None, None).await?;
    grant_unlimited_quota(&pool, token).await?;

    let router_port = 16_621_u16;
    start_test_server(router_port, &db_url).await;

    let resp = post_chat(router_port, token, model).await?;
    assert_eq!(resp.status().as_u16(), 502);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(
        body["error"]["code"], "all_upstreams_failed",
        "client error-code contract must stay unchanged"
    );
    let message = error_message(&body);
    assert!(
        message.contains(&format!(
            "Network error on channel {channel_id} (reason-network):"
        )),
        "network failover reason must include category and candidate identity, got: {message}"
    );
    assert_ne!(
        message.trim_end(),
        "All upstreams failed. Last error:",
        "Last error suffix must never be empty"
    );

    Ok(())
}

/// #662: an upstream HTTP failure must preserve the status category and the
/// identity of the final candidate in the final 502.
///
/// Mutation evidence: deleting the 5xx-path `last_error = ...` assignment
/// makes the final message lose the required status/channel text.
#[tokio::test]
async fn all_upstreams_failed_reports_upstream_status_and_candidate() -> anyhow::Result<()> {
    let upstream_listener = tokio::net::TcpListener::bind("127.0.0.1:16623").await?;
    tokio::spawn(async move {
        let app = axum::Router::new().fallback(|| async {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(serde_json::json!({"error": {"message": "upstream exploded"}})),
            )
        });
        axum::serve(upstream_listener, app)
            .await
            .expect("mock 500 upstream must stay available");
    });

    let (db, pool, db_url) = setup_db().await?;
    let model = "reason-status-model";
    let token = "tok-reason-status";
    let channel_id = 9_663;

    seed_price(&db, model).await?;
    insert_test_channel(
        &pool,
        channel_id,
        "reason-status",
        "http://127.0.0.1:16623",
        "k-status",
        model,
        "default",
    )
    .await?;
    insert_router_token(&db, token, "u-reason-status", "default", None, None).await?;
    grant_unlimited_quota(&pool, token).await?;

    let router_port = 16_622_u16;
    start_test_server(router_port, &db_url).await;

    let resp = post_chat(router_port, token, model).await?;
    assert_eq!(resp.status().as_u16(), 502);

    let body: serde_json::Value = resp.json().await?;
    assert_eq!(body["error"]["code"], "all_upstreams_failed");
    let message = error_message(&body);
    assert!(
        message.contains(&format!(
            "Upstream status on channel {channel_id} (reason-status): 500 Internal Server Error"
        )),
        "upstream status failure must include status and candidate identity, got: {message}"
    );

    Ok(())
}
