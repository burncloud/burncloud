use crate::cache::PriceCache;
use crate::error::BillingError;
use crate::types::{CostBreakdown, CostResult, UnifiedUsage};
use burncloud_common::types::Price;

/// Calculates request costs using the in-memory [`PriceCache`].
///
/// # Preflight check
/// Call [`CostCalculator::preflight`] **before** forwarding the upstream request.
/// If the model price is not configured, return 503 immediately.
/// This prevents billing gaps from requests that already started streaming.
///
/// # Cost formula
/// `cost_nano = tokens * price_per_million / 1_000_000`
/// All intermediate arithmetic uses i128 to avoid overflow.
#[derive(Clone)]
pub struct CostCalculator {
    cache: PriceCache,
}

impl CostCalculator {
    pub fn new(cache: PriceCache) -> Self {
        Self { cache }
    }

    /// Check whether a price is configured for `model`.
    /// Returns `Err(BillingError::PriceNotFound)` when the model is unknown.
    /// Call this before sending the upstream request.
    pub async fn preflight(&self, model: &str) -> Result<(), BillingError> {
        if self.cache.get(model).await.is_some() {
            Ok(())
        } else {
            Err(BillingError::PriceNotFound(model.to_string()))
        }
    }

    /// Calculate cost for a completed request.
    ///
    /// Returns `Err(BillingError::PriceNotFound)` when the model has no price entry.
    /// i64 overflow is truncated to `i64::MAX` with a `tracing::warn!`.
    pub async fn calculate(
        &self,
        model: &str,
        usage: &UnifiedUsage,
        request_id: &str,
        is_batch: bool,
        is_priority: bool,
    ) -> Result<CostResult, BillingError> {
        let price = self
            .cache
            .get(model)
            .await
            .ok_or_else(|| BillingError::PriceNotFound(model.to_string()))?;

        let breakdown = compute_breakdown(usage, &price, request_id, is_batch, is_priority);
        Ok(CostResult::from_breakdown(breakdown))
    }
}

/// Compute a [`CostBreakdown`] from usage and a [`Price`] entry.
///
/// Priority order (matches existing billing.rs logic):
///   embedding > cache/audio/image/video > priority > batch > standard
fn compute_breakdown(
    usage: &UnifiedUsage,
    price: &Price,
    request_id: &str,
    is_batch: bool,
    is_priority: bool,
) -> CostBreakdown {
    // --- Embedding ---
    let embedding_cost = if usage.embedding_tokens > 0 {
        // Embedding requests: use embedding_price if set, else input_price
        let embedding_rate = price.embedding_price.unwrap_or(price.input_price);
        nano(usage.embedding_tokens, embedding_rate, request_id, "embedding")
    } else {
        0
    };

    // --- Standard input / output ---
    let (effective_input_price, effective_output_price) = if is_priority {
        (
            price.priority_input_price.unwrap_or(
                // default: 170% of standard
                saturating_mul_percent(price.input_price, 170),
            ),
            price.priority_output_price.unwrap_or(
                saturating_mul_percent(price.output_price, 170),
            ),
        )
    } else if is_batch {
        (
            price.batch_input_price.unwrap_or(
                // default: 50% of standard
                saturating_mul_percent(price.input_price, 50),
            ),
            price.batch_output_price.unwrap_or(
                saturating_mul_percent(price.output_price, 50),
            ),
        )
    } else {
        (price.input_price, price.output_price)
    };

    let input_cost = nano(usage.input_tokens, effective_input_price, request_id, "input");
    let output_cost = nano(usage.output_tokens, effective_output_price, request_id, "output");

    // --- Reasoning tokens (billed at reasoning_price, or output rate as fallback) ---
    let reasoning_rate = price.reasoning_price.unwrap_or(effective_output_price);
    let reasoning_cost = nano(usage.reasoning_tokens, reasoning_rate, request_id, "reasoning");

    // --- Cache tokens ---
    let cache_read_price = price.cache_read_input_price.unwrap_or(
        // default: 10% of standard input price
        saturating_mul_percent(price.input_price, 10),
    );
    let cache_write_price = price.cache_creation_input_price.unwrap_or(
        // default: 125% of standard input price
        saturating_mul_percent(price.input_price, 125),
    );
    let cache_cost = nano(usage.cache_read_tokens, cache_read_price, request_id, "cache_read")
        .saturating_add(nano(usage.cache_write_tokens, cache_write_price, request_id, "cache_write"));

    // --- Audio tokens ---
    let audio_input_price = price.audio_input_price.unwrap_or(
        // default: 7x standard input price
        saturating_mul_percent(price.input_price, 700),
    );
    let audio_output_price = price.audio_output_price.unwrap_or(effective_output_price);
    let audio_cost = nano(usage.audio_input_tokens, audio_input_price, request_id, "audio_input")
        .saturating_add(nano(usage.audio_output_tokens, audio_output_price, request_id, "audio_output"));

    // --- Image / video tokens ---
    let image_cost = if usage.image_tokens > 0 {
        let image_price = price.image_price.unwrap_or(price.input_price);
        nano(usage.image_tokens, image_price, request_id, "image")
    } else {
        0
    };
    let video_cost = if usage.video_tokens > 0 {
        let video_price = price.video_price.unwrap_or(price.input_price);
        nano(usage.video_tokens, video_price, request_id, "video")
    } else {
        0
    };

    CostBreakdown {
        input_cost,
        output_cost,
        cache_cost,
        audio_cost,
        image_cost,
        video_cost,
        reasoning_cost,
        embedding_cost,
    }
}

/// Compute `tokens * price_per_million / 1_000_000` in nanodollars.
/// Uses i128 intermediates to prevent overflow; caps at i64::MAX with a warn.
fn nano(tokens: i64, price_per_million: i64, request_id: &str, field: &str) -> i64 {
    if tokens <= 0 || price_per_million <= 0 {
        return 0;
    }
    let result = tokens as i128 * price_per_million as i128 / 1_000_000;
    if result > i64::MAX as i128 {
        tracing::warn!(
            request_id = %request_id,
            field = %field,
            "i64 overflow in cost calculation; truncating to i64::MAX"
        );
        i64::MAX
    } else {
        result as i64
    }
}

/// Returns `price * percent / 100`, capped at i64::MAX.
fn saturating_mul_percent(price: i64, percent: i64) -> i64 {
    let result = price as i128 * percent as i128 / 100;
    result.min(i64::MAX as i128) as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_common::types::Price;

    fn make_price(input: i64, output: i64) -> Price {
        Price {
            id: 1,
            model: "test-model".to_string(),
            currency: "USD".to_string(),
            input_price: input,
            output_price: output,
            cache_read_input_price: None,
            cache_creation_input_price: None,
            batch_input_price: None,
            batch_output_price: None,
            priority_input_price: None,
            priority_output_price: None,
            audio_input_price: None,
            audio_output_price: None,
            reasoning_price: None,
            embedding_price: None,
            image_price: None,
            video_price: None,
            source: None,
            region: None,
            context_window: None,
            max_output_tokens: None,
            supports_vision: None,
            supports_function_calling: None,
            synced_at: None,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn test_standard_cost() {
        // input_price = 5000 nano/1M  →  100 tokens = 0.5 nano
        let price = make_price(5_000, 15_000);
        let usage = UnifiedUsage { input_tokens: 100, output_tokens: 50, ..Default::default() };
        let bd = compute_breakdown(&usage, &price, "req-1", false, false);
        // 100 * 5000 / 1_000_000 = 0 (integer), 50 * 15000 / 1_000_000 = 0
        // Need bigger numbers to get non-zero
        let usage2 = UnifiedUsage { input_tokens: 1_000_000, output_tokens: 1_000_000, ..Default::default() };
        let bd2 = compute_breakdown(&usage2, &price, "req-1", false, false);
        assert_eq!(bd2.input_cost, 5_000);
        assert_eq!(bd2.output_cost, 15_000);
        assert_eq!(bd2.total(), 20_000);
        let _ = bd;
    }

    #[test]
    fn test_batch_discount() {
        let price = make_price(10_000, 30_000);
        let usage = UnifiedUsage { input_tokens: 1_000_000, output_tokens: 1_000_000, ..Default::default() };
        let bd = compute_breakdown(&usage, &price, "req-1", true, false);
        // batch: 50% → input=5000, output=15000
        assert_eq!(bd.input_cost, 5_000);
        assert_eq!(bd.output_cost, 15_000);
    }

    #[test]
    fn test_priority_surcharge() {
        let price = make_price(10_000, 30_000);
        let usage = UnifiedUsage { input_tokens: 1_000_000, output_tokens: 1_000_000, ..Default::default() };
        let bd = compute_breakdown(&usage, &price, "req-1", false, true);
        // priority: 170% → input=17000, output=51000
        assert_eq!(bd.input_cost, 17_000);
        assert_eq!(bd.output_cost, 51_000);
    }

    #[test]
    fn test_cache_defaults() {
        // cache_read defaults to 10% of input_price
        let price = make_price(10_000, 30_000);
        let usage = UnifiedUsage {
            input_tokens: 1_000_000,
            cache_read_tokens: 1_000_000,
            cache_write_tokens: 1_000_000,
            ..Default::default()
        };
        let bd = compute_breakdown(&usage, &price, "req-1", false, false);
        // cache_read = 1M * 1000 / 1M = 1000; cache_write = 1M * 12500 / 1M = 12500
        assert_eq!(bd.cache_cost, 1_000 + 12_500);
    }

    #[test]
    fn test_embedding_cost() {
        let price = make_price(5_000, 0);
        let usage = UnifiedUsage { embedding_tokens: 1_000_000, ..Default::default() };
        let bd = compute_breakdown(&usage, &price, "req-1", false, false);
        assert_eq!(bd.embedding_cost, 5_000);
        assert_eq!(bd.input_cost, 0); // no double-count
    }

    #[test]
    fn test_overflow_truncation() {
        // Pathological: i64::MAX tokens * very large price
        let result = nano(i64::MAX, i64::MAX, "req-test", "input");
        assert_eq!(result, i64::MAX);
    }

    #[tokio::test]
    async fn test_preflight_not_found() {
        let cache = PriceCache::empty();
        let calc = CostCalculator::new(cache);
        let err = calc.preflight("nonexistent-model").await.unwrap_err();
        assert!(matches!(err, BillingError::PriceNotFound(m) if m == "nonexistent-model"));
    }

    #[tokio::test]
    async fn test_preflight_found() {
        use burncloud_common::types::Price;
        let cache = PriceCache::empty();
        let price = make_price(5_000, 15_000);
        {
            let mut guard = cache.inner.write().await;
            guard.insert("gpt-4o".to_string(), price);
        }
        let calc = CostCalculator::new(cache);
        assert!(calc.preflight("gpt-4o").await.is_ok());
        assert!(calc.preflight("GPT-4O").await.is_ok()); // case-insensitive
    }
}
