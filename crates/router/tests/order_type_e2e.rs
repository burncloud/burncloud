//! End-to-end coverage for the L1 Classifier integration (issue #150 / FU-1).
//!
//! Exercises the full L1 data flow: `router_tokens.order_type` /
//! `price_cap_nanodollars` → `validate_token_and_get_info` → `OrderType`
//! variant → `OrderType::filter_candidates` behavior, plus the
//! `SchedulingRequest` field round-trip (color from `UserService`, user_id
//! reaching L3 Affinity HRW).
//!
//! - T1: budget + price_cap=1000 filters out expensive channels (incl. None price)
//! - T2: `UserService::resolve_traffic_class` result lands in `SchedulingRequest.color`
//! - T3: `order_type='budget'` + price_cap=NULL → Value default (no Budget poseur)
//! - T4: legacy token with no `router_tokens` row → Value default
//! - T5: `user_id` reaches the L3 Affinity HRW key (deterministic stickiness)

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

mod common;

use burncloud_common::types::Channel;
use burncloud_common::TrafficColor;
use burncloud_database_router::RouterDatabase;
use burncloud_router::affinity::pick_hrw;
use burncloud_router::order_type::OrderType;
use burncloud_router::SchedulingRequest;
use burncloud_service_user::UserService;

use common::{insert_router_token, setup_db};

/// Minimal `Channel` for OrderType filter / Affinity tests.
fn channel(id: i32, weight: i32) -> (Channel, i32) {
    (
        Channel {
            id,
            type_: 1,
            key: format!("key-{id}"),
            status: 1,
            name: format!("ch-{id}"),
            weight,
            created_time: None,
            test_time: None,
            response_time: None,
            base_url: Some(format!("https://ch{id}.example.com")),
            models: String::new(),
            group: "default".to_string(),
            used_quota: 0,
            model_mapping: None,
            priority: 0,
            auto_ban: 0,
            other_info: None,
            tag: None,
            setting: None,
            param_override: None,
            header_override: None,
            remark: None,
            api_version: None,
            pricing_region: None,
        },
        weight,
    )
}

/// T1 — budget token with `price_cap_nanodollars=1000` filters out channels
/// priced above the cap. Includes one channel with `None` price to verify the
/// conservative-exclusion contract documented in
/// [`OrderType::filter_candidates`] (unknown price + a non-MAX cap → exclude).
#[tokio::test]
async fn t1_budget_filters_expensive_channels_and_excludes_none_priced() -> anyhow::Result<()> {
    let (db, _pool, _url) = setup_db().await?;
    insert_router_token(
        &db,
        "tok-budget",
        "u-budget",
        "default",
        Some("budget"),
        Some(1000),
    )
    .await?;

    let info = RouterDatabase::validate_token_and_get_info(&db, "tok-budget")
        .await?
        .expect("token should validate");
    assert_eq!(info.order_type.as_deref(), Some("budget"));
    assert_eq!(info.price_cap, Some(1000));

    let order = OrderType::from_db_row(info.order_type.as_deref(), info.price_cap);
    assert_eq!(
        order,
        OrderType::Budget {
            max_price_nanodollars: 1000
        }
    );

    // Three channels: id=1 cheap (500), id=2 expensive (2000), id=3 unknown (None).
    let candidates = vec![channel(1, 10), channel(2, 10), channel(3, 10)];
    let price_of = |ch: &Channel| -> Option<i64> {
        match ch.id {
            1 => Some(500),
            2 => Some(2000),
            3 => None,
            _ => None,
        }
    };

    let kept = order.filter_candidates(candidates, price_of);
    let kept_ids: Vec<i32> = kept.iter().map(|(c, _)| c.id).collect();
    assert_eq!(
        kept_ids,
        vec![1],
        "only the cheap channel survives; expensive (>cap) and unknown-priced both excluded"
    );

    Ok(())
}

/// T2 — `UserService::resolve_traffic_class` result feeds
/// `SchedulingRequest.color`. MVP returns Yellow for every user (decision D10);
/// this test pins the field round-trip so a future Trader Class refactor that
/// breaks the SchedulingRequest field wiring fails loudly.
#[tokio::test]
async fn t2_resolve_traffic_class_lands_in_scheduling_request() -> anyhow::Result<()> {
    let (db, _pool, _url) = setup_db().await?;
    insert_router_token(
        &db,
        "tok-color",
        "u-color",
        "default",
        Some("value"),
        None,
    )
    .await?;

    let color = UserService::resolve_traffic_class(&db, "u-color").await?;
    assert_eq!(color, TrafficColor::Yellow, "MVP: every user maps to Yellow");

    let req = SchedulingRequest {
        user_id: Some("u-color".into()),
        color,
        order_type: OrderType::default(),
        session_id: None,
    };
    assert_eq!(req.color, TrafficColor::Yellow);

    Ok(())
}

/// T3 — `order_type='budget'` with NULL `price_cap` falls back to Value default,
/// not `Budget { max_price_nanodollars: i64::MAX }`. Keeps logs honest: a
/// Budget label with no cap would claim enforcement that never happens.
#[tokio::test]
async fn t3_budget_with_null_cap_falls_back_to_value_default() -> anyhow::Result<()> {
    let (db, _pool, _url) = setup_db().await?;
    insert_router_token(
        &db,
        "tok-no-cap",
        "u-no-cap",
        "default",
        Some("budget"),
        None,
    )
    .await?;

    let info = RouterDatabase::validate_token_and_get_info(&db, "tok-no-cap")
        .await?
        .expect("token should validate");
    assert_eq!(info.order_type.as_deref(), Some("budget"));
    assert_eq!(info.price_cap, None);

    let order = OrderType::from_db_row(info.order_type.as_deref(), info.price_cap);
    assert_eq!(order, OrderType::default());
    assert_eq!(order.as_label(), "value");

    Ok(())
}

/// T4 — legacy token with no `router_tokens` row at all (LEFT JOIN yields
/// `(None, None)`) maps to Value default. Validates the LEFT JOIN
/// backward-compat contract for tokens predating migration 0011.
#[tokio::test]
async fn t4_legacy_token_no_router_tokens_row_maps_to_value_default() -> anyhow::Result<()> {
    let (db, _pool, _url) = setup_db().await?;
    insert_router_token(&db, "tok-legacy", "u-legacy", "default", None, None).await?;

    let info = RouterDatabase::validate_token_and_get_info(&db, "tok-legacy")
        .await?
        .expect("token should validate via user_api_keys join even without router_tokens");
    assert_eq!(info.order_type, None);
    assert_eq!(info.price_cap, None);

    let order = OrderType::from_db_row(info.order_type.as_deref(), info.price_cap);
    assert_eq!(order, OrderType::default());

    Ok(())
}

/// T5 — `user_id` carried by `SchedulingRequest` becomes the L3 Affinity HRW
/// key. Repeated `pick_hrw` calls with the same key + healthy candidates must
/// be deterministic — the prerequisite for the blueprint MVP target
/// "fixed user_id hits the same channel ≥ 9/10 times".
#[tokio::test]
async fn t5_user_id_drives_affinity_stickiness() {
    let req = SchedulingRequest {
        user_id: Some("u-sticky".into()),
        color: TrafficColor::Yellow,
        order_type: OrderType::default(),
        session_id: None,
    };
    let key = req
        .affinity_key()
        .expect("user_id supplies the affinity key when session_id is None");
    assert_eq!(key, "u-sticky");

    let candidates = vec![channel(1, 10), channel(2, 10)];
    let healthy = |_id: i32| 1.0;

    let first = pick_hrw(key, &candidates, healthy).expect("HRW should pick a candidate");
    for _ in 0..10 {
        let pick = pick_hrw(key, &candidates, healthy).expect("HRW should pick a candidate");
        assert_eq!(pick, first, "fixed user_id must produce stable HRW pick");
    }
}
