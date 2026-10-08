#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions on the compatibility fixtures are the test, and fail fast."
)]
//! S1-A compatibility evidence: the legacy `burncloud_common` import paths and the
//! Commerce-owned contract path must resolve to the same implementation and return
//! identical results for zero, positive/negative, rounding-boundary and large values.

use burncloud_commerce_contracts::price_u64 as commerce;
use burncloud_common as legacy;

// The legacy module path and the legacy root re-export must both stay usable.
use burncloud_common::price_u64 as legacy_module;

/// Cases kept together with the rounding boundary and the large-value case from the
/// migration baseline, so the compatibility check cannot silently narrow the range.
const DOLLAR_CASES: [f64; 10] = [
    0.0,
    0.000_000_001,
    0.000_000_001_4,
    0.000_000_001_5,
    0.000_15,
    0.15,
    1.0,
    3.0,
    150.5,
    1_000_000.0,
];

const RATE_CASES: [f64; 7] = [0.0, 0.138, 1.0, 1.08, 7.24, 150.5, 1_000.0];

const NANO_CASES: [i64; 9] = [
    -1_000_000_000,
    -1,
    0,
    1,
    123_456_789,
    150_000_000,
    3_000_000_000,
    10_000_000_000_000_000,
    i64::MAX,
];

const COST_CASES: [(u64, i64); 7] = [
    (0, 0),
    (0, 3_000_000_000),
    (1_000_000, 0),
    (150_000, 1_200_000_000),
    (1_000_000, 3_000_000_000),
    (10_000_000_000, 1_000_000_000_000),
    (u64::MAX, 1),
];

#[test]
fn constants_keep_their_values_and_paths() {
    assert_eq!(legacy::NANO_PER_DOLLAR, 1_000_000_000);
    assert_eq!(legacy::RATE_SCALE, 1_000_000_000);
    assert_eq!(legacy::NANO_PER_DOLLAR, commerce::NANO_PER_DOLLAR);
    assert_eq!(legacy::RATE_SCALE, commerce::RATE_SCALE);
    assert_eq!(legacy_module::NANO_PER_DOLLAR, commerce::NANO_PER_DOLLAR);
    assert_eq!(legacy_module::RATE_SCALE, commerce::RATE_SCALE);
}

#[test]
fn legacy_types_module_aliases_stay_available() {
    use burncloud_common::types::{
        dollars_to_nanodollars, nanodollars_to_dollars, NANODOLLAR_SCALE,
    };

    assert_eq!(NANODOLLAR_SCALE, commerce::NANO_PER_DOLLAR);
    assert_eq!(dollars_to_nanodollars(3.0), commerce::dollars_to_nano(3.0));
    assert_eq!(
        nanodollars_to_dollars(3_000_000_000).to_bits(),
        3.0f64.to_bits()
    );
}

#[test]
fn dollars_to_nano_matches_across_paths() {
    for dollars in DOLLAR_CASES {
        let expected = commerce::dollars_to_nano(dollars);
        assert_eq!(
            legacy::dollars_to_nano(dollars),
            expected,
            "root: {}",
            dollars
        );
        assert_eq!(
            legacy_module::dollars_to_nano(dollars),
            expected,
            "module: {}",
            dollars
        );
    }
}

#[test]
fn nano_to_dollars_matches_across_paths() {
    for nano in NANO_CASES {
        let expected = commerce::nano_to_dollars(nano);
        assert_eq!(
            legacy::nano_to_dollars(nano).to_bits(),
            expected.to_bits(),
            "root: {}",
            nano
        );
        assert_eq!(
            legacy_module::nano_to_dollars(nano).to_bits(),
            expected.to_bits(),
            "module: {}",
            nano
        );
    }
}

#[test]
fn rate_conversions_match_across_paths() {
    for rate in RATE_CASES {
        let expected = commerce::rate_to_scaled(rate);
        assert_eq!(legacy::rate_to_scaled(rate), expected, "root: {}", rate);
        assert_eq!(
            legacy_module::rate_to_scaled(rate),
            expected,
            "module: {}",
            rate
        );
    }

    for scaled in [
        i64::MIN,
        -7_240_000_000,
        0,
        138_000_000,
        7_240_000_000,
        i64::MAX,
    ] {
        let expected = commerce::scaled_to_rate(scaled);
        assert_eq!(
            legacy::scaled_to_rate(scaled).to_bits(),
            expected.to_bits(),
            "root: {}",
            scaled
        );
        assert_eq!(
            legacy_module::scaled_to_rate(scaled).to_bits(),
            expected.to_bits(),
            "module: {}",
            scaled
        );
    }
}

#[test]
fn calculate_cost_safe_matches_across_paths() {
    for (tokens, price) in COST_CASES {
        let expected = commerce::calculate_cost_safe(tokens, price);
        assert_eq!(
            legacy::calculate_cost_safe(tokens, price),
            expected,
            "root: {} @ {}",
            tokens,
            price
        );
        assert_eq!(
            legacy_module::calculate_cost_safe(tokens, price),
            expected,
            "module: {} @ {}",
            tokens,
            price
        );
    }
}

/// Documents the preserved baseline behavior rather than asserting new behavior.
#[test]
fn preserved_rounding_and_truncation_behavior() {
    assert_eq!(commerce::dollars_to_nano(0.000_000_001_5), 2);
    assert_eq!(commerce::dollars_to_nano(0.000_000_001_4), 1);
    assert_eq!(legacy::dollars_to_nano(0.000_000_001_5), 2);
    assert_eq!(legacy::dollars_to_nano(0.000_000_001_4), 1);

    // Tail division truncates toward zero, as in the pre-migration implementation.
    assert_eq!(commerce::calculate_cost_safe(1, 1_999_999), 1);
    assert_eq!(legacy::calculate_cost_safe(1, 1_999_999), 1);
    assert_eq!(commerce::calculate_cost_safe(1_999_999, 1), 1);
}
