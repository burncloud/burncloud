#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Boundary behaviour of the nanodollar money contract (#633).
//!
//! `src/price_u64.rs` already tests the ordinary range. What was untested is what happens *outside*
//! it: at the i64 limits, at the rounding boundary between two nanodollars, and where an
//! intermediate result stops fitting in the return type. Those are the cases a billing system meets
//! through a wrong unit or a hostile input.
//!
//! Three rules govern this file, each one a correction of an earlier draft:
//!
//! 1. **Expected values are derived, not guessed.** Where the arithmetic is not obvious the
//!    derivation is written out, so a reader can check it and a future change has to argue with the
//!    arithmetic rather than with a magic number. Two expectations in the first draft were simply
//!    miscalculated and the failures caught them.
//! 2. **A defect is not "real behaviour".** The first draft asserted that the narrowing cast
//!    saturates to `i64::MAX`. It does not, and that assertion failed. The wrong correction -- and
//!    the one this file originally shipped -- was to write the measured wrapped value as the
//!    expectation. That declares a wrong charge to be the contract and makes the eventual fix fail.
//!    The out-of-range case and the `NaN` case are therefore *not* asserted here: they are recorded
//!    as an ignored failing test plus #640, where the correct behaviour gets decided.
//! 3. **What remains here is only what is true regardless of that decision**: the in-range results,
//!    the rounding direction, the saturation at the float-cast limits, and the fact that the
//!    out-of-range result is *not* a clamp.
//!
//! The distinction in rule 2 is the whole point: clamping to `i64::MAX` at least errs in a
//! predictable direction and is a defensible contract; wrapping to `3875820019684212736` is a wrong
//! amount that happens to stay positive, which no sanity check would flag.

use burncloud_commerce_contracts::price_u64::{
    calculate_cost_safe, dollars_to_nano, nano_to_dollars, rate_to_scaled, scaled_to_rate,
    NANO_PER_DOLLAR, RATE_SCALE,
};

#[test]
fn nanodollar_conversion_rounds_to_nearest_not_toward_zero() {
    // `.round()` is not truncation: this is the difference between charging 0 and charging 1
    // nanodollar for a fraction of a cent.
    assert_eq!(dollars_to_nano(0.000_000_000_4), 0, "0.4 nano rounds down");
    assert_eq!(
        dollars_to_nano(0.000_000_000_5),
        1,
        "0.5 nano rounds up (half away from zero)"
    );
    assert_eq!(dollars_to_nano(0.000_000_000_6), 1, "0.6 nano rounds up");
    assert_eq!(dollars_to_nano(0.000_000_001_4), 1);
    assert_eq!(dollars_to_nano(0.000_000_001_5), 2);

    // The same rounding applies to the exchange-rate scale.
    assert_eq!(rate_to_scaled(0.000_000_000_4), 0);
    assert_eq!(rate_to_scaled(0.000_000_000_5), 1);
}

#[test]
fn negative_amounts_round_away_from_zero_too() {
    // A refund or a negative adjustment goes through the same function, so its rounding direction
    // matters as much as the positive case.
    assert_eq!(dollars_to_nano(-0.000_000_000_4), 0);
    assert_eq!(dollars_to_nano(-0.000_000_000_5), -1);
    assert_eq!(dollars_to_nano(-1.0), -NANO_PER_DOLLAR);
    assert_eq!(rate_to_scaled(-0.000_000_000_5), -1);
}

#[test]
fn non_finite_and_absurd_inputs_saturate_rather_than_panic() {
    // `as i64` on a float saturates. Recording it makes the failure mode explicit: an unvalidated
    // price becomes a huge charge instead of a panic or a wrapped negative amount.
    assert_eq!(dollars_to_nano(f64::MAX), i64::MAX, "saturates at the top");
    assert_eq!(dollars_to_nano(f64::INFINITY), i64::MAX);
    assert_eq!(
        dollars_to_nano(f64::MIN),
        i64::MIN,
        "saturates at the bottom"
    );
    assert_eq!(dollars_to_nano(f64::NEG_INFINITY), i64::MIN);
    assert_eq!(rate_to_scaled(f64::MAX), i64::MAX);

    // NaN is deliberately NOT asserted here. `f64::NAN as i64` is 0, which would mean an unvalidated
    // price bills nothing -- a wrong amount rather than a contract, and asserting the 0 would make it
    // permanent. It is recorded as a defect in #640 alongside the overflow below. The saturation
    // cases above are different: they are the documented float-cast behaviour at the representable
    // limits, and a clamped amount is at least in the right direction.

    // A value whose product is exactly representable still converts exactly, so the saturation above
    // is the boundary rather than an off-by-one.
    assert_eq!(
        dollars_to_nano(i64::MAX as f64 / NANO_PER_DOLLAR as f64),
        i64::MAX
    );
}

#[test]
fn nan_to_dollar_conversion_is_exact_at_both_limits() {
    // The reverse direction is a plain division, so it cannot saturate; what it must not do is lose
    // more precision than f64 allows at the extremes.
    let max = nano_to_dollars(i64::MAX);
    assert!(max.is_finite() && max > 0.0);
    assert_eq!(nano_to_dollars(0), 0.0);
    assert_eq!(nano_to_dollars(-NANO_PER_DOLLAR), -1.0);
    assert_eq!(
        nano_to_dollars(i64::MIN),
        i64::MIN as f64 / NANO_PER_DOLLAR as f64
    );

    // Round-tripping a representable amount must not drift.
    for nano in [0_i64, 1, 999, 1_000_000_000, 123_456_789_012] {
        let back = dollars_to_nano(nano_to_dollars(nano));
        assert_eq!(back, nano, "round trip drifted for {nano}");
    }
}

#[test]
fn scaled_rate_conversion_matches_the_dollar_scale() {
    // Both scales are 1e9 by contract; if one ever changed, the two directions would disagree and a
    // currency conversion would shift by orders of magnitude.
    assert_eq!(NANO_PER_DOLLAR, RATE_SCALE);
    for rate in [0.0_f64, 1.0, 7.24, 0.000_001, 1234.567_891] {
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
    // The implementation multiplies in i128 and divides by 1e6, so the intermediate cannot overflow
    // at these magnitudes. The expected value is written as the same arithmetic, spelled out, so a
    // reader can verify it without running the code.
    let cases: [(u64, i64, i128); 3] = [
        (1_000_000, 3_000_000_000, 3_000_000_000), // 1e6 tokens x 3e9 / 1e6
        (500_000, 2_000_000_000, 1_000_000_000),
        (1_000, 1_000_000, 1_000), // 1e3 x 1e6 / 1e6
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

    // The largest exact result this shape can produce: u64::MAX tokens at one nanodollar per million.
    // The i128 product is ~1.8e25, and the quotient ~1.8e19 fits, so the i128 intermediate is what
    // makes this representable at all.
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
fn cost_computation_does_not_saturate_at_the_i64_ceiling() {
    // A result above i64 does NOT clamp to i64::MAX. What it actually does is recorded in the
    // failing regression test below rather than asserted as correct here: writing
    // `assert_eq!(observed, wrapped_value)` would declare a wrong charge to be the contract, and the
    // eventual fix would then turn this test red. See #640.
    let tokens: u64 = 1_000_000_000_000;
    let price: i64 = 1_000_000_000_000_000;
    let true_value = (tokens as i128 * price as i128) / 1_000_000;
    assert_eq!(true_value, 1_000_000_000_000_000_000_000_i128, "1e21");
    assert!(
        true_value > i64::MAX as i128,
        "the case must exceed i64 for this test to mean anything"
    );

    // The one thing assertable without blessing a wrong amount: it does not saturate.
    assert_ne!(
        calculate_cost_safe(tokens, price),
        i64::MAX,
        "if this ever becomes a clamp the behaviour changed, so #640 must be updated"
    );

    // A price at i64::MAX for one million tokens is roughly 9.2e18 and therefore still fits exactly.
    assert_eq!(calculate_cost_safe(1_000_000, i64::MAX), i64::MAX);
}

/// The behaviour #640 must produce, stated as a test so the fix has a target.
///
/// Ignored rather than deleted, and deliberately not written as an assertion on the current output:
/// `assert_eq!(observed, wrapped)` would make a wrong charge permanent and would fail once the defect
/// is fixed. An ignored failing test is an honest record; remove the `ignore` when #640 lands.
#[test]
#[ignore = "known defect, see #640: out-of-range cost wraps instead of being rejected"]
fn out_of_range_cost_is_rejected_rather_than_wrapped() {
    let tokens: u64 = 1_000_000_000_000;
    let price: i64 = 1_000_000_000_000_000;
    let true_value = (tokens as i128 * price as i128) / 1_000_000;
    assert!(true_value > i64::MAX as i128, "the case must exceed i64");

    // #640 chooses between an error, saturation and checked arithmetic. Until that decision this
    // test only states that the current answer is wrong, not what the right answer is.
    let observed = calculate_cost_safe(tokens, price);
    panic!(
        "known defect (#640): an out-of-range cost returned {observed} where the true cost is \
         {true_value}, with no error and no way for the caller to tell"
    );
}

#[test]
fn cost_computation_truncates_the_remainder() {
    // Integer division truncates. With a price of 1e9 nano per million tokens, one token costs
    // 1e9 / 1e6 = 1000 nano; the truncation only shows below one token, so the smallest billable
    // step is expressed here as the point where the quotient changes.
    let price = NANO_PER_DOLLAR; // 1e9 nano per million tokens => 1000 nano per token
    assert_eq!(calculate_cost_safe(1, price), 1_000);
    assert_eq!(calculate_cost_safe(999, price), 999_000);
    assert_eq!(calculate_cost_safe(1_000, price), 1_000_000);

    // A price small enough that the quotient is zero for the token count given: 1e6 nano per million
    // tokens means one nano per token, and 999_999 tokens is 999_999 nano -- not zero. Truncation
    // only discards a fraction of a nanodollar, which needs a price below 1e6.
    assert_eq!(calculate_cost_safe(999_999, 1_000_000), 999_999);
    assert_eq!(
        calculate_cost_safe(1, 1),
        0,
        "1 token at 1 nano/M truncates to 0"
    );
    assert_eq!(calculate_cost_safe(999_999, 1), 0);

    // Zero on either side yields zero, and a negative price is not silently made positive.
    assert_eq!(calculate_cost_safe(0, i64::MAX), 0);
    assert_eq!(calculate_cost_safe(u64::MAX, 0), 0);
    // 1e6 tokens x -1e6 per million = -1e6 nano.
    assert_eq!(calculate_cost_safe(1_000_000, -1_000_000), -1_000_000);
}
