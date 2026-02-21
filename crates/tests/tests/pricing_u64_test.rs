//! Integration tests for u64 nanodollar pricing precision migration
//!
//! These tests validate the end-to-end behavior of the pricing system
//! after migration from f64 to i64 nanodollar precision.

use burncloud_common::{
    calculate_cost_safe, dollars_to_nano, nano_to_dollars, rate_to_scaled, scaled_to_rate,
    NANO_PER_DOLLAR, RATE_SCALE,
};

/// Test: $2.5 → 2500000000 nanodollars
#[test]
fn test_dollar_to_nano_conversion_2_5() {
    let dollars = 2.5;
    let nano = dollars_to_nano(dollars);
    assert_eq!(nano, 2_500_000_000, "$2.5 should be 2,500,000,000 nanodollars");

    // Roundtrip
    let back = nano_to_dollars(nano);
    assert!((back - dollars).abs() < 1e-9, "Roundtrip should preserve value");
}

/// Test: $0.000000123 → 123 nanodollars
#[test]
fn test_dollar_to_nano_conversion_tiny() {
    let dollars = 0.000000123;
    let nano = dollars_to_nano(dollars);
    assert_eq!(nano, 123, "$0.000000123 should be 123 nanodollars");

    // Roundtrip
    let back = nano_to_dollars(nano);
    assert!((back - dollars).abs() < 1e-9, "Roundtrip should preserve tiny values");
}

/// Test: Exchange rate scaling
#[test]
fn test_exchange_rate_scaling() {
    // Test USD → CNY rate (7.24)
    let rate = 7.24;
    let scaled = rate_to_scaled(rate);
    assert_eq!(scaled, 7_240_000_000, "Rate 7.24 should be 7,240,000,000 scaled");

    let back = scaled_to_rate(scaled);
    assert!((back - rate).abs() < 1e-9, "Rate roundtrip should preserve value");
}

/// Test: Cost calculation with safe overflow protection
#[test]
fn test_safe_cost_calculation() {
    // 1M tokens at $3/1M = $3.00
    let tokens = 1_000_000;
    let price_per_million = dollars_to_nano(3.0);
    let cost = calculate_cost_safe(tokens, price_per_million);
    assert_eq!(cost, 3_000_000_000, "1M tokens at $3/1M should cost $3");

    // 500K tokens at $10/1M = $5.00
    let tokens2 = 500_000;
    let price2 = dollars_to_nano(10.0);
    let cost2 = calculate_cost_safe(tokens2, price2);
    assert_eq!(cost2, 5_000_000_000, "500K tokens at $10/1M should cost $5");
}

/// Test: Large value handling (overflow protection)
#[test]
fn test_large_value_handling() {
    // 10B tokens at $1000/1M = $10,000,000 (10 million)
    // In nanodollars: $10,000,000 * 10^9 = 10,000,000,000,000,000 (10 quadrillion nanodollars)
    // This should not overflow with u128 intermediate
    let tokens = 10_000_000_000u64;
    let price_per_million = dollars_to_nano(1000.0);
    let cost = calculate_cost_safe(tokens, price_per_million);
    assert_eq!(cost, 10_000_000_000_000_000u64, "10B tokens at $1000/1M should cost $10M");

    // Verify conversion back to dollars
    let cost_dollars = nano_to_dollars(cost);
    assert!((cost_dollars - 10_000_000.0).abs() < 1e-3, "Cost should be $10M");
}

/// Test: 9 decimal precision
#[test]
fn test_nine_decimal_precision() {
    // Test that we can represent amounts with 9 decimal precision
    let amounts = [
        (0.000000001, 1),           // 1 nanodollar
        (0.000000009, 9),           // 9 nanodollars
        (0.000000010, 10),          // 10 nanodollars
        (0.000000100, 100),         // 100 nanodollars
        (0.000001000, 1000),        // 1000 nanodollars
        (0.000010000, 10000),       // 10000 nanodollars
        (0.000100000, 100000),      // 100000 nanodollars
        (0.001000000, 1000000),     // 1000000 nanodollars
        (0.010000000, 10000000),    // 10000000 nanodollars
        (0.100000000, 100000000),   // 100000000 nanodollars
        (1.000000000, 1000000000),  // 1 dollar = 1B nanodollars
    ];

    for (dollars, expected_nano) in amounts {
        let nano = dollars_to_nano(dollars);
        assert_eq!(
            nano, expected_nano,
            "${:.9} should be {} nanodollars, got {}",
            dollars, expected_nano, nano
        );
    }
}

/// Test: Constants are correct
#[test]
fn test_constants() {
    assert_eq!(NANO_PER_DOLLAR, 1_000_000_000, "NANO_PER_DOLLAR should be 10^9");
    assert_eq!(RATE_SCALE, 1_000_000_000, "RATE_SCALE should be 10^9");
}

/// Test: Zero and edge cases
#[test]
fn test_edge_cases() {
    // Zero
    assert_eq!(dollars_to_nano(0.0), 0);

    // Very small non-zero
    assert_eq!(dollars_to_nano(0.000000001), 1);

    // Roundtrip for various values
    for dollars in [0.0, 0.01, 0.1, 1.0, 10.0, 100.0, 1000.0] {
        let nano = dollars_to_nano(dollars);
        let back = nano_to_dollars(nano);
        assert!(
            (back - dollars).abs() < 1e-9,
            "Roundtrip failed for {}: got {}",
            dollars, back
        );
    }
}
