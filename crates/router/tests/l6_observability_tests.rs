//! End-to-end coverage for the L6 Observability integration (issue #152 / FU-3).
//!
//! Exercises the full L6 data flow: `layer_decision` / `traffic_color`
//! columns in `router_logs` INSERT → SELECT round-trip, plus label
//! enumeration coverage for all valid layer_decision values and
//! traffic_color chars.
//!
//! - T1: INSERT round-trip with layer_decision='affinity_hit' + traffic_color='Y'
//! - T2: All 7 layer_decision labels round-trip through INSERT → SELECT
//! - T3: All traffic_color chars (G, Y, R) round-trip through INSERT → SELECT

#![allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types)]

mod common;

use burncloud_database_router::RouterDatabase;
use burncloud_database_router::RouterLog;

use common::{ensure_l6_observability_columns, setup_db};

/// Build a minimal `RouterLog` for testing. Only `layer_decision` and
/// `traffic_color` vary between tests; everything else uses defaults.
fn make_log(layer_decision: Option<&str>, traffic_color: Option<&str>) -> RouterLog {
    RouterLog {
        id: 0,
        request_id: "req-l6-test".to_string(),
        user_id: Some("u-l6-test".to_string()),
        path: "/v1/chat/completions".to_string(),
        upstream_id: Some("ch-1".to_string()),
        status_code: 200,
        latency_ms: 42,
        prompt_tokens: 10,
        completion_tokens: 20,
        cost: 1000, // 1000 nanodollars
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
        layer_decision: layer_decision.map(|s| s.to_string()),
        traffic_color: traffic_color.map(|s| s.to_string()),
        created_at: None,
    }
}

/// T1 — INSERT with `layer_decision='affinity_hit'` and `traffic_color='Y'`,
/// then SELECT back and verify both columns survive the round-trip.
#[tokio::test]
async fn t1_insert_roundtrip_affinity_hit_yellow() -> anyhow::Result<()> {
    let (db, pool, _url) = setup_db().await?;
    ensure_l6_observability_columns(&pool).await?;

    let log = make_log(Some("affinity_hit"), Some("Y"));
    RouterDatabase::insert_log(&db, &log).await?;

    let rows = RouterDatabase::get_logs(&db, 1, 0).await?;
    assert!(!rows.is_empty(), "should get at least 1 row back");
    let row = &rows[0];
    assert_eq!(row.layer_decision.as_deref(), Some("affinity_hit"));
    assert_eq!(row.traffic_color.as_deref(), Some("Y"));

    Ok(())
}

/// T2 — All 7 valid `layer_decision` labels round-trip through INSERT → SELECT.
/// Labels: affinity_hit, scorer_picked, failover_1, shaper_own, shaper_borrow,
/// shaper_reject, shaper_unconfigured.
#[tokio::test]
async fn t2_all_layer_decision_labels_roundtrip() -> anyhow::Result<()> {
    let (db, pool, _url) = setup_db().await?;
    ensure_l6_observability_columns(&pool).await?;

    let labels = [
        "affinity_hit",
        "scorer_picked",
        "failover_1",
        "shaper_own",
        "shaper_borrow",
        "shaper_reject",
        "shaper_unconfigured",
    ];

    for (i, label) in labels.iter().enumerate() {
        let mut log = make_log(Some(label), Some("Y"));
        log.request_id = format!("req-t2-{i}");
        RouterDatabase::insert_log(&db, &log).await?;
    }

    let rows = RouterDatabase::get_logs(&db, labels.len() as i32, 0).await?;
    assert_eq!(rows.len(), labels.len(), "should get all 7 rows back");

    let mut found: Vec<String> = rows
        .iter()
        .filter_map(|r| r.layer_decision.clone())
        .collect();
    found.sort();
    let mut expected: Vec<&str> = labels.to_vec();
    expected.sort();
    assert_eq!(
        found,
        expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "all 7 layer_decision labels must survive INSERT → SELECT"
    );

    Ok(())
}

/// T3 — All traffic_color chars (G, Y, R) round-trip through INSERT → SELECT.
#[tokio::test]
async fn t3_traffic_color_gyr_roundtrip() -> anyhow::Result<()> {
    let (db, pool, _url) = setup_db().await?;
    ensure_l6_observability_columns(&pool).await?;

    let colors = ["G", "Y", "R"];

    for (i, color) in colors.iter().enumerate() {
        let mut log = make_log(Some("scorer_picked"), Some(color));
        log.request_id = format!("req-t3-{i}");
        RouterDatabase::insert_log(&db, &log).await?;
    }

    let rows = RouterDatabase::get_logs(&db, colors.len() as i32, 0).await?;
    assert_eq!(rows.len(), colors.len(), "should get all 3 rows back");

    let mut found: Vec<String> = rows
        .iter()
        .filter_map(|r| r.traffic_color.clone())
        .collect();
    found.sort();
    let mut expected: Vec<&str> = colors.to_vec();
    expected.sort();
    assert_eq!(
        found,
        expected.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
        "all 3 traffic_color chars must survive INSERT → SELECT"
    );

    Ok(())
}
