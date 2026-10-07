use serde::{Deserialize, Serialize};

/// Unified token usage — covers all mainstream providers and modalities.
/// All counts stored as i64 (DB-compatible, sufficient for any single request).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct UnifiedUsage {
    /// Standard text input tokens
    pub input_tokens: i64,
    /// Standard text output tokens
    pub output_tokens: i64,
    /// Cache read tokens (Prompt Caching — billed at ~10% of input price)
    pub cache_read_tokens: i64,
    /// Cache write tokens (Prompt Caching — billed at ~125% of input price)
    pub cache_write_tokens: i64,
    /// Audio input tokens (e.g., Whisper / GPT-4o audio)
    pub audio_input_tokens: i64,
    /// Audio output tokens (e.g., TTS)
    pub audio_output_tokens: i64,
    /// Image tokens (vision input)
    pub image_tokens: i64,
    /// Video tokens
    pub video_tokens: i64,
    /// Reasoning tokens (o1 / DeepSeek-R1 — billed at output rate)
    pub reasoning_tokens: i64,
    /// Embedding tokens (embedding requests; input_tokens = 0 for these)
    pub embedding_tokens: i64,
}

impl UnifiedUsage {
    /// True when all fields are zero
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// Sum of all token counts (for budget commit actual-TPM).
    pub fn total_tokens(&self) -> i64 {
        self.input_tokens
            .saturating_add(self.output_tokens)
            .saturating_add(self.cache_read_tokens)
            .saturating_add(self.cache_write_tokens)
            .saturating_add(self.audio_input_tokens)
            .saturating_add(self.audio_output_tokens)
            .saturating_add(self.image_tokens)
            .saturating_add(self.video_tokens)
            .saturating_add(self.reasoning_tokens)
            .saturating_add(self.embedding_tokens)
    }

    /// Accumulate another usage into self using saturating add.
    /// Overflow is capped at i64::MAX and a warning should be logged by the caller.
    pub fn saturating_add(&mut self, other: &UnifiedUsage) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.cache_read_tokens = self
            .cache_read_tokens
            .saturating_add(other.cache_read_tokens);
        self.cache_write_tokens = self
            .cache_write_tokens
            .saturating_add(other.cache_write_tokens);
        self.audio_input_tokens = self
            .audio_input_tokens
            .saturating_add(other.audio_input_tokens);
        self.audio_output_tokens = self
            .audio_output_tokens
            .saturating_add(other.audio_output_tokens);
        self.image_tokens = self.image_tokens.saturating_add(other.image_tokens);
        self.video_tokens = self.video_tokens.saturating_add(other.video_tokens);
        self.reasoning_tokens = self.reasoning_tokens.saturating_add(other.reasoning_tokens);
        self.embedding_tokens = self.embedding_tokens.saturating_add(other.embedding_tokens);
    }
}

/// Per-token-type cost breakdown, all in nanodollars.
/// Formula: cost_nano = tokens * price_per_million / 1_000_000
///
/// # Cache costs are split (#618)
///
/// `cache_cost` used to be the only cache field, and it held read **plus** write
/// cost merged. Anything that wanted to attribute cache spend read that single
/// number and could not tell the two apart — and the router's
/// `router_logs.cache_read_cost` column was filled with the merged value while
/// `cache_write_cost` stayed 0, so cache *writes* (the more expensive side) were
/// invisible in every per-column report.
///
/// The breakdown now carries [`Self::cache_read_cost`] and
/// [`Self::cache_write_cost`] separately, with [`Self::cache_cost`] retained as
/// their sum. New values satisfy:
///
/// ```text
/// cache_cost == cache_read_cost + cache_write_cost
/// ```
///
/// Old serialized values only contain `cache_cost`, so the split fields
/// deserialize with zero defaults. `cache_cost` remains the authoritative
/// cache component used by [`Self::total`]; the split fields are attribution
/// detail. The calculator maintains the invariant that new values satisfy
/// `cache_cost == cache_read_cost + cache_write_cost`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub input_cost: i64,
    pub output_cost: i64,
    /// Sum of [`Self::cache_read_cost`] and [`Self::cache_write_cost`].
    ///
    /// Retained because it is part of the published Commerce shape; prefer the
    /// two split fields when attributing cache spend.
    pub cache_cost: i64,
    /// Cost of `usage.cache_read_tokens` at the cache-read rate.
    #[serde(default)]
    pub cache_read_cost: i64,
    /// Cost of `usage.cache_write_tokens` at the cache-write rate.
    #[serde(default)]
    pub cache_write_cost: i64,
    pub audio_cost: i64,
    pub voice_cost: i64,
    pub image_cost: i64,
    pub video_cost: i64,
    pub music_cost: i64,
    pub reasoning_cost: i64,
    pub embedding_cost: i64,
}

impl CostBreakdown {
    /// Recompute the derived [`Self::cache_cost`] sum from the split fields.
    ///
    /// Call after mutating `cache_read_cost` / `cache_write_cost` so the
    /// published field stays consistent. Saturates rather than wrapping.
    pub fn recompute_cache_cost(&mut self) {
        self.cache_cost = self.cache_read_cost.saturating_add(self.cache_write_cost);
    }

    /// Sum all components into a total, capping at i64::MAX on overflow.
    ///
    /// `cache_cost` stays authoritative for total billing compatibility;
    /// `cache_read_cost` and `cache_write_cost` are attribution fields and
    /// are not added again here.
    pub fn total(&self) -> i64 {
        let total = self.input_cost as i128
            + self.output_cost as i128
            + self.cache_cost as i128
            + self.audio_cost as i128
            + self.voice_cost as i128
            + self.image_cost as i128
            + self.video_cost as i128
            + self.music_cost as i128
            + self.reasoning_cost as i128
            + self.embedding_cost as i128;
        total.min(i64::MAX as i128) as i64
    }
}

/// Final cost result including breakdown and multi-currency display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostResult {
    /// Total cost in USD nanodollars
    pub usd_amount_nano: i64,
    /// Detailed breakdown per token type
    pub breakdown: CostBreakdown,
    /// Local currency code (e.g., "CNY") — "USD" when no conversion
    pub local_currency: String,
    /// Cost in local currency nanodollars (None when currency = USD)
    pub local_amount_nano: Option<i64>,
    /// Human-readable display string (e.g., "$0.001234")
    pub display: String,
}

impl CostResult {
    pub fn from_breakdown(breakdown: CostBreakdown) -> Self {
        let total = breakdown.total();
        Self {
            usd_amount_nano: total,
            breakdown,
            local_currency: "USD".to_string(),
            local_amount_nano: None,
            display: format!("${:.6}", total as f64 / 1_000_000_000.0),
        }
    }

    pub fn with_local_currency(mut self, currency: &str, local_nano: i64) -> Self {
        self.local_currency = currency.to_string();
        self.local_amount_nano = Some(local_nano);
        self.display = format!(
            "{}{:.6}",
            currency_symbol(currency),
            local_nano as f64 / 1_000_000_000.0
        );
        self
    }
}

fn currency_symbol(code: &str) -> &'static str {
    match code.to_uppercase().as_str() {
        "USD" => "$",
        "CNY" => "¥",
        "EUR" => "€",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_cost_breakdown_json_keeps_merged_cache_cost() {
        let json = r#"{
            "input_cost": 10,
            "output_cost": 20,
            "cache_cost": 30,
            "audio_cost": 0,
            "voice_cost": 0,
            "image_cost": 0,
            "video_cost": 0,
            "music_cost": 0,
            "reasoning_cost": 0,
            "embedding_cost": 0
        }"#;

        let breakdown: CostBreakdown =
            serde_json::from_str(json).unwrap_or_else(|e| panic!("legacy JSON must parse: {e}"));
        assert_eq!(breakdown.cache_read_cost, 0);
        assert_eq!(breakdown.cache_write_cost, 0);
        assert_eq!(
            breakdown.total(),
            60,
            "legacy cache_cost must remain part of the total"
        );
    }

    #[test]
    fn total_keeps_cache_cost_as_the_authoritative_compatibility_field() {
        let breakdown = CostBreakdown {
            input_cost: 10,
            output_cost: 20,
            cache_cost: 999,
            cache_read_cost: 12,
            cache_write_cost: 18,
            ..Default::default()
        };

        assert_eq!(
            breakdown.total(),
            1_029,
            "split cache fields are attribution detail; total billing keeps the published cache_cost semantic"
        );
    }
}
