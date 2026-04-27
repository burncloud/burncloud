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
use burncloud_database_router::{RouterDatabase, RouterLog};
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
            rpm_cap: None,
            tpm_cap: None,
            reservation_green: None,
            reservation_yellow: None,
            reservation_red: None,
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

/// T5b — Database round-trip for `layer_decision` and `traffic_color`:
/// INSERT a `RouterLog` with `layer_decision='affinity_hit'` and
/// `traffic_color='Y'`, then SELECT it back and assert both columns
/// survive. Also validates that `RoutingDecision` enum labels and
/// `TrafficColor` chars are non-empty (the values written to the DB
/// must come from valid enum outputs).
#[tokio::test]
async fn t5b_layer_decision_and_traffic_color_db_roundtrip() -> anyhow::Result<()> {
    use burncloud_router::model_router::RoutingDecision;

    // Part A: Enum contract — all RoutingDecision variants produce non-empty
    // labels and TrafficColor chars are non-empty.
    let decisions = [
        RoutingDecision::AffinityHit,
        RoutingDecision::ScorerPicked,
        RoutingDecision::Failover { attempt: 1 },
        RoutingDecision::Failover { attempt: 3 },
    ];
    for d in &decisions {
        let label = d.to_label();
        assert!(
            !label.is_empty(),
            "RoutingDecision::{:?} must produce a non-empty label",
            d
        );
    }
    assert_eq!(RoutingDecision::AffinityHit.to_label(), "affinity_hit");
    assert_eq!(RoutingDecision::ScorerPicked.to_label(), "scorer_picked");
    assert_eq!(
        RoutingDecision::Failover { attempt: 1 }.to_label(),
        "failover_1"
    );
    assert_eq!(
        RoutingDecision::Failover { attempt: 3 }.to_label(),
        "failover_3"
    );
    assert_ne!(
        TrafficColor::Yellow.as_char(),
        '\0',
        "TrafficColor::Yellow must produce a non-empty char for traffic_color column"
    );
    assert_eq!(TrafficColor::Yellow.as_char(), 'Y');

    // Part B: Database round-trip — INSERT a RouterLog with
    // layer_decision='affinity_hit' and traffic_color='Y', then SELECT it
    // back and assert both columns survive the round-trip.
    let (db, pool, _url) = setup_db().await?;
    common::ensure_l6_observability_columns(&pool).await?;

    let log = RouterLog {
        id: 0,
        request_id: "req-t5b".to_string(),
        user_id: Some("u-t5b".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("ch-1".to_string()),
        status_code: 200,
        latency_ms: 42,
        prompt_tokens: 10,
        completion_tokens: 20,
        cost: 1000,
        model: Some("gpt-4".to_string()),
        cache_read_tokens: 0,
        reasoning_tokens: 0,
        pricing_region: None,
        video_tokens: 0,
        cache_write_tokens: 0,
        audio_input_tokens: 0,
        audio_output_tokens: 0,
        image_tokens: 0,
        embedding_tokens: 0,
        input_cost: 0,
        output_cost: 0,
        cache_read_cost: 0,
        cache_write_cost: 0,
        audio_cost: 0,
        image_cost: 0,
        video_cost: 0,
        reasoning_cost: 0,
        embedding_cost: 0,
        layer_decision: Some("affinity_hit".to_string()),
        traffic_color: Some("Y".to_string()),
        created_at: None,
    };
    RouterDatabase::insert_log(&db, &log).await?;

    let rows = RouterDatabase::get_logs(&db, 1, 0).await?;
    assert!(!rows.is_empty(), "should get at least 1 row back");
    let row = &rows[0];
    assert_eq!(
        row.layer_decision.as_deref(),
        Some("affinity_hit"),
        "layer_decision must round-trip as 'affinity_hit'"
    );
    assert!(
        row.traffic_color.is_some(),
        "traffic_color must be present (not NULL) after round-trip"
    );
    assert_eq!(
        row.traffic_color.as_deref(),
        Some("Y"),
        "traffic_color must round-trip as 'Y'"
    );

    Ok(())
}

/// T5c — E2E affinity observability: call `route_with_scheduler` twice with the
/// same `user_id` + `model`, verify that the second call returns
/// `RoutingDecision::AffinityHit`, then construct a `RouterLog` using the
/// decision's `to_label()` and `SchedulingRequest.color.as_char()` as
/// `layer_decision` / `traffic_color`, INSERT it, and SELECT it back —
/// proving the production code path produces the correct observability values.
#[tokio::test]
async fn t5c_affinity_hit_e2e_observability() -> anyhow::Result<()> {
    use burncloud_router::affinity::AffinityCache;
    use burncloud_router::channel_state::ChannelStateTracker;
    use burncloud_router::exchange_rate::ExchangeRateService;
    use burncloud_router::model_router::{ModelRouter, RouteInputs, RoutingDecision};
    use burncloud_router::SchedulingRequest;
    use burncloud_database::sqlx;
    use burncloud_service_billing::PriceCache;

    let (db, pool, _url) = setup_db().await?;
    common::ensure_l6_observability_columns(&pool).await?;

    // Seed channel_providers + channel_abilities so route_with_scheduler
    // can find candidates for the model.
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS channel_providers \
         (id INTEGER PRIMARY KEY, type INTEGER DEFAULT 0, key TEXT NOT NULL, \
          status INTEGER DEFAULT 1, name TEXT, weight INTEGER DEFAULT 0, \
          created_time INTEGER, test_time INTEGER, response_time INTEGER, \
          base_url TEXT DEFAULT '', models TEXT, `group` TEXT DEFAULT 'default', \
          used_quota INTEGER DEFAULT 0, model_mapping TEXT, priority INTEGER DEFAULT 0, \
          auto_ban INTEGER DEFAULT 1, other_info TEXT, tag TEXT, setting TEXT, \
          param_override TEXT, header_override TEXT, remark TEXT, \
          api_version VARCHAR(32) DEFAULT 'default', \
          pricing_region VARCHAR(32) DEFAULT 'international')",
    )
    .execute(&pool)
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS channel_abilities \
         (`group` VARCHAR(64) NOT NULL, model VARCHAR(255) NOT NULL, \
          channel_id INTEGER NOT NULL, enabled INTEGER DEFAULT 1, \
          priority INTEGER DEFAULT 0, weight INTEGER DEFAULT 0, \
          PRIMARY KEY (`group`, model, channel_id))",
    )
    .execute(&pool)
    .await?;

    // Insert two channels for the test model.
    for ch_id in [1, 2] {
        sqlx::query(
            "INSERT INTO channel_providers \
             (id, type, key, status, name, weight, base_url, models, `group`, \
              used_quota, priority, auto_ban, pricing_region) \
             VALUES (?, 1, 'k', 1, ?, 1, ?, ?, 'default', 0, 0, 0, 'international')",
        )
        .bind(ch_id)
        .bind(format!("ch-{ch_id}"))
        .bind(format!("http://127.0.0.1:1{ch_id}"))
        .bind("test-model")
        .execute(&pool)
        .await?;

        sqlx::query(
            "INSERT INTO channel_abilities \
             (`group`, model, channel_id, enabled, priority, weight) \
             VALUES ('default', 'test-model', ?, 1, 0, 1)",
        )
        .bind(ch_id)
        .execute(&pool)
        .await?;
    }

    let db_arc = std::sync::Arc::new(db);
    let model_router = ModelRouter::new(db_arc.clone());
    let affinity_cache = AffinityCache::default();
    let state_tracker = ChannelStateTracker::new();
    let price_cache = PriceCache::empty();
    let exchange_rate = ExchangeRateService::new(db_arc.clone());

    let sched_req = SchedulingRequest {
        user_id: Some("u-affinity-e2e".into()),
        color: TrafficColor::Yellow,
        order_type: OrderType::default(),
        session_id: None,
    };

    // First call: populates the affinity cache via HRW.
    let inputs1 = RouteInputs {
        group: "default",
        model: "test-model",
        state_tracker: &state_tracker,
        price_cache: &price_cache,
        exchange_rate: &exchange_rate,
        scheduler_kind: None,
        request: &sched_req,
        affinity_cache: Some(&affinity_cache),
    };
    let (channels1, decision1) = model_router.route_with_scheduler(inputs1).await?;
    assert!(!channels1.is_empty(), "first call should return candidates");
    // Both first and second calls produce AffinityHit because HRW/cache
    // lookup always returns a channel that gets hoisted.
    assert_eq!(
        decision1,
        Some(RoutingDecision::AffinityHit),
        "first call should produce AffinityHit (HRW pick hoisted)"
    );

    // Second call: cache hit — same user_id + model, affinity cache now
    // populated from the first call's insert.
    let inputs2 = RouteInputs {
        group: "default",
        model: "test-model",
        state_tracker: &state_tracker,
        price_cache: &price_cache,
        exchange_rate: &exchange_rate,
        scheduler_kind: None,
        request: &sched_req,
        affinity_cache: Some(&affinity_cache),
    };
    let (channels2, decision2) = model_router.route_with_scheduler(inputs2).await?;
    assert!(!channels2.is_empty(), "second call should return candidates");
    assert_eq!(
        decision2,
        Some(RoutingDecision::AffinityHit),
        "second call should produce AffinityHit (cache hit hoisted)"
    );

    // Now construct a RouterLog using the production code path's values:
    // layer_decision from RoutingDecision::to_label(), traffic_color from
    // SchedulingRequest.color.as_char().
    let layer_decision = decision2.map(|d| d.to_label());
    let traffic_color = Some(sched_req.color.as_char().to_string());

    assert_eq!(
        layer_decision.as_deref(),
        Some("affinity_hit"),
        "layer_decision from e2e routing must be 'affinity_hit'"
    );
    assert!(
        traffic_color.is_some(),
        "traffic_color from e2e routing must be non-NULL"
    );

    // INSERT the RouterLog with the e2e-derived values and SELECT it back.
    let log = RouterLog {
        id: 0,
        request_id: "req-t5c".to_string(),
        user_id: Some("u-affinity-e2e".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some(channels2[0].id.to_string()),
        status_code: 200,
        latency_ms: 42,
        prompt_tokens: 10,
        completion_tokens: 20,
        cost: 1000,
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
        input_cost: 0,
        output_cost: 0,
        cache_read_cost: 0,
        cache_write_cost: 0,
        audio_cost: 0,
        image_cost: 0,
        video_cost: 0,
        reasoning_cost: 0,
        embedding_cost: 0,
        layer_decision,
        traffic_color,
        created_at: None,
    };
    RouterDatabase::insert_log(&db_arc, &log).await?;

    let rows = RouterDatabase::get_logs(&db_arc, 1, 0).await?;
    assert!(!rows.is_empty(), "should get at least 1 row back");
    let row = &rows[0];
    assert_eq!(
        row.layer_decision.as_deref(),
        Some("affinity_hit"),
        "e2e layer_decision must round-trip as 'affinity_hit'"
    );
    assert!(
        row.traffic_color.is_some(),
        "e2e traffic_color must be present (not NULL) after round-trip"
    );
    assert_eq!(
        row.traffic_color.as_deref(),
        Some("Y"),
        "e2e traffic_color must round-trip as 'Y'"
    );

    Ok(())
}
