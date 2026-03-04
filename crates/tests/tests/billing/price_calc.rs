//! BL-02 & BL-03: Price Calculation Tests (P0/P1)
//!
//! Tests for verifying accurate price calculation with i64 nanodollar precision.
//!
//! Key Requirements (from CLAUDE.md Spec 2.3):
//! - All amounts MUST use i64 nanodollars (9 decimal precision)
//! - NO f32/f64 for financial calculations
//! - NO rust_decimal - use native i64
//! - Database uses BIGINT type
//!
//! Formula: cost = (tokens * price_per_million) / 1_000_000
//! All intermediate calculations use i128 to prevent overflow.

use burncloud_common::{
    calculate_cost_safe, dollars_to_nano, nano_to_dollars, rate_to_scaled, scaled_to_rate,
    NANO_PER_DOLLAR,
};
use burncloud_common::types::TieredPrice;
use burncloud_router::billing::{
    calculate_tiered_cost_nano, AdvancedPricing, BillingError, CostResult, MultiCurrencyPricing,
    TokenUsage,
};
use burncloud_common::Currency;

// ============================================================================
// BL-02: Amount Calculation (i64 Nanodollar Precision) - P0
// ============================================================================

/// Test: $0.002 should be exactly 2,000,000 nanodollars
/// This is the classic GPT-4 pricing scenario
#[test]
fn test_nanodollar_conversion_0_002() {
    let dollars = 0.002;
    let nano = dollars_to_nano(dollars);

    assert_eq!(nano, 2_000_000, "$0.002 should be 2,000,000 nanodollars");

    // Roundtrip verification
    let back = nano_to_dollars(nano);
    assert!(
        (back - dollars).abs() < 1e-9,
        "Roundtrip should preserve value: got {}",
        back
    );
}

/// Test: $1.00 should be exactly 1,000,000,000 nanodollars
#[test]
fn test_nanodollar_conversion_1_dollar() {
    let dollars = 1.0;
    let nano = dollars_to_nano(dollars);

    assert_eq!(nano, 1_000_000_000, "$1.00 should be 1B nanodollars");
}

/// Test: $10.00 signup bonus = 10,000,000,000 nanodollars
#[test]
fn test_nanodollar_conversion_signup_bonus() {
    let signup_bonus_dollars = 10.0;
    let signup_bonus_nano = dollars_to_nano(signup_bonus_dollars);

    assert_eq!(
        signup_bonus_nano, 10_000_000_000,
        "$10 signup bonus should be 10B nanodollars"
    );
}

/// Test: $100.00 demo user balance = 100,000,000,000 nanodollars
#[test]
fn test_nanodollar_conversion_demo_balance() {
    let demo_balance_dollars = 100.0;
    let demo_balance_nano = dollars_to_nano(demo_balance_dollars);

    assert_eq!(
        demo_balance_nano, 100_000_000_000,
        "$100 balance should be 100B nanodollars"
    );
}

/// Test: 9 decimal precision is maintained
#[test]
fn test_nine_decimal_precision() {
    let test_cases = [
        (0.000000001, 1),
        (0.000000009, 9),
        (0.000000010, 10),
        (0.000000100, 100),
        (0.000001000, 1000),
        (0.000010000, 10000),
        (0.000100000, 100000),
        (0.001000000, 1000000),
        (0.010000000, 10000000),
        (0.100000000, 100000000),
    ];

    for (dollars, expected_nano) in test_cases {
        let nano = dollars_to_nano(dollars);
        assert_eq!(
            nano, expected_nano,
            "${:.9} should be {} nanodollars, got {}",
            dollars, expected_nano, nano
        );
    }
}

/// Test: Cost calculation with typical GPT-4 pricing
/// GPT-4 Turbo: $10/1M input, $30/1M output
#[test]
fn test_cost_calculation_gpt4_turbo() {
    let prompt_tokens = 1_000_000u64; // 1M tokens
    let completion_tokens = 500_000u64; // 500K tokens

    let input_price = dollars_to_nano(10.0); // $10/1M = 10B nano
    let output_price = dollars_to_nano(30.0); // $30/1M = 30B nano

    let input_cost = calculate_cost_safe(prompt_tokens, input_price);
    let output_cost = calculate_cost_safe(completion_tokens, output_price);
    let total_cost = input_cost + output_cost;

    assert_eq!(input_cost, 10_000_000_000, "Input cost should be $10");
    assert_eq!(output_cost, 15_000_000_000, "Output cost should be $15");
    assert_eq!(total_cost, 25_000_000_000, "Total cost should be $25");

    // Verify in dollars
    assert!(
        (nano_to_dollars(total_cost) - 25.0).abs() < 1e-6,
        "Total should be $25"
    );
}

/// Test: Cost calculation with GPT-3.5 Turbo pricing
/// GPT-3.5 Turbo: $0.50/1M input, $1.50/1M output
#[test]
fn test_cost_calculation_gpt35_turbo() {
    let prompt_tokens = 500_000u64;
    let completion_tokens = 200_000u64;

    let input_price = dollars_to_nano(0.5); // $0.50/1M
    let output_price = dollars_to_nano(1.5); // $1.50/1M

    let input_cost = calculate_cost_safe(prompt_tokens, input_price);
    let output_cost = calculate_cost_safe(completion_tokens, output_price);
    let total_cost = input_cost + output_cost;

    // 500K * $0.50/1M = $0.25
    // 200K * $1.50/1M = $0.30
    // Total = $0.55
    assert_eq!(input_cost, 250_000_000, "Input cost should be $0.25");
    assert_eq!(output_cost, 300_000_000, "Output cost should be $0.30");
    assert_eq!(total_cost, 550_000_000, "Total cost should be $0.55");
}

/// Test: Cost calculation with very small amounts (sub-cent precision)
#[test]
fn test_cost_calculation_small_amounts() {
    // 1K tokens at $0.002/1M = $0.000002 = 2000 nanodollars
    let tokens = 1_000u64;
    let price = dollars_to_nano(0.002);

    let cost = calculate_cost_safe(tokens, price);

    assert_eq!(cost, 2_000, "1K tokens at $0.002/1M should be 2000 nano");
    assert!(
        (nano_to_dollars(cost) - 0.000002).abs() < 1e-9,
        "Cost should be $0.000002"
    );
}

/// Test: Overflow protection with large values
#[test]
fn test_cost_calculation_large_values() {
    // 10B tokens at $1000/1M = $10,000,000 (10 million dollars)
    let tokens = 10_000_000_000u64;
    let price = dollars_to_nano(1000.0);

    let cost = calculate_cost_safe(tokens, price);

    assert_eq!(
        cost, 10_000_000_000_000_000i64,
        "10B tokens at $1000/1M should cost $10M"
    );

    // Verify conversion back
    let cost_dollars = nano_to_dollars(cost);
    assert!(
        (cost_dollars - 10_000_000.0).abs() < 1e-3,
        "Cost should be $10M, got ${}",
        cost_dollars
    );
}

/// Test: Zero tokens should result in zero cost
#[test]
fn test_cost_calculation_zero_tokens() {
    let price = dollars_to_nano(10.0);
    let cost = calculate_cost_safe(0, price);

    assert_eq!(cost, 0, "Zero tokens should cost $0");
}

/// Test: Constants are correct
#[test]
fn test_constants() {
    assert_eq!(
        NANO_PER_DOLLAR, 1_000_000_000,
        "NANO_PER_DOLLAR should be 10^9"
    );
}

/// Test: Exchange rate scaling for multi-currency
#[test]
fn test_exchange_rate_scaling() {
    // USD to CNY rate: 7.24
    let rate = 7.24;
    let scaled = rate_to_scaled(rate);

    assert_eq!(scaled, 7_240_000_000, "Rate 7.24 should scale to 7.24B");

    let back = scaled_to_rate(scaled);
    assert!(
        (back - rate).abs() < 1e-9,
        "Rate roundtrip should preserve value"
    );
}

/// Test: CostResult with USD only
#[test]
fn test_cost_result_usd_only() {
    let cost_nano = 5_500_000_000i64; // $5.50
    let result = CostResult::from_usd_nano(cost_nano);

    assert_eq!(result.usd_amount_nano, cost_nano);
    assert_eq!(result.local_currency, "USD");
    assert_eq!(result.local_amount_nano, None);

    // Check display format
    assert!(result.display.contains('$'));
}

/// Test: CostResult with local currency
#[test]
fn test_cost_result_with_local() {
    let usd_nano = 1_000_000_000i64; // $1.00
    let cny_nano = 7_240_000_000i64; // ¥7.24

    let result = CostResult::with_local_nano(usd_nano, "CNY", cny_nano);

    assert_eq!(result.usd_amount_nano, usd_nano);
    assert_eq!(result.local_currency, "CNY");
    assert_eq!(result.local_amount_nano, Some(cny_nano));
}

// ============================================================================
// BL-03: Tiered Pricing - P1
// ============================================================================

/// Test: Simple two-tier pricing
/// Tier 1: 0-1M tokens at $1/1M
/// Tier 2: 1M+ tokens at $0.5/1M
#[test]
fn test_tiered_pricing_simple() {
    let tiers = vec![
        TieredPrice {
            id: 1,
            model: "test-model".to_string(),
            region: None,
            tier_start: 0,
            tier_end: Some(1_000_000),
            input_price: dollars_to_nano(1.0),
            output_price: dollars_to_nano(2.0),
        },
        TieredPrice {
            id: 2,
            model: "test-model".to_string(),
            region: None,
            tier_start: 1_000_000,
            tier_end: None,
            input_price: dollars_to_nano(0.5),
            output_price: dollars_to_nano(1.0),
        },
    ];

    // 500K tokens - should be in first tier
    // Formula: 500_000 * 1_000_000_000 / 1_000_000 = 500_000_000 nanodollars
    let cost_500k = calculate_tiered_cost_nano(500_000, &tiers, None).unwrap();
    assert_eq!(cost_500k, 500_000_000, "500K at $1/1M = $0.50 = 500M nano");

    // 1M tokens - exactly at tier boundary
    let cost_1m = calculate_tiered_cost_nano(1_000_000, &tiers, None).unwrap();
    assert_eq!(cost_1m, 1_000_000_000, "1M at $1/1M = $1.00");

    // 2M tokens - crosses tier boundary
    // 1M at $1 + 1M at $0.5 = $1.50
    let cost_2m = calculate_tiered_cost_nano(2_000_000, &tiers, None).unwrap();
    assert_eq!(cost_2m, 1_500_000_000, "2M tokens should cost $1.50");
}

/// Test: Three-tier pricing (Qwen-style)
/// Tier 1: 0-32K at $1.2/1M
/// Tier 2: 32K-128K at $2.4/1M
/// Tier 3: 128K+ at $3.0/1M
#[test]
fn test_tiered_pricing_qwen_style() {
    let tiers = vec![
        TieredPrice {
            id: 1,
            model: "qwen-model".to_string(),
            region: None,
            tier_start: 0,
            tier_end: Some(32_000),
            input_price: dollars_to_nano(1.2),
            output_price: dollars_to_nano(1.2),
        },
        TieredPrice {
            id: 2,
            model: "qwen-model".to_string(),
            region: None,
            tier_start: 32_000,
            tier_end: Some(128_000),
            input_price: dollars_to_nano(2.4),
            output_price: dollars_to_nano(2.4),
        },
        TieredPrice {
            id: 3,
            model: "qwen-model".to_string(),
            region: None,
            tier_start: 128_000,
            tier_end: None,
            input_price: dollars_to_nano(3.0),
            output_price: dollars_to_nano(3.0),
        },
    ];

    // 150K tokens crosses all three tiers
    // 32K * $1.2 + 96K * $2.4 + 22K * $3.0
    // = $0.0384 + $0.2304 + $0.066 = $0.3348
    let cost = calculate_tiered_cost_nano(150_000, &tiers, None).unwrap();

    let expected_nano = 32_000 * 1_200 + 96_000 * 2_400 + 22_000 * 3_000;
    assert_eq!(
        cost, expected_nano as i64,
        "150K tokens should cost correct tiered amount"
    );

    let cost_dollars = nano_to_dollars(cost);
    assert!(
        (cost_dollars - 0.3348).abs() < 1e-4,
        "150K tokens should cost ${:.4}",
        cost_dollars
    );
}

/// Test: Tiered pricing with region filtering
#[test]
fn test_tiered_pricing_with_region() {
    let tiers = vec![
        TieredPrice {
            id: 1,
            model: "regional-model".to_string(),
            region: Some("us".to_string()),
            tier_start: 0,
            tier_end: None,
            input_price: dollars_to_nano(1.0),
            output_price: dollars_to_nano(1.0),
        },
        TieredPrice {
            id: 2,
            model: "regional-model".to_string(),
            region: Some("eu".to_string()),
            tier_start: 0,
            tier_end: None,
            input_price: dollars_to_nano(1.2),
            output_price: dollars_to_nano(1.2),
        },
    ];

    // US region
    let cost_us = calculate_tiered_cost_nano(1_000_000, &tiers, Some("us")).unwrap();
    assert_eq!(cost_us, 1_000_000_000, "US region: 1M tokens at $1/1M = $1");

    // EU region
    let cost_eu = calculate_tiered_cost_nano(1_000_000, &tiers, Some("eu")).unwrap();
    assert_eq!(
        cost_eu, 1_200_000_000,
        "EU region: 1M tokens at $1.2/1M = $1.2"
    );
}

/// Test: Tiered pricing - region mismatch error
#[test]
fn test_tiered_pricing_region_mismatch() {
    let tiers = vec![TieredPrice {
        id: 1,
        model: "us-only-model".to_string(),
        region: Some("us".to_string()),
        tier_start: 0,
        tier_end: None,
        input_price: dollars_to_nano(1.0),
        output_price: dollars_to_nano(1.0),
    }];

    let result = calculate_tiered_cost_nano(1_000_000, &tiers, Some("asia"));

    assert!(
        matches!(result, Err(BillingError::RegionMismatch { .. })),
        "Should return RegionMismatch error for unknown region"
    );
}

/// Test: Tiered pricing - zero tokens
#[test]
fn test_tiered_pricing_zero_tokens() {
    let tiers = vec![TieredPrice {
        id: 1,
        model: "test".to_string(),
        region: None,
        tier_start: 0,
        tier_end: None,
        input_price: dollars_to_nano(1.0),
        output_price: dollars_to_nano(1.0),
    }];

    let cost = calculate_tiered_cost_nano(0, &tiers, None).unwrap();
    assert_eq!(cost, 0, "Zero tokens should cost $0");
}

/// Test: Tiered pricing - empty tiers error
#[test]
fn test_tiered_pricing_empty_tiers() {
    let tiers: Vec<TieredPrice> = vec![];

    let result = calculate_tiered_cost_nano(1000, &tiers, None);

    assert!(
        matches!(result, Err(BillingError::NoTiers)),
        "Should return NoTiers error for empty tier list"
    );
}

/// Test: Advanced pricing structure
#[test]
fn test_advanced_pricing() {
    let pricing = AdvancedPricing {
        input_price: dollars_to_nano(10.0),
        output_price: dollars_to_nano(30.0),
        cache_read_price: Some(dollars_to_nano(1.0)),     // 10% of input
        cache_creation_price: Some(dollars_to_nano(2.5)), // 25% of input
        batch_input_price: Some(dollars_to_nano(5.0)),    // 50% of input
        batch_output_price: Some(dollars_to_nano(15.0)),  // 50% of output
        priority_input_price: Some(dollars_to_nano(17.0)), // 170% of input
        priority_output_price: Some(dollars_to_nano(51.0)), // 170% of output
        audio_input_price: Some(dollars_to_nano(20.0)),   // 200% of input
    };

    assert_eq!(pricing.input_price, 10_000_000_000);
    assert_eq!(pricing.output_price, 30_000_000_000);
    assert_eq!(pricing.cache_read_price, Some(1_000_000_000));
    assert_eq!(pricing.cache_creation_price, Some(2_500_000_000));
}

/// Test: Multi-currency pricing structure
#[test]
fn test_multi_currency_pricing() {
    let pricing = MultiCurrencyPricing {
        usd: AdvancedPricing {
            input_price: dollars_to_nano(1.0),
            output_price: dollars_to_nano(2.0),
            ..Default::default()
        },
        local: Some((
            Currency::CNY,
            AdvancedPricing {
                input_price: dollars_to_nano(7.24),
                output_price: dollars_to_nano(14.48),
                ..Default::default()
            },
        )),
        exchange_rate_nano: Some(rate_to_scaled(7.24)),
    };

    assert!(pricing.local.is_some());
    assert_eq!(pricing.exchange_rate(), Some(7.24));
}

/// Test: Token usage with pricing calculation
#[test]
fn test_token_usage_cost_calculation() {
    let usage = TokenUsage {
        prompt_tokens: 1000,
        completion_tokens: 500,
        cache_read_tokens: 0,
        cache_creation_tokens: 0,
        audio_tokens: 0,
    };

    let pricing = AdvancedPricing {
        input_price: dollars_to_nano(1.0),
        output_price: dollars_to_nano(2.0),
        ..Default::default()
    };

    // Standard calculation
    let input_cost = (usage.prompt_tokens as i128 * pricing.input_price as i128) / 1_000_000;
    let output_cost = (usage.completion_tokens as i128 * pricing.output_price as i128) / 1_000_000;
    let total_cost = (input_cost + output_cost) as i64;

    // 1000 tokens at $1/1M = $0.001 = 1M nano
    // 500 tokens at $2/1M = $0.001 = 1M nano
    assert_eq!(total_cost, 2_000_000, "Total cost should be $0.002");
}
