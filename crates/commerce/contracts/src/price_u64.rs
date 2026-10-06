//! Price conversion utilities for i64 nanodollar precision.
//!
//! This module provides conversion functions between floating-point dollar values
//! and i64 nanodollar (10^-9 dollars) representation for precise financial calculations.
//! Using i64 for PostgreSQL BIGINT compatibility.
//!
//! # Precision
//! - 1 nanodollar = 10^-9 dollars = $0.000000001
//! - 1 dollar = 1,000,000,000 nanodollars
//!
//! # Example
//! ```
//! use burncloud_commerce_contracts::price_u64::{dollars_to_nano, nano_to_dollars};
//!
//! let nano = dollars_to_nano(3.0);  // 3_000_000_000
//! let dollars = nano_to_dollars(nano);  // 3.0
//! ```

/// Number of nanodollars in one dollar (9 decimal places precision)
pub const NANO_PER_DOLLAR: i64 = 1_000_000_000;

/// Scaling factor for exchange rates (9 decimal places precision)
pub const RATE_SCALE: i64 = 1_000_000_000;

/// Convert a dollar amount to nanodollars (i64).
///
/// Uses rounding to ensure the nearest integer nanodollar value. Finite values
/// outside the representable i64 range saturate at the corresponding bound.
/// `NaN` is treated as an invalid price and fails closed to `i64::MAX` rather
/// than silently becoming zero/free through Rust's float-to-integer cast.
///
/// # Arguments
/// * `price` - Price in dollars (e.g., 3.0 for $3.00)
///
/// # Returns
/// * Nanodollar amount as i64
///
/// # Example
/// ```
/// use burncloud_commerce_contracts::price_u64::dollars_to_nano;
///
/// assert_eq!(dollars_to_nano(3.0), 3_000_000_000);
/// assert_eq!(dollars_to_nano(0.15), 150_000_000);
/// assert_eq!(dollars_to_nano(0.00015), 150_000);
/// ```
pub fn dollars_to_nano(price: f64) -> i64 {
    if price.is_nan() {
        return i64::MAX;
    }
    (price * NANO_PER_DOLLAR as f64).round() as i64
}

/// Convert nanodollars (i64) to dollar amount.
///
/// # Arguments
/// * `nano` - Price in nanodollars
///
/// # Returns
/// * Price in dollars as f64
///
/// # Example
/// ```
/// use burncloud_commerce_contracts::price_u64::nano_to_dollars;
///
/// assert!((nano_to_dollars(3_000_000_000) - 3.0).abs() < 1e-9);
/// assert!((nano_to_dollars(150_000_000) - 0.15).abs() < 1e-9);
/// ```
pub fn nano_to_dollars(nano: i64) -> f64 {
    nano as f64 / NANO_PER_DOLLAR as f64
}

/// Convert an exchange rate to a scaled i64 value.
///
/// Scales the rate by 10^9 for precise storage and calculation. Finite values
/// outside the representable i64 range saturate at the corresponding bound.
/// `NaN` fails closed to `i64::MAX` instead of silently becoming zero.
///
/// # Arguments
/// * `rate` - Exchange rate as f64 (e.g., 7.24 for CNY/USD)
///
/// # Returns
/// * Scaled rate as i64
///
/// # Example
/// ```
/// use burncloud_commerce_contracts::price_u64::rate_to_scaled;
///
/// assert_eq!(rate_to_scaled(7.24), 7_240_000_000);
/// assert_eq!(rate_to_scaled(0.138), 138_000_000);
/// ```
pub fn rate_to_scaled(rate: f64) -> i64 {
    if rate.is_nan() {
        return i64::MAX;
    }
    (rate * RATE_SCALE as f64).round() as i64
}

/// Convert a scaled i64 exchange rate back to f64.
///
/// # Arguments
/// * `scaled` - Scaled exchange rate as i64 (can be negative)
///
/// # Returns
/// * Exchange rate as f64
///
/// # Example
/// ```
/// use burncloud_commerce_contracts::price_u64::scaled_to_rate;
///
/// assert!((scaled_to_rate(7_240_000_000) - 7.24).abs() < 1e-9);
/// ```
pub fn scaled_to_rate(scaled: i64) -> f64 {
    scaled as f64 / RATE_SCALE as f64
}

/// Calculate cost safely using i128 intermediate values to prevent overflow.
///
/// For billing calculations: cost = (tokens * price_per_million) / 1_000_000
///
/// # Arguments
/// * `tokens` - Number of tokens consumed
/// * `price_per_million_nano` - Price per 1M tokens in nanodollars
///
/// # Returns
/// * Cost in nanodollars as i64. Results outside the i64 range saturate at
///   `i64::MIN` / `i64::MAX` instead of wrapping during narrowing conversion.
///
/// # Overflow Protection
/// Uses i128 intermediate calculation to handle large token counts, then clamps
/// the final value before converting to i64.
///
/// # Example
/// ```
/// use burncloud_commerce_contracts::price_u64::calculate_cost_safe;
///
/// // 1M tokens at $3/1M = $3.00
/// let cost = calculate_cost_safe(1_000_000, 3_000_000_000);
/// assert_eq!(cost, 3_000_000_000);
///
/// // 150K tokens at $1.2/1M = $0.18
/// let cost = calculate_cost_safe(150_000, 1_200_000_000);
/// assert_eq!(cost, 180_000_000);
/// ```
pub fn calculate_cost_safe(tokens: u64, price_per_million_nano: i64) -> i64 {
    let tokens_128 = tokens as i128;
    let price_128 = price_per_million_nano as i128;
    let divisor: i128 = 1_000_000;

    let cost_128 = (tokens_128 * price_128) / divisor;
    cost_128.clamp(i64::MIN as i128, i64::MAX as i128) as i64
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "unit-test module: unwrap/expect failures are the intended failure signal"
)]
mod tests {
    use super::*;

    #[test]
    fn test_dollars_to_nano_basic() {
        assert_eq!(dollars_to_nano(3.0), 3_000_000_000);
        assert_eq!(dollars_to_nano(0.15), 150_000_000);
        assert_eq!(dollars_to_nano(0.00015), 150_000);
        assert_eq!(dollars_to_nano(0.0), 0);
    }

    #[test]
    fn test_dollars_to_nano_roundtrip() {
        let test_values = [0.0, 0.15, 1.0, 3.0, 10.5, 100.0, 1000.0];

        for &value in &test_values {
            let nano = dollars_to_nano(value);
            let back = nano_to_dollars(nano);
            let diff = (back - value).abs();
            assert!(
                diff < 1e-9,
                "Roundtrip failed for {}: nano={}, back={}, diff={}",
                value,
                nano,
                back,
                diff
            );
        }
    }

    #[test]
    fn test_nine_decimal_precision() {
        assert_eq!(dollars_to_nano(0.000_000_001), 1);
        assert_eq!(dollars_to_nano(1.000_000_001), 1_000_000_001);
        assert_eq!(dollars_to_nano(0.123_456_789), 123_456_789);

        assert!((nano_to_dollars(1) - 0.000_000_001).abs() < 1e-12);
        assert!((nano_to_dollars(123_456_789) - 0.123_456_789).abs() < 1e-12);
    }

    #[test]
    fn test_rate_to_scaled_roundtrip() {
        let test_rates = [0.138, 1.0, 1.08, 7.24, 150.5];

        for &rate in &test_rates {
            let scaled = rate_to_scaled(rate);
            let back = scaled_to_rate(scaled);
            let diff = (back - rate).abs();
            assert!(
                diff < 1e-9,
                "Rate roundtrip failed for {}: scaled={}, back={}, diff={}",
                rate,
                scaled,
                back,
                diff
            );
        }
    }

    #[test]
    fn test_calculate_cost_safe_basic() {
        assert_eq!(calculate_cost_safe(1_000_000, 3_000_000_000), 3_000_000_000);
        assert_eq!(calculate_cost_safe(500_000, 2_000_000_000), 1_000_000_000);
        assert_eq!(calculate_cost_safe(150_000, 1_200_000_000), 180_000_000);
        assert_eq!(calculate_cost_safe(0, 3_000_000_000), 0);
        assert_eq!(calculate_cost_safe(1_000_000, 0), 0);
    }

    #[test]
    fn test_overflow_protection() {
        let large_tokens = 10_000_000_000u64;
        let high_price = 1_000_000_000_000i64;
        let cost = calculate_cost_safe(large_tokens, high_price);
        assert_eq!(cost, 10_000_000_000_000_000);
    }

    #[test]
    fn cost_saturates_instead_of_wrapping_when_final_value_exceeds_i64() {
        let tokens = 1_000_000_000_000u64;
        let price = 1_000_000_000_000_000i64;
        assert_eq!(calculate_cost_safe(tokens, price), i64::MAX);
        assert_eq!(calculate_cost_safe(tokens, -price), i64::MIN);
    }

    #[test]
    fn nan_price_and_rate_fail_closed_instead_of_becoming_zero() {
        assert_eq!(dollars_to_nano(f64::NAN), i64::MAX);
        assert_eq!(rate_to_scaled(f64::NAN), i64::MAX);
    }

    #[test]
    fn finite_float_conversion_preserves_existing_rounding_and_saturation() {
        assert_eq!(dollars_to_nano(0.000_000_001_5), 2);
        assert_eq!(dollars_to_nano(-0.000_000_001_5), -2);
        assert_eq!(dollars_to_nano(f64::INFINITY), i64::MAX);
        assert_eq!(dollars_to_nano(f64::NEG_INFINITY), i64::MIN);
        assert_eq!(rate_to_scaled(f64::INFINITY), i64::MAX);
        assert_eq!(rate_to_scaled(f64::NEG_INFINITY), i64::MIN);
    }

    #[test]
    fn test_tiered_pricing_scenario() {
        let tier1_tokens = 32_000u64;
        let tier1_price = 1_200_000_000i64;
        let tier1_cost = calculate_cost_safe(tier1_tokens, tier1_price);
        assert_eq!(tier1_cost, 38_400_000);

        let tier2_tokens = 96_000u64;
        let tier2_price = 2_400_000_000i64;
        let tier2_cost = calculate_cost_safe(tier2_tokens, tier2_price);
        assert_eq!(tier2_cost, 230_400_000);

        let tier3_tokens = 22_000u64;
        let tier3_price = 3_000_000_000i64;
        let tier3_cost = calculate_cost_safe(tier3_tokens, tier3_price);
        assert_eq!(tier3_cost, 66_000_000);

        let total_cost = tier1_cost + tier2_cost + tier3_cost;
        assert_eq!(total_cost, 334_800_000);
        assert!((nano_to_dollars(total_cost) - 0.3348).abs() < 1e-9);
    }

    #[test]
    fn test_edge_cases() {
        assert_eq!(dollars_to_nano(0.000_000_001), 1);

        #[allow(
            clippy::inconsistent_digit_grouping,
            reason = "the sub-nanodollar probe value is more readable with its trailing half-unit digit split out"
        )]
        let val1 = 0.000_000_0015;
        assert_eq!(dollars_to_nano(val1), 2);

        #[allow(
            clippy::inconsistent_digit_grouping,
            reason = "the sub-nanodollar probe value is more readable with its trailing digit split out"
        )]
        let val2 = 0.000_000_0014;
        assert_eq!(dollars_to_nano(val2), 1);
    }

    #[test]
    fn test_constants() {
        assert_eq!(NANO_PER_DOLLAR, 1_000_000_000);
        assert_eq!(RATE_SCALE, 1_000_000_000);
    }
}
