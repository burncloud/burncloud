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

    /// Accumulate another usage into self using saturating add.
    /// Overflow is capped at i64::MAX and a warning should be logged by the caller.
    pub fn saturating_add(&mut self, other: &UnifiedUsage) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.cache_read_tokens = self.cache_read_tokens.saturating_add(other.cache_read_tokens);
        self.cache_write_tokens = self.cache_write_tokens.saturating_add(other.cache_write_tokens);
        self.audio_input_tokens = self.audio_input_tokens.saturating_add(other.audio_input_tokens);
        self.audio_output_tokens =
            self.audio_output_tokens.saturating_add(other.audio_output_tokens);
        self.image_tokens = self.image_tokens.saturating_add(other.image_tokens);
        self.video_tokens = self.video_tokens.saturating_add(other.video_tokens);
        self.reasoning_tokens = self.reasoning_tokens.saturating_add(other.reasoning_tokens);
        self.embedding_tokens = self.embedding_tokens.saturating_add(other.embedding_tokens);
    }
}

/// Per-token-type cost breakdown, all in nanodollars.
/// Formula: cost_nano = tokens * price_per_million / 1_000_000
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostBreakdown {
    pub input_cost: i64,
    pub output_cost: i64,
    pub cache_cost: i64,
    pub audio_cost: i64,
    pub voice_cost: i64,
    pub image_cost: i64,
    pub video_cost: i64,
    pub music_cost: i64,
    pub reasoning_cost: i64,
    pub embedding_cost: i64,
}

impl CostBreakdown {
    /// Sum all components into a total, capping at i64::MAX on overflow.
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
        self.display = format!("{}{:.6}", currency_symbol(currency), local_nano as f64 / 1_000_000_000.0);
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
