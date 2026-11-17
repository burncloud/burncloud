#![allow(clippy::unwrap_used, reason = "测试固定输入与断言失败应中止")]
use super::*;
use rand::{rngs::StdRng, SeedableRng};

#[test]
fn weights_use_one_positive_wide_integer_draw() {
    let mut rng = StdRng::seed_from_u64(73);
    let mut counts = [0_usize; 2];
    for _ in 0..20000 {
        counts[weighted_pick(&[(0, 1), (1, 3)], &mut rng) as usize] += 1;
    }
    assert!(
        (0.23..0.27).contains(&(counts[0] as f64 / 20000.0)),
        "{counts:?}"
    );
    assert_eq!(weighted_pick(&[(7, u32::MAX)], &mut rng), 7);
    assert!([1, 2].contains(&weighted_pick(&[(1, u32::MAX), (2, u32::MAX)], &mut rng)));
}

#[test]
fn prices_are_integer_strict_zero_aware_and_direct_rate_has_precedence() {
    let mut rates = HashMap::new();
    assert_eq!(reference_price(0, 0, "USD", &rates), Some(0));
    assert_eq!(reference_price(-1, 0, "USD", &rates), None);
    assert_eq!(reference_price(0, 0, "XXX", &rates), None);
    assert_eq!(reference_price(0, 0, "CNY", &rates), None);
    assert_eq!(
        reference_price(i64::MAX, i64::MAX, "USD", &rates),
        Some(i128::from(i64::MAX) * 2)
    );
    rates.insert((Currency::USD, Currency::CNY), 2_000_000_000);
    assert_eq!(reference_price(1, 0, "CNY", &rates), Some(1));
    rates.insert((Currency::CNY, Currency::USD), 0);
    assert_eq!(reference_price(0, 0, "CNY", &rates), None);
    rates.insert((Currency::CNY, Currency::USD), 1_500_000_000);
    assert_eq!(reference_price(1, 0, "CNY", &rates), Some(2));
    assert_eq!(reference_price(0, 0, "CNY", &rates), Some(0));
}

#[test]
fn policies_reject_unknown_mismatched_and_duplicate_groups() {
    for raw in [
        "",
        "null",
        "[]",
        r#"{"a":{}}"#,
        r#"{"a":{"type":"unknown"}}"#,
        r#"{"a":{"type":"balanced","cost_weight":1}}"#,
        r#"{"a":{"type":"combined","cost_weight":-1}}"#,
        r#"{"A":{"type":"balanced"},"a":{"type":"price_first"}}"#,
        r#"{"a":{"type":"balanced"},"a":{"type":"balanced"}}"#,
        r#"{"a":{"type":"balanced","extra":1}}"#,
    ] {
        assert!(super::super::parse_scheduler_config(raw).is_err(), "{raw}");
    }
    for kind in [
        "balanced",
        "stability_first",
        "price_first",
        "passthrough",
        "combined",
    ] {
        assert!(
            super::super::parse_scheduler_config(&format!(r#"{{"a":{{"type":"{kind}"}}}}"#))
                .is_ok()
        );
    }
}

#[tokio::test]
async fn priority_health_price_and_missing_quotes_obey_strict_tiers() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = Arc::new(
        burncloud_database::create_database_with_url(&burncloud_database::sqlite_url(
            &file.path().to_string_lossy(),
            cfg!(windows),
            true,
        ))
        .await
        .unwrap(),
    );
    let local = LocalAttachmentState::for_database(&db);
    let tracker = ChannelStateTracker::new();
    let mut plan = StrictPlan {
        kind: SchedulerKind::Balanced,
        model: "test".into(),
        local,
        candidates: [
            (
                1,
                Candidate {
                    priority: 20,
                    weight: 1,
                    price: None,
                    local_url: None,
                    order_allowed: true,
                    enabled: true,
                },
            ),
            (
                2,
                Candidate {
                    priority: 10,
                    weight: 100,
                    price: Some(0),
                    local_url: None,
                    order_allowed: true,
                    enabled: true,
                },
            ),
            (
                3,
                Candidate {
                    priority: 20,
                    weight: 1,
                    price: Some(5),
                    local_url: None,
                    order_allowed: true,
                    enabled: true,
                },
            ),
        ]
        .into_iter()
        .collect(),
    };
    assert_eq!(
        plan.pick([1, 2].into_iter(), &tracker, |_| true).unwrap(),
        1
    );
    assert_eq!(
        plan.pick([1, 2].into_iter(), &tracker, |id| id != 1)
            .unwrap(),
        2
    );
    plan.kind = SchedulerKind::PriceFirst;
    assert_eq!(
        plan.pick([1, 2].into_iter(), &tracker, |_| true),
        Err(SelectionError::NoComparablePrice)
    );
    assert_eq!(
        plan.pick([1, 2, 3].into_iter(), &tracker, |_| true)
            .unwrap(),
        3
    );
    plan.kind = SchedulerKind::StabilityFirst;
    tracker.record_success(1, Some("test"), 10000, None);
    tracker.record_success(3, Some("test"), 1, None);
    assert_eq!(
        plan.pick([1, 2, 3].into_iter(), &tracker, |_| true)
            .unwrap(),
        3
    );
    db.get_connection().unwrap().pool().close().await;
}
