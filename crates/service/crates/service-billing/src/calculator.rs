use crate::cache::PriceCache;
use crate::error::BillingError;
use crate::types::{CostBreakdown, CostResult, UnifiedUsage};
use burncloud_common::types::Price;
use std::collections::HashMap;

// === Pricing percentage constants ===
// These define default multipliers when model-specific prices aren't configured.
// All values are percentages (100 = standard rate).

/// Priority requests cost 170% of standard rate (70% surcharge)
const PRIORITY_SURCHARGE_PERCENT: i64 = 170;
/// Batch requests cost 50% of standard rate (50% discount)
const BATCH_DISCOUNT_PERCENT: i64 = 50;
/// Cache read tokens cost 10% of standard input rate
const CACHE_READ_DISCOUNT_PERCENT: i64 = 10;
/// Cache write tokens cost 125% of standard input rate (creation overhead)
const CACHE_WRITE_SURCHARGE_PERCENT: i64 = 125;
/// Audio input tokens cost 700% of standard input rate (7x multiplier)
const AUDIO_INPUT_SURCHARGE_PERCENT: i64 = 700;

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

    /// Check whether a price is configured for `model` (optionally in a specific `region`).
    /// Returns `Err(BillingError::PriceNotFound)` when the model is unknown.
    /// Call this before sending the upstream request.
    pub async fn preflight(&self, model: &str, region: Option<&str>) -> Result<(), BillingError> {
        if self.cache.get(model, region).await.is_some() {
            Ok(())
        } else {
            Err(BillingError::PriceNotFound(model.to_string()))
        }
    }

    /// Calculate cost for a completed request.
    ///
    /// `region` selects the region-specific price (e.g. `"international"`, `"cn"`).
    /// Pass `None` to use the universal price.
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
        region: Option<&str>,
    ) -> Result<CostResult, BillingError> {
        self.calculate_with_voice(
            model,
            usage,
            request_id,
            is_batch,
            is_priority,
            region,
            None,
        )
        .await
    }

    /// Calculate cost for a completed request with optional voice-specific pricing.
    ///
    /// `region` selects the region-specific price (e.g. `"international"`, `"cn"`).
    /// `voice_id` is used for TTS models that have per-voice pricing.
    /// If the voice ID is not found in the model's voices_pricing, falls back to audio_output_price.
    pub async fn calculate_with_voice(
        &self,
        model: &str,
        usage: &UnifiedUsage,
        request_id: &str,
        is_batch: bool,
        is_priority: bool,
        region: Option<&str>,
        voice_id: Option<&str>,
    ) -> Result<CostResult, BillingError> {
        let price = self
            .cache
            .get(model, region)
            .await
            .ok_or_else(|| BillingError::PriceNotFound(model.to_string()))?;

        let breakdown =
            compute_breakdown(usage, &price, request_id, is_batch, is_priority, voice_id);
        Ok(CostResult::from_breakdown(breakdown))
    }
}

/// Look up voice-specific price from the voices_pricing JSON string.
///
/// Returns the price per 1M tokens/chars in nanodollars, or None if:
/// - The voices_pricing field is empty
/// - The voice_id is not found
/// - JSON parsing fails
pub fn lookup_voice_price(voices_json: &Option<String>, voice_id: &str) -> Option<i64> {
    let json = voices_json.as_ref()?;
    let voices: HashMap<String, i64> = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(
                voice_id = %voice_id,
                error = %e,
                "Failed to parse voices_pricing JSON; falling back to default audio pricing"
            );
            return None;
        }
    };
    voices.get(voice_id).copied()
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
    voice_id: Option<&str>,
) -> CostBreakdown {
    // --- Embedding ---
    let embedding_cost = if usage.embedding_tokens > 0 {
        // Embedding requests: use embedding_price if set, else input_price
        let embedding_rate = price.embedding_price.unwrap_or(price.input_price);
        nano(
            usage.embedding_tokens,
            embedding_rate,
            request_id,
            "embedding",
        )
    } else {
        0
    };

    // --- Standard input / output ---
    let (effective_input_price, effective_output_price) = if is_priority {
        (
            price.priority_input_price.unwrap_or(saturating_mul_percent(
                price.input_price,
                PRIORITY_SURCHARGE_PERCENT,
            )),
            price
                .priority_output_price
                .unwrap_or(saturating_mul_percent(
                    price.output_price,
                    PRIORITY_SURCHARGE_PERCENT,
                )),
        )
    } else if is_batch {
        (
            price.batch_input_price.unwrap_or(saturating_mul_percent(
                price.input_price,
                BATCH_DISCOUNT_PERCENT,
            )),
            price.batch_output_price.unwrap_or(saturating_mul_percent(
                price.output_price,
                BATCH_DISCOUNT_PERCENT,
            )),
        )
    } else {
        (price.input_price, price.output_price)
    };

    let input_cost = nano(
        usage.input_tokens,
        effective_input_price,
        request_id,
        "input",
    );
    let output_cost = nano(
        usage.output_tokens,
        effective_output_price,
        request_id,
        "output",
    );

    // --- Reasoning tokens (billed at reasoning_price, or output rate as fallback) ---
    let reasoning_rate = price.reasoning_price.unwrap_or(effective_output_price);
    let reasoning_cost = nano(
        usage.reasoning_tokens,
        reasoning_rate,
        request_id,
        "reasoning",
    );

    // --- Cache tokens ---
    let cache_read_price = price
        .cache_read_input_price
        .unwrap_or(saturating_mul_percent(
            price.input_price,
            CACHE_READ_DISCOUNT_PERCENT,
        ));
    let cache_write_price = price
        .cache_creation_input_price
        .unwrap_or(saturating_mul_percent(
            price.input_price,
            CACHE_WRITE_SURCHARGE_PERCENT,
        ));
    let cache_cost = nano(
        usage.cache_read_tokens,
        cache_read_price,
        request_id,
        "cache_read",
    )
    .saturating_add(nano(
        usage.cache_write_tokens,
        cache_write_price,
        request_id,
        "cache_write",
    ));

    // --- Audio tokens ---
    let audio_input_price = price.audio_input_price.unwrap_or(saturating_mul_percent(
        price.input_price,
        AUDIO_INPUT_SURCHARGE_PERCENT,
    ));
    let audio_output_price = price.audio_output_price.unwrap_or(effective_output_price);
    let audio_cost = nano(
        usage.audio_input_tokens,
        audio_input_price,
        request_id,
        "audio_input",
    )
    .saturating_add(nano(
        usage.audio_output_tokens,
        audio_output_price,
        request_id,
        "audio_output",
    ));

    // --- Voice-specific cost (TTS) ---
    // If voice_id is provided and found in voices_pricing, use that rate
    // Otherwise fall back to audio_output_price for audio_output_tokens
    let voice_cost = if let Some(vid) = voice_id {
        if usage.audio_output_tokens > 0 {
            if let Some(voice_price) = lookup_voice_price(&price.voices_pricing, vid) {
                nano(usage.audio_output_tokens, voice_price, request_id, "voice")
            } else {
                // Voice not found in pricing, use default audio_output_price
                0 // Don't double-count; audio_cost already includes this
            }
        } else {
            0
        }
    } else {
        0
    };

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

    // Music: flat fee per request (music_price is nanodollars/request, not nanodollars/MTok)
    // Do NOT use nano() here — the price is already the full cost, no token division needed.
    let music_cost = price.music_price.unwrap_or(0);

    CostBreakdown {
        input_cost,
        output_cost,
        cache_cost,
        audio_cost,
        voice_cost,
        image_cost,
        video_cost,
        music_cost,
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
            music_price: None,
            source: None,
            region: None,
            context_window: None,
            max_output_tokens: None,
            supports_vision: None,
            supports_function_calling: None,
            synced_at: None,
            created_at: None,
            updated_at: None,
            voices_pricing: None,
            video_pricing: None,
            asr_pricing: None,
            realtime_pricing: None,
            model_type: None,
        }
    }

    #[test]
    fn test_standard_cost() {
        // input_price = 5000 nano/1M  →  100 tokens = 0.5 nano
        let price = make_price(5_000, 15_000);
        let usage = UnifiedUsage {
            input_tokens: 100,
            output_tokens: 50,
            ..Default::default()
        };
        let bd = compute_breakdown(&usage, &price, "req-1", false, false, None);
        // 100 * 5000 / 1_000_000 = 0 (integer), 50 * 15000 / 1_000_000 = 0
        // Need bigger numbers to get non-zero
        let usage2 = UnifiedUsage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            ..Default::default()
        };
        let bd2 = compute_breakdown(&usage2, &price, "req-1", false, false, None);
        assert_eq!(bd2.input_cost, 5_000);
        assert_eq!(bd2.output_cost, 15_000);
        assert_eq!(bd2.total(), 20_000);
        let _ = bd;
    }

    #[test]
    fn test_batch_discount() {
        let price = make_price(10_000, 30_000);
        let usage = UnifiedUsage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            ..Default::default()
        };
        let bd = compute_breakdown(&usage, &price, "req-1", true, false, None);
        // batch: 50% → input=5000, output=15000
        assert_eq!(bd.input_cost, 5_000);
        assert_eq!(bd.output_cost, 15_000);
    }

    #[test]
    fn test_priority_surcharge() {
        let price = make_price(10_000, 30_000);
        let usage = UnifiedUsage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            ..Default::default()
        };
        let bd = compute_breakdown(&usage, &price, "req-1", false, true, None);
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
        let bd = compute_breakdown(&usage, &price, "req-1", false, false, None);
        // cache_read = 1M * 1000 / 1M = 1000; cache_write = 1M * 12500 / 1M = 12500
        assert_eq!(bd.cache_cost, 1_000 + 12_500);
    }

    #[test]
    fn test_embedding_cost() {
        let price = make_price(5_000, 0);
        let usage = UnifiedUsage {
            embedding_tokens: 1_000_000,
            ..Default::default()
        };
        let bd = compute_breakdown(&usage, &price, "req-1", false, false, None);
        assert_eq!(bd.embedding_cost, 5_000);
        assert_eq!(bd.input_cost, 0); // no double-count
    }

    #[test]
    fn test_overflow_truncation() {
        // Pathological: i64::MAX tokens * very large price
        let result = nano(i64::MAX, i64::MAX, "req-test", "input");
        assert_eq!(result, i64::MAX);
    }

    #[test]
    fn test_music_cost_flat_fee() {
        // music_price = 80_000_000 nano (= $0.08/request)
        // music_cost must be exactly 80_000_000 — no token division
        let mut price = make_price(5_000, 15_000);
        price.music_price = Some(80_000_000);
        let usage = UnifiedUsage {
            ..Default::default()
        };
        let bd = compute_breakdown(&usage, &price, "req-music", false, false, None);
        assert_eq!(
            bd.music_cost, 80_000_000,
            "music is flat fee, not divided by 1M tokens"
        );
    }

    #[test]
    fn test_music_cost_none() {
        // music_price = None → music_cost must be 0, no panic
        let price = make_price(5_000, 15_000);
        let usage = UnifiedUsage {
            ..Default::default()
        };
        let bd = compute_breakdown(&usage, &price, "req-no-music", false, false, None);
        assert_eq!(bd.music_cost, 0);
    }

    #[tokio::test]
    async fn test_preflight_not_found() {
        let cache = PriceCache::empty();
        let calc = CostCalculator::new(cache);
        let err = calc.preflight("nonexistent-model", None).await.unwrap_err();
        assert!(matches!(err, BillingError::PriceNotFound(m) if m == "nonexistent-model"));
    }

    #[tokio::test]
    async fn test_preflight_found() {
        use burncloud_common::types::Price;
        let cache = PriceCache::empty();
        let price = make_price(5_000, 15_000);
        {
            let mut guard = cache.inner.write().await;
            guard.insert(("gpt-4o".to_string(), String::new()), price);
        }
        let calc = CostCalculator::new(cache);
        assert!(calc.preflight("gpt-4o", None).await.is_ok());
        assert!(calc.preflight("GPT-4O", None).await.is_ok()); // case-insensitive
    }
}
