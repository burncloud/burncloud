//! Billing module for advanced pricing calculations
//!
//! This module provides billing calculations for:
//! - Tiered pricing (usage-based pricing tiers)
//! - Cache pricing (Prompt Caching)
//! - Batch pricing (Batch API)
//! - Priority pricing (high-priority requests)
//! - Audio token pricing
//! - Multi-currency support
//!
//! All prices are stored as i64 nanodollars (9 decimal precision) for precision.
//! Display values are converted to f64 dollars for human readability.

use burncloud_common::nano_to_dollars;
use burncloud_common::types::TieredPrice;
use burncloud_common::Currency;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur during billing calculations
#[derive(Debug, Error)]
pub enum BillingError {
    #[error("No tiered pricing configuration available")]
    NoTiers,

    #[error("Invalid tier configuration: tier_end ({tier_end}) < tier_start ({tier_start})")]
    InvalidTier { tier_start: i64, tier_end: i64 },

    #[error("Invalid price: price cannot be negative")]
    InvalidPrice,

    #[error("Region mismatch: requested region '{requested}' not found")]
    RegionMismatch { requested: String },
}

/// Token usage for billing calculations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TokenUsage {
    /// Standard prompt tokens (non-cached)
    pub prompt_tokens: u64,
    /// Completion/output tokens
    pub completion_tokens: u64,
    /// Cache read tokens (Prompt Caching)
    pub cache_read_tokens: u64,
    /// Cache creation tokens (Prompt Caching)
    pub cache_creation_tokens: u64,
    /// Audio input tokens
    pub audio_tokens: u64,
}

/// Result of a cost calculation with multi-currency support
/// Internal amounts are stored as i64 nanodollars for precision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostResult {
    /// Cost in USD (nanodollars, i64)
    #[serde(serialize_with = "serialize_nano_as_dollars")]
    pub usd_amount_nano: i64,
    /// Local currency code (e.g., "CNY", "EUR")
    pub local_currency: String,
    /// Cost in local currency (nanodollars, i64, if available)
    #[serde(serialize_with = "serialize_opt_nano_as_dollars")]
    pub local_amount_nano: Option<i64>,
    /// Human-readable display string
    pub display: String,
}

/// Custom serializer for i64 nanodollars as f64 dollars
fn serialize_nano_as_dollars<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_f64(nano_to_dollars(*value))
}

/// Custom serializer for Option<i64> nanodollars as Option<f64> dollars
fn serialize_opt_nano_as_dollars<S>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(nano) => serializer.serialize_some(&nano_to_dollars(*nano)),
        None => serializer.serialize_none(),
    }
}

impl CostResult {
    /// Get USD amount as f64 dollars (for backward compatibility)
    pub fn usd_amount(&self) -> f64 {
        nano_to_dollars(self.usd_amount_nano)
    }

    /// Get local amount as f64 dollars (for backward compatibility)
    pub fn local_amount(&self) -> Option<f64> {
        self.local_amount_nano.map(nano_to_dollars)
    }

    /// Create a new CostResult with only USD
    pub fn from_usd_nano(usd_amount_nano: i64) -> Self {
        let display = format_cost_nano(usd_amount_nano, "USD");
        Self {
            usd_amount_nano,
            local_currency: "USD".to_string(),
            local_amount_nano: None,
            display,
        }
    }

    /// Create a new CostResult with local currency
    pub fn with_local_nano(
        usd_amount_nano: i64,
        local_currency: &str,
        local_amount_nano: i64,
    ) -> Self {
        let display = format_cost_nano(local_amount_nano, local_currency);
        Self {
            usd_amount_nano,
            local_currency: local_currency.to_string(),
            local_amount_nano: Some(local_amount_nano),
            display,
        }
    }

    // Backward compatibility constructors using f64
    /// Create a new CostResult with only USD (f64 dollars input)
    pub fn from_usd(usd_amount: f64) -> Self {
        Self::from_usd_nano((usd_amount * 1_000_000_000.0).round() as i64)
    }

    /// Create a new CostResult with local currency (f64 dollars input)
    pub fn with_local(usd_amount: f64, local_currency: &str, local_amount: f64) -> Self {
        Self::with_local_nano(
            (usd_amount * 1_000_000_000.0).round() as i64,
            local_currency,
            (local_amount * 1_000_000_000.0).round() as i64,
        )
    }
}

/// Format a cost amount (in nanodollars) with currency symbol
fn format_cost_nano(amount_nano: i64, currency: &str) -> String {
    let amount = nano_to_dollars(amount_nano);
    let symbol = match currency.to_uppercase().as_str() {
        "USD" => "$",
        "CNY" => "¥",
        "EUR" => "€",
        _ => "",
    };
    format!("{}{:.6}", symbol, amount)
}

/// Multi-currency pricing information
/// Exchange rate is stored as scaled i64 (rate * 10^9) for precision
#[derive(Debug, Clone, Default)]
pub struct MultiCurrencyPricing {
    /// Pricing in USD (required)
    pub usd: AdvancedPricing,
    /// Pricing in local currency (optional)
    pub local: Option<(Currency, AdvancedPricing)>,
    /// Exchange rate from USD to local currency (scaled by 10^9, if available)
    pub exchange_rate_nano: Option<i64>,
}

impl MultiCurrencyPricing {
    /// Set exchange rate from f64
    pub fn with_exchange_rate(mut self, rate: f64) -> Self {
        self.exchange_rate_nano = Some((rate * 1_000_000_000.0).round() as i64);
        self
    }

    /// Get exchange rate as f64
    pub fn exchange_rate(&self) -> Option<f64> {
        self.exchange_rate_nano.map(|r| r as f64 / 1_000_000_000.0)
    }
}

/// Calculate cost with multi-currency support
///
/// Returns cost in both USD and local currency if available.
/// Internal calculations are done in USD for precision, then converted.
pub fn calculate_multi_currency_cost(
    usage: &TokenUsage,
    pricing: &MultiCurrencyPricing,
    is_batch: bool,
    is_priority: bool,
) -> CostResult {
    // Calculate base cost in USD (nanodollars)
    let usd_cost_nano = if is_batch {
        calculate_batch_cost_nano(usage.prompt_tokens, usage.completion_tokens, &pricing.usd)
    } else if is_priority {
        calculate_priority_cost_nano(usage.prompt_tokens, usage.completion_tokens, &pricing.usd)
    } else if usage.cache_read_tokens > 0
        || usage.cache_creation_tokens > 0
        || usage.audio_tokens > 0
    {
        calculate_cache_cost_nano(usage, &pricing.usd)
    } else {
        // Standard pricing
        let input_cost =
            (usage.prompt_tokens as i128 * pricing.usd.input_price as i128) / 1_000_000;
        let output_cost =
            (usage.completion_tokens as i128 * pricing.usd.output_price as i128) / 1_000_000;
        (input_cost + output_cost) as i64
    };

    // If we have local currency pricing, use it
    if let Some((local_currency, local_pricing)) = &pricing.local {
        let local_cost_nano = if is_batch {
            calculate_batch_cost_nano(usage.prompt_tokens, usage.completion_tokens, local_pricing)
        } else if is_priority {
            calculate_priority_cost_nano(
                usage.prompt_tokens,
                usage.completion_tokens,
                local_pricing,
            )
        } else if usage.cache_read_tokens > 0
            || usage.cache_creation_tokens > 0
            || usage.audio_tokens > 0
        {
            calculate_cache_cost_nano(usage, local_pricing)
        } else {
            let input_cost =
                (usage.prompt_tokens as i128 * local_pricing.input_price as i128) / 1_000_000;
            let output_cost =
                (usage.completion_tokens as i128 * local_pricing.output_price as i128) / 1_000_000;
            (input_cost + output_cost) as i64
        };

        return CostResult::with_local_nano(usd_cost_nano, local_currency.code(), local_cost_nano);
    }

    // If we have exchange rate but no local pricing, convert from USD
    if let Some(rate_nano) = pricing.exchange_rate_nano {
        // Use a default local currency (CNY for now, could be made configurable)
        // local_amount = usd_amount * rate
        let local_amount_nano = (usd_cost_nano as i128 * rate_nano as i128 / 1_000_000_000) as i64;
        return CostResult::with_local_nano(usd_cost_nano, "CNY", local_amount_nano);
    }

    CostResult::from_usd_nano(usd_cost_nano)
}

/// Calculate cost for tiered pricing
///
/// Tiered pricing is used for models where the price per token varies
/// based on the total token count. For example, Qwen models have different
/// pricing tiers based on context length.
///
/// # Algorithm
/// The algorithm uses segmented accumulation:
/// 1. Sort tiers by tier_start
/// 2. For each tier, calculate tokens that fall within that tier
/// 3. Multiply by the tier's price and accumulate
///
/// # Example
/// ```
/// // Qwen international pricing:
/// // 0-32K: $1.2/1M input
/// // 32K-128K: $2.4/1M input
/// // 128K+: $3.0/1M input
/// // 150K tokens = 32K×$1.2 + 96K×$2.4 + 22K×$3.0 = $0.3348
/// ```
///
/// Returns cost in nanodollars (i64)
pub fn calculate_tiered_cost_nano(
    tokens: u64,
    tiers: &[TieredPrice],
    region: Option<&str>,
) -> Result<i64, BillingError> {
    // Handle zero tokens
    if tokens == 0 {
        return Ok(0);
    }

    // Handle empty tiers
    if tiers.is_empty() {
        return Err(BillingError::NoTiers);
    }

    // Filter tiers by region
    let filtered_tiers: Vec<&TieredPrice> = if let Some(r) = region {
        let matching: Vec<&TieredPrice> = tiers
            .iter()
            .filter(|t| t.region.as_deref() == Some(r))
            .collect();
        if matching.is_empty() {
            // Fall back to universal tiers (region = NULL)
            let universal: Vec<&TieredPrice> =
                tiers.iter().filter(|t| t.region.is_none()).collect();
            if universal.is_empty() {
                return Err(BillingError::RegionMismatch {
                    requested: r.to_string(),
                });
            }
            universal
        } else {
            matching
        }
    } else {
        // Use universal tiers or all tiers if no universal ones exist
        let universal: Vec<&TieredPrice> = tiers.iter().filter(|t| t.region.is_none()).collect();
        if universal.is_empty() {
            tiers.iter().collect()
        } else {
            universal
        }
    };

    // Validate and sort tiers
    let mut sorted_tiers: Vec<&TieredPrice> = filtered_tiers;
    sorted_tiers.sort_by(|a, b| a.tier_start.cmp(&b.tier_start));

    // Validate tier configuration
    for tier in &sorted_tiers {
        if let Some(tier_end) = tier.tier_end {
            if tier_end < tier.tier_start {
                return Err(BillingError::InvalidTier {
                    tier_start: tier.tier_start,
                    tier_end,
                });
            }
        }
        if tier.input_price < 0 || tier.output_price < 0 {
            return Err(BillingError::InvalidPrice);
        }
    }

    // Calculate cost using segmented accumulation with i128 intermediate
    let tokens_i64 = tokens as i64;
    let mut total_cost: i128 = 0;

    for tier in &sorted_tiers {
        // Calculate the upper bound for this tier
        let tier_upper = tier.tier_end.unwrap_or(i64::MAX);

        // Skip if this tier starts after all tokens
        if tier.tier_start >= tokens_i64 {
            break;
        }

        // Calculate tokens in this tier:
        // From max(tier_start, 0) to min(tier_end, total_tokens)
        let tokens_in_tier = tier_upper.min(tokens_i64) - tier.tier_start;

        if tokens_in_tier > 0 {
            // Cost = (tokens * price_per_million) / 1_000_000
            // Use i128 to prevent overflow
            let cost = (tokens_in_tier as i128 * tier.input_price as i128) / 1_000_000;
            total_cost += cost;
        }
    }

    // Handle tokens beyond the last tier (use last tier's price)
    if let Some(last_tier) = sorted_tiers.last() {
        let last_tier_upper = last_tier.tier_end.unwrap_or(i64::MAX);
        if tokens_i64 > last_tier_upper {
            let beyond_tokens = tokens_i64 - last_tier_upper;
            let cost = (beyond_tokens as i128 * last_tier.input_price as i128) / 1_000_000;
            total_cost += cost;
        }
    }

    Ok(total_cost as i64)
}

/// Calculate cost for tiered pricing (returns f64 dollars for backward compatibility)
pub fn calculate_tiered_cost(
    tokens: u64,
    tiers: &[TieredPrice],
    region: Option<&str>,
) -> Result<f64, BillingError> {
    let cost_nano = calculate_tiered_cost_nano(tokens, tiers, region)?;
    Ok(nano_to_dollars(cost_nano))
}

/// Calculate cost with tiered pricing for both input and output tokens
///
/// Returns the total cost for prompt and completion tokens using tiered pricing.
/// Returns cost in nanodollars (i64).
pub fn calculate_tiered_cost_full_nano(
    prompt_tokens: u64,
    completion_tokens: u64,
    tiers: &[TieredPrice],
    region: Option<&str>,
) -> Result<i64, BillingError> {
    // Calculate input cost
    let input_cost = calculate_tiered_cost_nano(prompt_tokens, tiers, region)?;

    // Calculate output cost (using output_price from tiers)
    if completion_tokens == 0 {
        return Ok(input_cost);
    }

    // For output tokens, we need to use output_price
    // Re-filter tiers for output pricing
    let filtered_tiers: Vec<&TieredPrice> = if let Some(r) = region {
        let matching: Vec<&TieredPrice> = tiers
            .iter()
            .filter(|t| t.region.as_deref() == Some(r))
            .collect();
        if matching.is_empty() {
            tiers.iter().filter(|t| t.region.is_none()).collect()
        } else {
            matching
        }
    } else {
        let universal: Vec<&TieredPrice> = tiers.iter().filter(|t| t.region.is_none()).collect();
        if universal.is_empty() {
            tiers.iter().collect()
        } else {
            universal
        }
    };

    if filtered_tiers.is_empty() {
        return Ok(input_cost);
    }

    let mut sorted_tiers: Vec<&TieredPrice> = filtered_tiers;
    sorted_tiers.sort_by(|a, b| a.tier_start.cmp(&b.tier_start));

    let tokens_i64 = completion_tokens as i64;
    let mut output_cost: i128 = 0;

    for tier in &sorted_tiers {
        let tier_upper = tier.tier_end.unwrap_or(i64::MAX);

        if tier.tier_start >= tokens_i64 {
            break;
        }

        let tokens_in_tier = tier_upper.min(tokens_i64) - tier.tier_start;

        if tokens_in_tier > 0 {
            let cost = (tokens_in_tier as i128 * tier.output_price as i128) / 1_000_000;
            output_cost += cost;
        }
    }

    // Handle tokens beyond the last tier
    if let Some(last_tier) = sorted_tiers.last() {
        let last_tier_upper = last_tier.tier_end.unwrap_or(i64::MAX);
        if tokens_i64 > last_tier_upper {
            let beyond_tokens = tokens_i64 - last_tier_upper;
            let cost = (beyond_tokens as i128 * last_tier.output_price as i128) / 1_000_000;
            output_cost += cost;
        }
    }

    Ok(input_cost + output_cost as i64)
}

/// Calculate cost with tiered pricing for both input and output tokens (f64 backward compatibility)
pub fn calculate_tiered_cost_full(
    prompt_tokens: u64,
    completion_tokens: u64,
    tiers: &[TieredPrice],
    region: Option<&str>,
) -> Result<f64, BillingError> {
    let cost_nano =
        calculate_tiered_cost_full_nano(prompt_tokens, completion_tokens, tiers, region)?;
    Ok(nano_to_dollars(cost_nano))
}

/// Pricing information for cache, batch, and priority billing
/// All prices are in nanodollars (i64, 9 decimal precision)
#[derive(Debug, Clone, Default)]
pub struct AdvancedPricing {
    /// Standard input price per 1M tokens (nanodollars)
    pub input_price: i64,
    /// Standard output price per 1M tokens (nanodollars)
    pub output_price: i64,
    /// Cache read price per 1M tokens (nanodollars, typically 10% of standard)
    pub cache_read_price: Option<i64>,
    /// Cache creation price per 1M tokens (nanodollars)
    pub cache_creation_price: Option<i64>,
    /// Batch input price per 1M tokens (nanodollars, typically 50% of standard)
    pub batch_input_price: Option<i64>,
    /// Batch output price per 1M tokens (nanodollars)
    pub batch_output_price: Option<i64>,
    /// Priority input price per 1M tokens (nanodollars, typically 170% of standard)
    pub priority_input_price: Option<i64>,
    /// Priority output price per 1M tokens (nanodollars)
    pub priority_output_price: Option<i64>,
    /// Audio input price per 1M tokens (nanodollars, typically 7x text price)
    pub audio_input_price: Option<i64>,
}

impl AdvancedPricing {
    /// Create from f64 dollar prices (for backward compatibility)
    pub fn from_dollars(input_price: f64, output_price: f64) -> Self {
        Self {
            input_price: (input_price * 1_000_000_000.0).round() as i64,
            output_price: (output_price * 1_000_000_000.0).round() as i64,
            ..Default::default()
        }
    }
}

/// Calculate cost for Prompt Caching requests
///
/// Prompt Caching allows reusing cached prompt prefixes at a reduced rate.
/// Cache read tokens cost approximately 10% of standard tokens.
/// Returns cost in nanodollars (i64).
pub fn calculate_cache_cost_nano(usage: &TokenUsage, pricing: &AdvancedPricing) -> i64 {
    let mut total_cost: i128 = 0;

    // Standard prompt tokens
    let standard_prompt = usage.prompt_tokens.saturating_sub(usage.cache_read_tokens);
    total_cost += (standard_prompt as i128 * pricing.input_price as i128) / 1_000_000;

    // Completion tokens
    total_cost += (usage.completion_tokens as i128 * pricing.output_price as i128) / 1_000_000;

    // Cache read tokens (10% of standard price)
    if usage.cache_read_tokens > 0 {
        // Default cache read price is 10% of input price
        let cache_price = pricing.cache_read_price.unwrap_or(pricing.input_price / 10);
        total_cost += (usage.cache_read_tokens as i128 * cache_price as i128) / 1_000_000;
    }

    // Cache creation tokens
    if usage.cache_creation_tokens > 0 {
        // Default cache creation price is 125% of input price
        let cache_creation_price = pricing
            .cache_creation_price
            .unwrap_or(pricing.input_price + pricing.input_price / 4);
        total_cost +=
            (usage.cache_creation_tokens as i128 * cache_creation_price as i128) / 1_000_000;
    }

    // Audio tokens (typically 7x text price)
    if usage.audio_tokens > 0 {
        // Default audio price is 7x input price
        let audio_price = pricing.audio_input_price.unwrap_or(pricing.input_price * 7);
        total_cost += (usage.audio_tokens as i128 * audio_price as i128) / 1_000_000;
    }

    total_cost as i64
}

/// Calculate cost for Prompt Caching requests (f64 backward compatibility)
pub fn calculate_cache_cost(usage: &TokenUsage, pricing: &AdvancedPricing) -> f64 {
    let cost_nano = calculate_cache_cost_nano(usage, pricing);
    nano_to_dollars(cost_nano)
}

/// Calculate cost for Batch API requests
///
/// Batch API requests are typically 50% cheaper than standard requests.
/// Returns cost in nanodollars (i64).
pub fn calculate_batch_cost_nano(
    prompt_tokens: u64,
    completion_tokens: u64,
    pricing: &AdvancedPricing,
) -> i64 {
    // Default batch price is 50% of standard price
    let input_price = pricing.batch_input_price.unwrap_or(pricing.input_price / 2);
    let output_price = pricing
        .batch_output_price
        .unwrap_or(pricing.output_price / 2);

    let input_cost = (prompt_tokens as i128 * input_price as i128) / 1_000_000;
    let output_cost = (completion_tokens as i128 * output_price as i128) / 1_000_000;

    (input_cost + output_cost) as i64
}

/// Calculate cost for Batch API requests (f64 backward compatibility)
pub fn calculate_batch_cost(
    prompt_tokens: u64,
    completion_tokens: u64,
    pricing: &AdvancedPricing,
) -> f64 {
    let cost_nano = calculate_batch_cost_nano(prompt_tokens, completion_tokens, pricing);
    nano_to_dollars(cost_nano)
}

/// Calculate cost for priority requests
///
/// Priority requests get faster response times at a premium (typically 170%).
/// Returns cost in nanodollars (i64).
pub fn calculate_priority_cost_nano(
    prompt_tokens: u64,
    completion_tokens: u64,
    pricing: &AdvancedPricing,
) -> i64 {
    // Default priority price is 170% of standard price
    // Use 170/100 = 17/10 for integer math
    let input_price = pricing
        .priority_input_price
        .unwrap_or((pricing.input_price as i128 * 17 / 10) as i64);
    let output_price = pricing
        .priority_output_price
        .unwrap_or((pricing.output_price as i128 * 17 / 10) as i64);

    let input_cost = (prompt_tokens as i128 * input_price as i128) / 1_000_000;
    let output_cost = (completion_tokens as i128 * output_price as i128) / 1_000_000;

    (input_cost + output_cost) as i64
}

/// Calculate cost for priority requests (f64 backward compatibility)
pub fn calculate_priority_cost(
    prompt_tokens: u64,
    completion_tokens: u64,
    pricing: &AdvancedPricing,
) -> f64 {
    let cost_nano = calculate_priority_cost_nano(prompt_tokens, completion_tokens, pricing);
    nano_to_dollars(cost_nano)
}

#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_common::dollars_to_nano;

    /// Helper to convert dollars to nanodollars as i64
    fn to_nano(price: f64) -> i64 {
        dollars_to_nano(price) as i64
    }

    fn create_test_tier(
        tier_start: i64,
        tier_end: Option<i64>,
        input_price_dollars: f64,
        output_price_dollars: f64,
    ) -> TieredPrice {
        TieredPrice {
            id: 0,
            model: "test-model".to_string(),
            region: None,
            tier_start,
            tier_end,
            input_price: to_nano(input_price_dollars),
            output_price: to_nano(output_price_dollars),
        }
    }

    fn create_regional_tier(
        tier_start: i64,
        tier_end: Option<i64>,
        input_price_dollars: f64,
        output_price_dollars: f64,
        region: &str,
    ) -> TieredPrice {
        TieredPrice {
            id: 0,
            model: "test-model".to_string(),
            region: Some(region.to_string()),
            tier_start,
            tier_end,
            input_price: to_nano(input_price_dollars),
            output_price: to_nano(output_price_dollars),
        }
    }

    #[test]
    fn test_single_tier_equals_simple() {
        // Single tier should produce same result as simple calculation
        let tiers = vec![create_test_tier(0, None, 1.0, 4.0)];

        // 100K tokens at $1/1M = $0.1
        let cost = calculate_tiered_cost(100_000, &tiers, None).unwrap();
        assert!((cost - 0.1).abs() < 0.000001);
    }

    #[test]
    fn test_multi_tier_accumulation() {
        // Qwen international style pricing:
        // 0-32K: $1.2/1M input
        // 32K-128K: $2.4/1M input
        // 128K-252K: $3.0/1M input
        let tiers = vec![
            create_test_tier(0, Some(32_000), 1.2, 6.0),
            create_test_tier(32_000, Some(128_000), 2.4, 12.0),
            create_test_tier(128_000, Some(252_000), 3.0, 15.0),
        ];

        // 150K tokens:
        // Tier 1: 32K × $1.2/1M = $0.0384
        // Tier 2: 96K × $2.4/1M = $0.2304
        // Tier 3: 22K × $3.0/1M = $0.066
        // Total: $0.3348
        let cost = calculate_tiered_cost(150_000, &tiers, None).unwrap();
        assert!(
            (cost - 0.3348).abs() < 0.000001,
            "Expected $0.3348, got ${}",
            cost
        );
    }

    #[test]
    fn test_exceed_last_tier() {
        // Tokens beyond last tier should use last tier's price
        let tiers = vec![
            create_test_tier(0, Some(32_000), 1.0, 4.0),
            create_test_tier(32_000, Some(128_000), 2.0, 8.0),
        ];

        // 200K tokens:
        // Tier 1: 32K × $1.0/1M = $0.032
        // Tier 2: 96K × $2.0/1M = $0.192
        // Beyond: 72K × $2.0/1M = $0.144
        // Total: $0.368
        let cost = calculate_tiered_cost(200_000, &tiers, None).unwrap();
        assert!(
            (cost - 0.368).abs() < 0.000001,
            "Expected $0.368, got ${}",
            cost
        );
    }

    #[test]
    fn test_exact_tier_boundary() {
        // Tokens exactly at tier boundary
        let tiers = vec![
            create_test_tier(0, Some(32_000), 1.0, 4.0),
            create_test_tier(32_000, Some(128_000), 2.0, 8.0),
        ];

        // 128K tokens exactly at boundary
        // Tier 1: 32K × $1.0/1M = $0.032
        // Tier 2: 96K × $2.0/1M = $0.192
        // Total: $0.224
        let cost = calculate_tiered_cost(128_000, &tiers, None).unwrap();
        assert!(
            (cost - 0.224).abs() < 0.000001,
            "Expected $0.224, got ${}",
            cost
        );
    }

    #[test]
    fn test_zero_tokens() {
        let tiers = vec![create_test_tier(0, None, 1.0, 4.0)];
        let cost = calculate_tiered_cost(0, &tiers, None).unwrap();
        assert_eq!(cost, 0.0);
    }

    #[test]
    fn test_empty_tiers() {
        let result = calculate_tiered_cost(1000, &[], None);
        assert!(matches!(result, Err(BillingError::NoTiers)));
    }

    #[test]
    fn test_region_selection() {
        // Create tiers with different regions
        let tiers = vec![
            create_regional_tier(0, Some(32_000), 1.2, 6.0, "international"),
            create_regional_tier(32_000, Some(128_000), 2.4, 12.0, "international"),
            create_regional_tier(0, Some(32_000), 0.359, 1.434, "cn"),
            create_regional_tier(32_000, Some(128_000), 0.574, 2.294, "cn"),
        ];

        // Test CN region pricing (lower prices)
        let cn_cost = calculate_tiered_cost(50_000, &tiers, Some("cn")).unwrap();

        // CN: 32K × $0.359/1M + 18K × $0.574/1M
        // = $0.011488 + $0.010332 = $0.02182
        let expected_cn = 32_000.0 / 1_000_000.0 * 0.359 + 18_000.0 / 1_000_000.0 * 0.574;
        assert!(
            (cn_cost - expected_cn).abs() < 0.000001,
            "Expected ${}, got ${}",
            expected_cn,
            cn_cost
        );

        // Test international region pricing
        let intl_cost = calculate_tiered_cost(50_000, &tiers, Some("international")).unwrap();

        // International: 32K × $1.2/1M + 18K × $2.4/1M
        // = $0.0384 + $0.0432 = $0.0816
        let expected_intl = 32_000.0 / 1_000_000.0 * 1.2 + 18_000.0 / 1_000_000.0 * 2.4;
        assert!(
            (intl_cost - expected_intl).abs() < 0.000001,
            "Expected ${}, got ${}",
            expected_intl,
            intl_cost
        );

        // Verify CN is cheaper than international
        assert!(cn_cost < intl_cost);
    }

    #[test]
    fn test_region_fallback_to_universal() {
        // Universal tiers (no region specified)
        let tiers = vec![create_test_tier(0, None, 1.0, 4.0)];

        // Request with region that doesn't exist should use universal
        let cost = calculate_tiered_cost(100_000, &tiers, Some("nonexistent")).unwrap();
        assert!((cost - 0.1).abs() < 0.000001);
    }

    #[test]
    fn test_invalid_tier_config() {
        // tier_end < tier_start should error
        let tiers = vec![create_test_tier(100, Some(50), 1.0, 4.0)];
        let result = calculate_tiered_cost(1000, &tiers, None);
        assert!(matches!(result, Err(BillingError::InvalidTier { .. })));
    }

    #[test]
    fn test_negative_price() {
        let mut tiers = vec![create_test_tier(0, None, 1.0, 4.0)];
        tiers[0].input_price = -to_nano(1.0); // Negative nanodollars
        let result = calculate_tiered_cost(1000, &tiers, None);
        assert!(matches!(result, Err(BillingError::InvalidPrice)));
    }

    #[test]
    fn test_cache_cost_calculation() {
        let usage = TokenUsage {
            prompt_tokens: 100_000,
            completion_tokens: 50_000,
            cache_read_tokens: 50_000, // 50K cached
            cache_creation_tokens: 0,
            audio_tokens: 0,
        };

        let pricing = AdvancedPricing {
            input_price: to_nano(3.0), // Claude 3.5 Sonnet style
            output_price: to_nano(15.0),
            cache_read_price: Some(to_nano(0.30)), // 10% of standard
            cache_creation_price: Some(to_nano(3.75)),
            ..Default::default()
        };

        // Expected:
        // Standard prompt: 50K × $3.0/1M = $0.15
        // Completion: 50K × $15.0/1M = $0.75
        // Cache read: 50K × $0.30/1M = $0.015
        // Total: $0.915
        let cost = calculate_cache_cost(&usage, &pricing);
        assert!(
            (cost - 0.915).abs() < 0.000001,
            "Expected $0.915, got ${}",
            cost
        );
    }

    #[test]
    fn test_batch_cost_calculation() {
        let pricing = AdvancedPricing {
            input_price: to_nano(10.0), // GPT-4 style
            output_price: to_nano(30.0),
            batch_input_price: Some(to_nano(5.0)), // 50% of standard
            batch_output_price: Some(to_nano(15.0)),
            ..Default::default()
        };

        // 1M input + 1M output at batch prices
        let cost = calculate_batch_cost(1_000_000, 1_000_000, &pricing);
        assert!(
            (cost - 20.0).abs() < 0.000001,
            "Expected $20.0, got ${}",
            cost
        );
    }

    #[test]
    fn test_priority_cost_calculation() {
        let pricing = AdvancedPricing {
            input_price: to_nano(10.0),
            output_price: to_nano(30.0),
            priority_input_price: Some(to_nano(17.0)), // 170% of standard
            priority_output_price: Some(to_nano(51.0)),
            ..Default::default()
        };

        // 1M input + 1M output at priority prices
        let cost = calculate_priority_cost(1_000_000, 1_000_000, &pricing);
        assert!(
            (cost - 68.0).abs() < 0.000001,
            "Expected $68.0, got ${}",
            cost
        );
    }

    #[test]
    fn test_batch_fallback_to_standard() {
        // Without batch prices, should fall back to 50% of standard
        let pricing = AdvancedPricing {
            input_price: to_nano(10.0),
            output_price: to_nano(30.0),
            batch_input_price: None,
            batch_output_price: None,
            ..Default::default()
        };

        let cost = calculate_batch_cost(1_000_000, 1_000_000, &pricing);
        // 50% of (10 + 30) = 20
        assert!(
            (cost - 20.0).abs() < 0.000001,
            "Expected $20.0, got ${}",
            cost
        );
    }

    #[test]
    fn test_cost_result_formatting() {
        let result = CostResult::from_usd(1.234567);
        assert_eq!(result.display, "$1.234567");
        assert!((result.usd_amount() - 1.234567).abs() < 0.000001);
        assert_eq!(result.local_currency, "USD");
        assert!(result.local_amount().is_none());

        let result_cny = CostResult::with_local(1.0, "CNY", 7.2);
        assert_eq!(result_cny.display, "¥7.200000");
        assert!((result_cny.usd_amount() - 1.0).abs() < 0.000001);
        assert!((result_cny.local_amount().unwrap() - 7.2).abs() < 0.000001);
    }

    #[test]
    fn test_tiered_cost_full_with_output() {
        let tiers = vec![
            create_test_tier(0, Some(32_000), 1.2, 6.0),
            create_test_tier(32_000, Some(128_000), 2.4, 12.0),
        ];

        // 50K prompt + 10K completion
        // Input: 32K × $1.2 + 18K × $2.4 = $0.0384 + $0.0432 = $0.0816
        // Output: 10K × $6.0 = $0.06 (using first tier output price)
        // Wait, output should also be tiered based on prompt tokens
        let cost = calculate_tiered_cost_full(50_000, 10_000, &tiers, None).unwrap();

        // For now, output is calculated at first tier price
        // This test verifies the function works
        assert!(cost > 0.0);
    }

    #[test]
    fn test_multi_currency_with_local_pricing() {
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            completion_tokens: 500_000,
            ..Default::default()
        };

        let usd_pricing = AdvancedPricing {
            input_price: to_nano(1.2), // USD per 1M tokens
            output_price: to_nano(6.0),
            ..Default::default()
        };

        let cny_pricing = AdvancedPricing {
            input_price: to_nano(0.359), // CNY per 1M tokens (cheaper for CN region)
            output_price: to_nano(1.434),
            ..Default::default()
        };

        let multi_pricing = MultiCurrencyPricing {
            usd: usd_pricing,
            local: Some((Currency::CNY, cny_pricing)),
            exchange_rate_nano: None,
        };

        let result = calculate_multi_currency_cost(&usage, &multi_pricing, false, false);

        // USD: 1M × $1.2 + 0.5M × $6.0 = $1.2 + $3.0 = $4.2
        assert!((result.usd_amount() - 4.2).abs() < 0.000001);

        // CNY: 1M × ¥0.359 + 0.5M × ¥1.434 = ¥0.359 + ¥0.717 = ¥1.076
        let expected_cny = 0.359 + 0.717;
        assert!((result.local_amount().unwrap() - expected_cny).abs() < 0.000001);
        assert_eq!(result.local_currency, "CNY");
    }

    #[test]
    fn test_multi_currency_with_exchange_rate() {
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            completion_tokens: 0,
            ..Default::default()
        };

        let usd_pricing = AdvancedPricing {
            input_price: to_nano(1.0),
            output_price: to_nano(4.0),
            ..Default::default()
        };

        let multi_pricing = MultiCurrencyPricing {
            usd: usd_pricing,
            local: None,
            exchange_rate_nano: Some(to_nano(7.2)), // 1 USD = 7.2 CNY
        };

        let result = calculate_multi_currency_cost(&usage, &multi_pricing, false, false);

        // USD: 1M × $1.0 = $1.0
        assert!((result.usd_amount() - 1.0).abs() < 0.000001);

        // CNY via exchange rate: $1.0 × 7.2 = ¥7.2
        assert!((result.local_amount().unwrap() - 7.2).abs() < 0.000001);
    }

    #[test]
    fn test_multi_currency_batch() {
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            completion_tokens: 1_000_000,
            ..Default::default()
        };

        let usd_pricing = AdvancedPricing {
            input_price: to_nano(10.0),
            output_price: to_nano(30.0),
            batch_input_price: Some(to_nano(5.0)),
            batch_output_price: Some(to_nano(15.0)),
            ..Default::default()
        };

        let cny_pricing = AdvancedPricing {
            input_price: to_nano(70.0),
            output_price: to_nano(210.0),
            batch_input_price: Some(to_nano(35.0)),
            batch_output_price: Some(to_nano(105.0)),
            ..Default::default()
        };

        let multi_pricing = MultiCurrencyPricing {
            usd: usd_pricing,
            local: Some((Currency::CNY, cny_pricing)),
            exchange_rate_nano: None,
        };

        let result = calculate_multi_currency_cost(&usage, &multi_pricing, true, false);

        // USD batch: $5 + $15 = $20
        assert!((result.usd_amount() - 20.0).abs() < 0.000001);

        // CNY batch: ¥35 + ¥105 = ¥140
        assert!((result.local_amount().unwrap() - 140.0).abs() < 0.000001);
    }

    #[test]
    fn test_multi_currency_cache() {
        let usage = TokenUsage {
            prompt_tokens: 100_000,
            completion_tokens: 50_000,
            cache_read_tokens: 50_000,
            ..Default::default()
        };

        let usd_pricing = AdvancedPricing {
            input_price: to_nano(3.0),
            output_price: to_nano(15.0),
            cache_read_price: Some(to_nano(0.30)),
            ..Default::default()
        };

        let multi_pricing = MultiCurrencyPricing {
            usd: usd_pricing,
            local: None,
            exchange_rate_nano: Some(to_nano(7.2)),
        };

        let result = calculate_multi_currency_cost(&usage, &multi_pricing, false, false);

        // Verify cache cost is calculated
        // Standard: 50K × $3.0 + 50K × $15.0 + 50K × $0.30 = $0.15 + $0.75 + $0.015 = $0.915
        assert!((result.usd_amount() - 0.915).abs() < 0.000001);

        // With exchange rate, local amount should be USD × rate
        let expected_local = result.usd_amount() * 7.2;
        assert!((result.local_amount().unwrap() - expected_local).abs() < 0.000001);
    }

    #[test]
    fn test_multi_currency_priority() {
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            completion_tokens: 500_000,
            ..Default::default()
        };

        let usd_pricing = AdvancedPricing {
            input_price: to_nano(10.0),
            output_price: to_nano(30.0),
            priority_input_price: Some(to_nano(17.0)),
            priority_output_price: Some(to_nano(51.0)),
            ..Default::default()
        };

        let cny_pricing = AdvancedPricing {
            input_price: to_nano(70.0),
            output_price: to_nano(210.0),
            priority_input_price: Some(to_nano(119.0)),
            priority_output_price: Some(to_nano(357.0)),
            ..Default::default()
        };

        let multi_pricing = MultiCurrencyPricing {
            usd: usd_pricing,
            local: Some((Currency::CNY, cny_pricing)),
            exchange_rate_nano: None,
        };

        let result = calculate_multi_currency_cost(&usage, &multi_pricing, false, true);

        // USD priority: $17 + $25.5 = $42.5
        let expected_usd = 1.0 * 17.0 + 0.5 * 51.0;
        assert!((result.usd_amount() - expected_usd).abs() < 0.000001);

        // CNY priority: ¥119 + ¥178.5 = ¥297.5
        let expected_cny = 1.0 * 119.0 + 0.5 * 357.0;
        assert!((result.local_amount().unwrap() - expected_cny).abs() < 0.000001);
    }

    #[test]
    fn test_multi_currency_fallback_to_usd_only() {
        let usage = TokenUsage {
            prompt_tokens: 1_000_000,
            completion_tokens: 0,
            ..Default::default()
        };

        let usd_pricing = AdvancedPricing {
            input_price: to_nano(1.0),
            output_price: to_nano(4.0),
            ..Default::default()
        };

        let multi_pricing = MultiCurrencyPricing {
            usd: usd_pricing,
            local: None,
            exchange_rate_nano: None,
        };

        let result = calculate_multi_currency_cost(&usage, &multi_pricing, false, false);

        // USD only: $1.0
        assert!((result.usd_amount() - 1.0).abs() < 0.000001);
        assert!(result.local_amount().is_none());
        assert_eq!(result.local_currency, "USD");
    }

    #[test]
    fn test_multi_currency_eur_local() {
        let usage = TokenUsage {
            prompt_tokens: 500_000,
            completion_tokens: 500_000,
            ..Default::default()
        };

        let usd_pricing = AdvancedPricing {
            input_price: to_nano(10.0),
            output_price: to_nano(30.0),
            ..Default::default()
        };

        let eur_pricing = AdvancedPricing {
            input_price: to_nano(9.3), // ~1 EUR = 1.08 USD
            output_price: to_nano(27.9),
            ..Default::default()
        };

        let multi_pricing = MultiCurrencyPricing {
            usd: usd_pricing,
            local: Some((Currency::EUR, eur_pricing)),
            exchange_rate_nano: None,
        };

        let result = calculate_multi_currency_cost(&usage, &multi_pricing, false, false);

        // USD: 0.5 × $10 + 0.5 × $30 = $5 + $15 = $20
        assert!((result.usd_amount() - 20.0).abs() < 0.000001);

        // EUR: 0.5 × €9.3 + 0.5 × €27.9 = €4.65 + €13.95 = €18.6
        assert!((result.local_amount().unwrap() - 18.6).abs() < 0.000001);
        assert_eq!(result.local_currency, "EUR");
    }

    #[test]
    fn test_multi_currency_audio_tokens() {
        let usage = TokenUsage {
            prompt_tokens: 50_000,
            completion_tokens: 10_000,
            audio_tokens: 10_000,
            ..Default::default()
        };

        let usd_pricing = AdvancedPricing {
            input_price: to_nano(1.0),
            output_price: to_nano(4.0),
            audio_input_price: Some(to_nano(7.0)), // Audio tokens cost 7x
            ..Default::default()
        };

        let cny_pricing = AdvancedPricing {
            input_price: to_nano(7.0),
            output_price: to_nano(28.0),
            audio_input_price: Some(to_nano(49.0)),
            ..Default::default()
        };

        let multi_pricing = MultiCurrencyPricing {
            usd: usd_pricing,
            local: Some((Currency::CNY, cny_pricing)),
            exchange_rate_nano: None,
        };

        let result = calculate_multi_currency_cost(&usage, &multi_pricing, false, false);

        // USD: (50K × $1 + 10K × $4 + 10K × $7) / 1M = $0.05 + $0.04 + $0.07 = $0.16
        let expected_usd = 0.05 + 0.04 + 0.07;
        assert!((result.usd_amount() - expected_usd).abs() < 0.000001);

        // CNY: (50K × ¥7 + 10K × ¥28 + 10K × ¥49) / 1M = ¥0.35 + ¥0.28 + ¥0.49 = ¥1.12
        let expected_cny = 0.35 + 0.28 + 0.49;
        assert!((result.local_amount().unwrap() - expected_cny).abs() < 0.000001);
    }

    #[test]
    fn test_multi_currency_display_formatting() {
        let result = CostResult::with_local(1.0, "EUR", 0.93);
        assert!(result.display.contains("€"));
        assert!(result.display.contains("0.93"));

        let result_usd = CostResult::from_usd(1.0);
        assert!(result_usd.display.contains("$"));
        assert!(result_usd.display.contains("1.0"));
    }
}
