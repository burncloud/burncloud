#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "test-only file: unwrap/expect are how these boundary assertions report a failure"
)]
//! Boundary behaviour of the nanodollar money contract (#633 / #640).
//!
//! `src/price_u64.rs` covers the ordinary range. This file pins the boundary rules:
//! rounding remains half-away-from-zero, finite float casts saturate at i64 bounds,
//! and cost arithmetic saturates rather than wrapping when its true i128 result is
//! outside the i64 return type. #640 also makes NaN fail closed instead of becoming
//! an accidental zero/free price.

use burncloud_commerce_contracts::price_u64::{
    calculate_cost_safe, dollars_to_nano, nano_to_dollars, rate_to_scaled, scaled_to_rate,
    NANO_PER_DOLLAR, RATE_SCALE,
};

#[test]
fn nanodollar_conversion_rounds_to_nearest_not_toward_zero() {
    assert_eq!(dollars_to_nano(0.000_000_000_4), 0, "0.4 nano rounds down");
    assert_eq!(
        dollars_to_nano(0.000_000_000_5),
        1,
        "0.5 nano rounds up (half away from zero)"
    );
    assert_eq!(dollars_to_nano(0.000_000_000_6), 1, "0.6 nano rounds up");
    assert_eq!(dollars_to_nano(0.000_000_001_4), 1);
    assert_eq!(dollars_to_nano(0.000_000_001_5), 2);

    assert_eq!(rate_to_scaled(0.000_000_000_4), 0);
    assert_eq!(rate_to_scaled(0.000_000_000_5), 1);
}

#[test]
fn negative_amounts_round_away_from_zero_too() {
    assert_eq!(dollars_to_nano(-0.000_000_000_4), 0);
    assert_eq!(dollars_to_nano(-0.000_000_000_5), -1);
    assert_eq!(dollars_to_nano(-1.0), -NANO_PER_DOLLAR);
    assert_eq!(rate_to_scaled(-0.000_000_000_5), -1);
}

#[test]
fn non_finite_and_absurd_inputs_fail_closed_or_saturate() {
    assert_eq!(dollars_to_nano(f64::MAX), i64::MAX, "saturates at the top");
    assert_eq!(dollars_to_nano(f64::INFINITY), i64::MAX);
    assert_eq!(
        dollars_to_nano(f64::MIN),
        i64::MIN,
        "saturates at the bottom"
    );
    assert_eq!(dollars_to_nano(f64::NEG_INFINITY), i64::MIN);
    assert_eq!(rate_to_scaled(f64::MAX), i64::MAX);

    // #640: Rust's raw `NaN as i64` would produce zero. In a price path zero is
    // a fail-open/free result, so NaN now fails closed to the upper bound.
    assert_eq!(dollars_to_nano(f64::NAN), i64::MAX);
    assert_eq!(rate_to_scaled(f64::NAN), i64::MAX);

    assert_eq!(
        dollars_to_nano(i64::MAX as f64 / NANO_PER_DOLLAR as f64),
        i64::MAX
    );
}

#[test]
fn nan_to_dollar_conversion_is_exact_at_both_limits() {
    let max = nano_to_dollars(i64::MAX);
    assert!(max.is_finite() && max > 0.0);
    assert_eq!(nano_to_dollars(0), 0.0);
    assert_eq!(nano_to_dollars(-NANO_PER_DOLLAR), -1.0);
    assert_eq!(
        nano_to_dollars(i64::MIN),
        i64::MIN as f64 / NANO_PER_DOLLAR as f64
    );

    for nano in [0_i64, 1, 999, 1_000_000_000, 123_456_789_012] {
        let back = dollars_to_nano(nano_to_dollars(nano));
        assert_eq!(back, nano, "round trip drifted for {nano}");
    }
}

#[test]
fn scaled_rate_conversion_matches_the_dollar_scale() {
    assert_eq!(NANO_PER_DOLLAR, RATE_SCALE);
    for rate in [0.0_f64, 1.0, 7.24, 0.000_001, 1_234.567_891] {
        let scaled = rate_to_scaled(rate);
        let back = scaled_to_rate(scaled);
        assert!(
            (back - rate).abs() < 1e-9,
            "rate {rate} round-tripped to {back} through {scaled}"
        );
    }
}

#[test]
fn cost_computation_keeps_the_exact_result_while_it_fits_in_i64() {
    let cases: [(u64, i64, i128); 3] = [
        (1_000_000, 3_000_000_000, 3_000_000_000),
        (500_000, 2_000_000_000, 1_000_000_000),
        (1_000, 1_000_000, 1_000),
    ];
    for (tokens, price, expected) in cases {
        let derived = (tokens as i128 * price as i128) / 1_000_000;
        assert_eq!(
            derived, expected,
            "the derivation itself is wrong for {tokens}/{price}"
        );
        assert!(
            derived <= i64::MAX as i128,
            "this case is meant to fit in i64; move it to the saturation test"
        );
        assert_eq!(calculate_cost_safe(tokens, price), expected as i64);
    }

    let tokens = u64::MAX;
    let price = 1_i64;
    let derived = (tokens as i128 * price as i128) / 1_000_000;
    assert!(
        derived <= i64::MAX as i128,
        "this case should fit: {derived}"
    );
    assert_eq!(calculate_cost_safe(tokens, price), derived as i64);
}

#[test]
fn out_of_range_cost_saturates_instead_of_wrapping() {
    let tokens: u64 = 1_000_000_000_000;
    let price: i64 = 1_000_000_000_000_000;
    let true_value = (tokens as i128 * price as i128) / 1_000_000;
    assert_eq!(true_value, 1_000_000_000_000_000_000_000_i128, "1e21");
    assert!(true_value > i64::MAX as i128, "the case must exceed i64");

    assert_eq!(calculate_cost_safe(tokens, price), i64::MAX);
    assert_eq!(calculate_cost_safe(tokens, -price), i64::MIN);

    // The exact bound still remains exact rather than being changed by the clamp.
    assert_eq!(calculate_cost_safe(1_000_000, i64::MAX), i64::MAX);
}

#[test]
fn cost_computation_truncates_the_remainder() {
    let price = NANO_PER_DOLLAR;
    assert_eq!(calculate_cost_safe(1, price), 1_000);
    assert_eq!(calculate_cost_safe(999, price), 999_000);
    assert_eq!(calculate_cost_safe(1_000, price), 1_000_000);

    assert_eq!(calculate_cost_safe(999_999, 1_000_000), 999_999);
    assert_eq!(
        calculate_cost_safe(1, 1),
        0,
        "1 token at 1 nano/M truncates to 0"
    );
    assert_eq!(calculate_cost_safe(999_999, 1), 0);

    assert_eq!(calculate_cost_safe(0, i64::MAX), 0);
    assert_eq!(calculate_cost_safe(u64::MAX, 0), 0);
    assert_eq!(calculate_cost_safe(1_000_000, -1_000_000), -1_000_000);
}
