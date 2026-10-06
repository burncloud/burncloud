// Commerce-owned value objects for pricing and the pricing.json schema.
//
// Moved here by S1-B from `burncloud_common` (`types.rs` price types and the whole
// `pricing_config` module). Field names, serde attributes, nanodollar units and the JSON
// shapes are unchanged, so the legacy `burncloud_common` paths can keep re-exporting these
// items and every stored price document and billing row still decodes.
//
// This module is a pure contract: it knows neither a database framework nor logging. The
// persistence representation of the billing tables (the row structs, their SQL row mapping
// and the conversions into these domain types) lives in `burncloud-database-billing::rows`,
// which is the adapter that knows the database.
//
// [`ModelMetadata`] stays in this module by decision, not by omission. S0 assigns
// "model-capability truth" to Supply (see `docs/architecture-s0-baseline.md`), and S1-C was
// originally expected to move this type there. Investigating the target before moving it showed
// that Supply has no capability type to move it into: `burncloud-database-model` calls itself the
// canonical home of the `model_capabilities` table, but its row type `ModelInfo` describes
// HuggingFace repository fields (downloads, likes, tags) that the table does not have, and its CRUD
// methods are stubs. Meanwhile this type is the live one: the pricing document carries it, the CLI
// reads and writes it, and Traffic's price sync reads the capability columns off it when it writes
// that table.
//
// Moving a live type into a crate whose claim to own it is not backed by a matching type or a real
// implementation would make the dependency graph look tidier while making the code harder to
// follow. The honest resolution is recorded in #621: give the capability truth a real Supply type
// and real persistence, and move this one only then. Until that happens this type is a pricing
// document DTO, which is what S0 already describes it as ("本类型随价格文档迁移").
//
// `serde_json::Value` appears in [`FullPricing::additional_fields`] only: the full-pricing
// blob is open-ended pricing metadata by design, so the workspace-wide typed-struct rule
// cannot be satisfied here.
#![allow(clippy::disallowed_types)]

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

// Currency conversion helpers stay in the sibling `price_u64` module. They are re-exported
// (not privately imported) because the serde helper modules below reach them through
// `super::` and because `burncloud_common::types` used to re-export these alias names.
pub use crate::price_u64::{dollars_to_nano, nano_to_dollars};
pub use crate::price_u64::{
    dollars_to_nano as dollars_to_nanodollars, nano_to_dollars as nanodollars_to_dollars,
    NANO_PER_DOLLAR as NANODOLLAR_SCALE,
};

/// Supported currencies for pricing
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Currency {
    #[default]
    USD,
    CNY,
    EUR,
}

impl Currency {
    /// Get the currency symbol
    pub fn symbol(&self) -> &'static str {
        match self {
            Currency::USD => "$",
            Currency::CNY => "Â¥",
            Currency::EUR => "â‚¬",
        }
    }

    /// Get the currency code
    pub fn code(&self) -> &'static str {
        match self {
            Currency::USD => "USD",
            Currency::CNY => "CNY",
            Currency::EUR => "EUR",
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl FromStr for Currency {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "USD" | "usd" => Ok(Currency::USD),
            "CNY" | "cny" => Ok(Currency::CNY),
            "EUR" | "eur" => Ok(Currency::EUR),
            _ => Err(format!("Unknown currency: {}", s)),
        }
    }
}

/// Multi-currency price information
/// Prices are stored as i64 nanodollars (9 decimal precision)
/// Note: Using i64 instead of u64 for PostgreSQL BIGINT compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCurrencyPrice {
    /// Currency of the price
    pub currency: Currency,
    /// Input price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub input_price: i64,
    /// Output price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub output_price: i64,
}

/// Exchange rate for currency conversion
/// Rate is stored as scaled i64 (rate * 10^9) for precision
/// Note: Using i64 instead of u64 for PostgreSQL BIGINT compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExchangeRate {
    pub id: i32,
    pub from_currency: String,
    pub to_currency: String,
    /// Exchange rate scaled by 10^9 (e.g., 7.24 CNY/USD = 7_240_000_000)
    pub rate: i64,
    pub updated_at: Option<i64>,
}

/// Spec-aligned alias: `billing_exchange_rates` row type.
pub type BillingExchangeRate = ExchangeRate;
/// Tiered pricing configuration for models with usage-based pricing tiers
/// (e.g., Qwen models with different prices based on context length)
/// Prices are stored as i64 nanodollars (9 decimal precision)
/// Note: Using i64 instead of u64 for PostgreSQL BIGINT compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredPrice {
    pub id: i32,
    /// Model name
    pub model: String,
    /// Region for pricing (e.g., "cn", "international", NULL for universal)
    pub region: Option<String>,
    /// Currency for this tier (e.g., "USD", "CNY")
    pub currency: Option<String>,
    /// Tier type (e.g., "context_length", "usage_volume")
    pub tier_type: Option<String>,
    /// Starting token count for this tier
    pub tier_start: i64,
    /// Ending token count for this tier (NULL means no upper limit)
    pub tier_end: Option<i64>,
    /// Input price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub input_price: i64,
    /// Output price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub output_price: i64,
}

/// Input for creating/updating a tiered price
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredPriceInput {
    pub model: String,
    pub region: Option<String>,
    /// Currency for this tier (e.g., "USD", "CNY")
    pub currency: Option<String>,
    /// Tier type (e.g., "context_length", "usage_volume")
    pub tier_type: Option<String>,
    pub tier_start: i64,
    pub tier_end: Option<i64>,
    /// Input price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub input_price: i64,
    /// Output price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub output_price: i64,
}

/// Full pricing configuration for extensibility
/// Used for the full_pricing JSON blob in prices table
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FullPricing {
    /// Additional pricing fields not covered by dedicated columns
    #[serde(default)]
    pub additional_fields: HashMap<String, Value>,
}

/// Multi-currency price entry for prices table
/// Supports USD, CNY, EUR and advanced pricing fields
/// All prices are stored in nanodollars (i64, 9 decimal precision)
/// Note: Using i64 for PostgreSQL BIGINT compatibility (BIGINT is signed)
/// Values must always be non-negative (>= 0)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Price {
    pub id: i32,
    /// Model name
    pub model: String,
    /// Currency (USD, CNY, EUR)
    pub currency: String,
    /// Input price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub input_price: i64,
    /// Output price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub output_price: i64,
    /// Cache read input price per 1M tokens in nanodollars (for Prompt Caching)
    pub cache_read_input_price: Option<i64>,
    /// Cache creation input price per 1M tokens in nanodollars
    pub cache_creation_input_price: Option<i64>,
    /// Batch input price per 1M tokens in nanodollars (typically 50% of standard)
    pub batch_input_price: Option<i64>,
    /// Batch output price per 1M tokens in nanodollars
    pub batch_output_price: Option<i64>,
    /// Priority input price per 1M tokens in nanodollars (typically 170% of standard)
    pub priority_input_price: Option<i64>,
    /// Priority output price per 1M tokens in nanodollars
    pub priority_output_price: Option<i64>,
    /// Audio input price per 1M tokens in nanodollars (typically 7x text)
    pub audio_input_price: Option<i64>,
    /// Audio output price per 1M tokens in nanodollars
    pub audio_output_price: Option<i64>,
    /// Reasoning token price per 1M tokens in nanodollars (defaults to output rate)
    pub reasoning_price: Option<i64>,
    /// Embedding token price per 1M tokens in nanodollars (defaults to input rate)
    pub embedding_price: Option<i64>,
    /// Image generation/input price per 1M tokens in nanodollars
    pub image_price: Option<i64>,
    /// Video generation/input price per 1M tokens in nanodollars
    pub video_price: Option<i64>,
    /// Music generation price per request in nanodollars.
    /// NOTE: unit is nanodollars/request, NOT nanodollars/MTok like other price fields.
    pub music_price: Option<i64>,
    /// Source of pricing data (litellm, manual, community, etc.)
    pub source: Option<String>,
    /// Region for pricing (cn, international, NULL for universal)
    pub region: Option<String>,
    /// Context window for the model
    pub context_window: Option<i64>,
    /// Maximum output tokens
    pub max_output_tokens: Option<i64>,
    /// Whether the model supports vision/image input (stored as INTEGER 0/1 in DB)
    pub supports_vision: Option<i32>,
    /// Whether the model supports function calling (stored as INTEGER 0/1 in DB)
    pub supports_function_calling: Option<i32>,
    /// Last sync timestamp
    pub synced_at: Option<i64>,
    /// Creation timestamp
    pub created_at: Option<i64>,
    /// Update timestamp
    pub updated_at: Option<i64>,
    /// TTS voices pricing: JSON {"alloy": 15000000000, "echo": 15000000000} (nanodollars/1M chars)
    pub voices_pricing: Option<String>,
    /// Video pricing: JSON {"720p": 50000000, "1080p": 100000000} (nanodollars/second)
    pub video_pricing: Option<String>,
    /// ASR pricing: JSON {"per_minute": 360000000} (nanodollars/minute)
    pub asr_pricing: Option<String>,
    /// Realtime API pricing: JSON {"audio_input": 32000000000, "audio_output": 64000000000} (nanodollars/1M tokens)
    pub realtime_pricing: Option<String>,
    /// Model type: chat, realtime, video, image, tts, asr
    pub model_type: Option<String>,
}

/// Spec-aligned alias: `billing_prices` row type.
pub type BillingPrice = Price;

/// Spec-aligned alias: `billing_tiered_prices` row type.
pub type BillingTieredPrice = TieredPrice;

/// Spec-aligned alias: `billing_tiered_prices` input type.
pub type BillingTieredPriceInput = TieredPriceInput;

/// Input for creating/updating a Price entry
/// All prices are in nanodollars (i64, 9 decimal precision)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceInput {
    pub model: String,
    pub currency: String,
    /// Input price per 1M tokens in nanodollars
    pub input_price: i64,
    /// Output price per 1M tokens in nanodollars
    pub output_price: i64,
    /// Cache read input price per 1M tokens in nanodollars
    pub cache_read_input_price: Option<i64>,
    /// Cache creation input price per 1M tokens in nanodollars
    pub cache_creation_input_price: Option<i64>,
    /// Batch input price per 1M tokens in nanodollars
    pub batch_input_price: Option<i64>,
    /// Batch output price per 1M tokens in nanodollars
    pub batch_output_price: Option<i64>,
    /// Priority input price per 1M tokens in nanodollars
    pub priority_input_price: Option<i64>,
    /// Priority output price per 1M tokens in nanodollars
    pub priority_output_price: Option<i64>,
    /// Audio input price per 1M tokens in nanodollars
    pub audio_input_price: Option<i64>,
    /// Audio output price per 1M tokens in nanodollars
    pub audio_output_price: Option<i64>,
    /// Reasoning token price per 1M tokens in nanodollars (defaults to output rate)
    pub reasoning_price: Option<i64>,
    /// Embedding token price per 1M tokens in nanodollars (defaults to input rate)
    pub embedding_price: Option<i64>,
    /// Image generation/input price per 1M tokens in nanodollars
    pub image_price: Option<i64>,
    /// Video generation/input price per 1M tokens in nanodollars
    pub video_price: Option<i64>,
    /// Music generation price per request in nanodollars.
    /// NOTE: unit is nanodollars/request, NOT nanodollars/MTok like other price fields.
    pub music_price: Option<i64>,
    pub source: Option<String>,
    pub region: Option<String>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub supports_vision: Option<bool>,
    pub supports_function_calling: Option<bool>,
    /// TTS voices pricing: JSON string with nanodollar prices per voice
    pub voices_pricing: Option<String>,
    /// Video pricing: JSON string with nanodollar prices per resolution
    pub video_pricing: Option<String>,
    /// ASR pricing: JSON string with nanodollar prices
    pub asr_pricing: Option<String>,
    /// Realtime API pricing: JSON string with nanodollar prices
    pub realtime_pricing: Option<String>,
    /// Model type: chat, realtime, video, image, tts, asr
    pub model_type: Option<String>,
}

impl Default for PriceInput {
    fn default() -> Self {
        Self {
            model: String::new(),
            currency: "USD".to_string(),
            input_price: 0,
            output_price: 0,
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
            voices_pricing: None,
            video_pricing: None,
            asr_pricing: None,
            realtime_pricing: None,
            model_type: None,
        }
    }
}

/// Spec-aligned alias: `billing_prices` input type.
pub type BillingPriceInput = PriceInput;

impl Price {
    /// Check if vision is supported
    pub fn supports_vision_bool(&self) -> Option<bool> {
        self.supports_vision.map(|v| v != 0)
    }

    /// Check if function calling is supported
    pub fn supports_function_calling_bool(&self) -> Option<bool> {
        self.supports_function_calling.map(|v| v != 0)
    }
}
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod currency_tests {
    use super::*;

    #[test]
    fn test_currency_default() {
        let currency = Currency::default();
        assert_eq!(currency, Currency::USD);
    }

    #[test]
    fn test_currency_symbol() {
        assert_eq!(Currency::USD.symbol(), "$" );
        assert_eq!(Currency::CNY.symbol(), "Â¥");
        assert_eq!(Currency::EUR.symbol(), "â‚¬");
    }

    #[test]
    fn test_currency_code() {
        assert_eq!(Currency::USD.code(), "USD");
        assert_eq!(Currency::CNY.code(), "CNY");
        assert_eq!(Currency::EUR.code(), "EUR");
    }

    #[test]
    fn test_currency_display() {
        assert_eq!(format!("{}", Currency::USD), "USD");
        assert_eq!(format!("{}", Currency::CNY), "CNY");
    }

    #[test]
    fn test_currency_from_str() {
        assert_eq!(
            Currency::from_str("USD").unwrap_or_else(|e| panic!("Failed to parse currency: {e}")),
            Currency::USD
        );
        assert_eq!(
            Currency::from_str("usd").unwrap_or_else(|e| panic!("Failed to parse currency: {e}")),
            Currency::USD
        );
        assert_eq!(
            Currency::from_str("CNY").unwrap_or_else(|e| panic!("Failed to parse currency: {e}")),
            Currency::CNY
        );
        assert_eq!(
            Currency::from_str("eur").unwrap_or_else(|e| panic!("Failed to parse currency: {e}")),
            Currency::EUR
        );
        assert!(Currency::from_str("GBP").is_err());
    }

    #[test]
    fn test_currency_serde() {
        let json = serde_json::to_string(&Currency::USD)
            .unwrap_or_else(|e| panic!("Failed to serialize currency: {e}"));
        assert_eq!(json, "\"usd\"");

        let json = serde_json::to_string(&Currency::CNY)
            .unwrap_or_else(|e| panic!("Failed to serialize currency: {e}"));
        assert_eq!(json, "\"cny\"");

        let currency: Currency = serde_json::from_str("\"usd\"")
            .unwrap_or_else(|e| panic!("Failed to deserialize currency: {e}"));
        assert_eq!(currency, Currency::USD);

        let currency: Currency = serde_json::from_str("\"eur\"")
            .unwrap_or_else(|e| panic!("Failed to deserialize currency: {e}"));
        assert_eq!(currency, Currency::EUR);
    }

    #[test]
    fn test_multi_currency_price() {
        let price = MultiCurrencyPrice {
            currency: Currency::CNY,
            input_price: 2_000_000,
            output_price: 6_000_000,
        };

        assert_eq!(price.currency, Currency::CNY);
        assert_eq!(price.input_price, 2_000_000);
        assert_eq!(price.output_price, 6_000_000);

        let json = serde_json::to_string(&price)
            .unwrap_or_else(|e| panic!("Failed to serialize price: {e}"));
        assert!(json.contains("\"currency\":\"cny\""));
    }
}

mod nano_as_dollars {
    use super::{dollars_to_nano, nano_to_dollars};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let dollars = nano_to_dollars(*value);
        dollars.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dollars: f64 = Deserialize::deserialize(deserializer)?;
        Ok(dollars_to_nano(dollars))
    }
}

mod option_nano_as_dollars {
    use super::{dollars_to_nano, nano_to_dollars};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub(super) fn serialize<S>(value: &Option<i64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(nano) => Some(nano_to_dollars(*nano)).serialize(serializer),
            None => None::<f64>.serialize(serializer),
        }
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<Option<i64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt_dollars: Option<f64> = Option::deserialize(deserializer)?;
        Ok(opt_dollars.map(dollars_to_nano))
    }
}

mod nano_map_as_dollars {
    use super::{dollars_to_nano, nano_to_dollars};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::collections::HashMap;

    pub(super) fn serialize<S>(
        value: &HashMap<String, i64>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let dollars_map: HashMap<String, f64> = value
            .iter()
            .map(|(k, v)| (k.clone(), nano_to_dollars(*v)))
            .collect();
        dollars_map.serialize(serializer)
    }

    pub(super) fn deserialize<'de, D>(deserializer: D) -> Result<HashMap<String, i64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dollars_map: HashMap<String, f64> = HashMap::deserialize(deserializer)?;
        Ok(dollars_map
            .into_iter()
            .map(|(k, v)| (k, dollars_to_nano(v)))
            .collect())
    }
}

fn default_version() -> String {
    "1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingConfig {
    #[serde(default = "default_version")]
    pub version: String,
    pub updated_at: DateTime<Utc>,
    pub source: String,
    pub models: HashMap<String, ModelPricing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    #[serde(default)]
    pub pricing: HashMap<String, CurrencyPricing>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tiered_pricing: Option<HashMap<String, Vec<TieredPriceConfig>>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_pricing: Option<HashMap<String, CachePricingConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub batch_pricing: Option<HashMap<String, BatchPricingConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voices_pricing: Option<HashMap<String, VoicesPricingConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video_pricing: Option<HashMap<String, VideoPricingConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asr_pricing: Option<HashMap<String, ASRPricingConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub realtime_pricing: Option<HashMap<String, RealtimePricingConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ModelMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CurrencyPricing {
    #[serde(with = "nano_as_dollars")]
    pub input_price: i64,
    #[serde(with = "nano_as_dollars")]
    pub output_price: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(
        with = "option_nano_as_dollars",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub image_output_price: Option<i64>,
    #[serde(
        with = "option_nano_as_dollars",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_output_price: Option<i64>,
    #[serde(
        with = "option_nano_as_dollars",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub music_price: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieredPriceConfig {
    pub tier_start: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tier_end: Option<i64>,
    #[serde(with = "nano_as_dollars")]
    pub input_price: i64,
    #[serde(with = "nano_as_dollars")]
    pub output_price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachePricingConfig {
    #[serde(with = "nano_as_dollars")]
    pub cache_read_input_price: i64,
    #[serde(
        with = "option_nano_as_dollars",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub cache_creation_input_price: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchPricingConfig {
    #[serde(with = "nano_as_dollars")]
    pub batch_input_price: i64,
    #[serde(with = "nano_as_dollars")]
    pub batch_output_price: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoicesPricingConfig {
    #[serde(with = "nano_map_as_dollars")]
    #[serde(flatten)]
    pub voices: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPricingConfig {
    #[serde(with = "nano_map_as_dollars")]
    #[serde(flatten)]
    pub resolutions: HashMap<String, i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASRPricingConfig {
    #[serde(with = "nano_as_dollars")]
    pub per_minute: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealtimePricingConfig {
    #[serde(
        with = "option_nano_as_dollars",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_input: Option<i64>,
    #[serde(
        with = "option_nano_as_dollars",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub audio_output: Option<i64>,
    #[serde(
        with = "option_nano_as_dollars",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub image_input: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelMetadata {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<i64>,
    #[serde(default)]
    pub supports_vision: bool,
    #[serde(default)]
    pub supports_function_calling: bool,
    #[serde(default = "default_true")]
    pub supports_streaming: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_date: Option<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Deserialize)]
struct NewFormatTextPricing {
    #[serde(rename = "in", default)]
    input: f64,
    #[serde(rename = "out", default)]
    output: f64,
}

#[derive(Debug, Deserialize)]
struct NewFormatCachePricing {
    #[serde(rename = "read")]
    read: f64,
    #[serde(rename = "write", default)]
    write: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct NewFormatBatchPricing {
    #[serde(rename = "in")]
    input: f64,
    #[serde(rename = "out")]
    output: f64,
    #[serde(default)]
    #[allow(dead_code)]
    image_out: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct NewFormatImagePricing {
    #[serde(rename = "out")]
    output: f64,
}

#[derive(Debug, Deserialize)]
struct NewFormatAudioPricing {
    #[serde(rename = "out")]
    output: f64,
}

#[derive(Debug, Deserialize)]
struct NewFormatMusicPricing {
    #[serde(rename = "per")]
    per: f64,
}

#[derive(Debug, Deserialize)]
struct NewFormatVideoPricing {
    #[serde(rename = "sec", default)]
    #[allow(dead_code)]
    sec: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct NewFormatTier {
    tier_start: i64,
    #[serde(default)]
    tier_end: Option<i64>,
    #[serde(rename = "in")]
    input: f64,
    #[serde(rename = "out")]
    output: f64,
}

#[derive(Debug, Deserialize)]
struct NewFormatCurrencyBlock {
    #[serde(default)]
    text: Option<NewFormatTextPricing>,
    #[serde(default)]
    cache: Option<NewFormatCachePricing>,
    #[serde(default)]
    batch: Option<NewFormatBatchPricing>,
    #[serde(default)]
    image: Option<NewFormatImagePricing>,
    #[serde(default)]
    audio: Option<NewFormatAudioPricing>,
    #[serde(default)]
    #[allow(dead_code)]
    video: Option<NewFormatVideoPricing>,
    #[serde(default)]
    music: Option<NewFormatMusicPricing>,
    #[serde(default)]
    tiered: Option<Vec<NewFormatTier>>,
}

#[derive(Debug, Deserialize)]
struct NewFormatPricingConfig {
    version: String,
    updated_at: DateTime<Utc>,
    source: String,
    models: HashMap<String, HashMap<String, NewFormatCurrencyBlock>>,
}

impl From<NewFormatPricingConfig> for PricingConfig {
    fn from(new: NewFormatPricingConfig) -> Self {
        let mut models = HashMap::new();

        for (model_name, currencies) in new.models {
            let mut pricing: HashMap<String, CurrencyPricing> = HashMap::new();
            let mut cache_map: HashMap<String, CachePricingConfig> = HashMap::new();
            let mut batch_map: HashMap<String, BatchPricingConfig> = HashMap::new();
            let mut tiered_map: HashMap<String, Vec<TieredPriceConfig>> = HashMap::new();

            for (currency, block) in currencies {
                let text = match block.text {
                    Some(t) => t,
                    None => continue,
                };

                pricing.insert(
                    currency.clone(),
                    CurrencyPricing {
                        input_price: dollars_to_nano(text.input),
                        output_price: dollars_to_nano(text.output),
                        image_output_price: block.image.map(|img| dollars_to_nano(img.output)),
                        audio_output_price: block.audio.map(|aud| dollars_to_nano(aud.output)),
                        music_price: block.music.map(|m| dollars_to_nano(m.per)),
                        source: None,
                    },
                );

                if let Some(cache) = block.cache {
                    cache_map.insert(
                        currency.clone(),
                        CachePricingConfig {
                            cache_read_input_price: dollars_to_nano(cache.read),
                            cache_creation_input_price: cache.write.map(dollars_to_nano),
                        },
                    );
                }

                if let Some(batch) = block.batch {
                    batch_map.insert(
                        currency.clone(),
                        BatchPricingConfig {
                            batch_input_price: dollars_to_nano(batch.input),
                            batch_output_price: dollars_to_nano(batch.output),
                        },
                    );
                }

                if let Some(tiers) = block.tiered {
                    tiered_map.insert(
                        currency,
                        tiers
                            .into_iter()
                            .map(|t| TieredPriceConfig {
                                tier_start: t.tier_start,
                                tier_end: t.tier_end,
                                input_price: dollars_to_nano(t.input),
                                output_price: dollars_to_nano(t.output),
                            })
                            .collect(),
                    );
                }
            }

            models.insert(
                model_name,
                ModelPricing {
                    pricing,
                    tiered_pricing: (!tiered_map.is_empty()).then_some(tiered_map),
                    cache_pricing: (!cache_map.is_empty()).then_some(cache_map),
                    batch_pricing: (!batch_map.is_empty()).then_some(batch_map),
                    voices_pricing: None,
                    video_pricing: None,
                    asr_pricing: None,
                    realtime_pricing: None,
                    metadata: None,
                },
            );
        }

        PricingConfig {
            version: new.version,
            updated_at: new.updated_at,
            source: new.source,
            models,
        }
    }
}

fn version_major(version: &str) -> u32 {
    version
        .split('.')
        .next()
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModelLayout {
    NormalisedV1,
    FlatV7,
    Ambiguous,
}

/// Detect the layout from fields that are unique to each published shape.
///
/// `ModelPricing.pricing` has `#[serde(default)]`, so hand-written and historic
/// normalised documents may legally omit it. Every specialised v1 pricing field
/// therefore participates in detection; otherwise a tiered/media-only document
/// carrying version "7.0" would be sent to the flat parser and silently emptied.
fn detect_model_layout(value: &serde_json::Value) -> ModelLayout {
    const V1_ONLY_KEYS: [&str; 8] = [
        "pricing",
        "tiered_pricing",
        "cache_pricing",
        "batch_pricing",
        "voices_pricing",
        "video_pricing",
        "asr_pricing",
        "realtime_pricing",
    ];
    const V7_ONLY_KEYS: [&str; 8] = [
        "text", "cache", "batch", "image", "audio", "video", "music", "tiered",
    ];

    let Some(models) = value.get("models").and_then(|m| m.as_object()) else {
        return ModelLayout::Ambiguous;
    };

    for entry in models.values() {
        let Some(fields) = entry.as_object() else {
            continue;
        };
        if V1_ONLY_KEYS.iter().any(|key| fields.contains_key(*key)) {
            return ModelLayout::NormalisedV1;
        }
        for currency_block in fields.values() {
            let Some(block) = currency_block.as_object() else {
                continue;
            };
            if V7_ONLY_KEYS.iter().any(|key| block.contains_key(*key)) {
                return ModelLayout::FlatV7;
            }
        }
    }

    ModelLayout::Ambiguous
}

impl PricingConfig {
    pub fn new(source: &str) -> Self {
        Self {
            version: "1.0".to_string(),
            updated_at: Utc::now(),
            source: source.to_string(),
            models: HashMap::new(),
        }
    }

    /// Parse pricing configuration from JSON string.
    ///
    /// Explicit layout markers win over the version string. Version is only a
    /// tie-breaker for documents whose body carries no recognised pricing shape.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let version = value["version"]
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_else(default_version);
        let use_flat_v7 = match detect_model_layout(&value) {
            ModelLayout::NormalisedV1 => false,
            ModelLayout::FlatV7 => true,
            ModelLayout::Ambiguous => version_major(&version) >= 7,
        };
        if use_flat_v7 {
            let mut value = value;
            if value.get("version").is_none() {
                if let Some(obj) = value.as_object_mut() {
                    obj.insert(
                        "version".to_string(),
                        serde_json::Value::String(version.clone()),
                    );
                }
            }
            let new_cfg: NewFormatPricingConfig = serde_json::from_value(value)?;
            return Ok(PricingConfig::from(new_cfg));
        }
        serde_json::from_value(value)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn validate(&self) -> Result<Vec<ValidationWarning>, ValidationError> {
        let mut warnings = Vec::new();

        if !self.version.contains('.') {
            warnings.push(ValidationWarning {
                field: "version".to_string(),
                message: format!("Version '{}' should be in format 'X.Y'", self.version),
                suggestion: "Use semantic versioning like '1.0'".to_string(),
            });
        }

        for (model_name, model_pricing) in &self.models {
            let high_price_threshold: i64 = 1_000_000_000_000;

            for (currency, pricing) in &model_pricing.pricing {
                if pricing.input_price > high_price_threshold
                    || pricing.output_price > high_price_threshold
                {
                    warnings.push(ValidationWarning {
                        field: format!("models.{}.pricing.{}", model_name, currency),
                        message: format!(
                            "Price ${:.2}/1M seems unusually high",
                            nano_to_dollars(pricing.input_price.max(pricing.output_price))
                        ),
                        suggestion: "Verify the pricing is correct".to_string(),
                    });
                }
            }

            if let Some(ref tiered) = model_pricing.tiered_pricing {
                for (currency, tiers) in tiered {
                    for (i, tier) in tiers.iter().enumerate() {
                        if let Some(tier_end) = tier.tier_end {
                            if tier_end <= tier.tier_start {
                                return Err(ValidationError::InvalidTier {
                                    model: model_name.clone(),
                                    tier_index: i,
                                    message: format!(
                                        "tier_end ({}) must be greater than tier_start ({})",
                                        tier_end, tier.tier_start
                                    ),
                                });
                            }
                        }

                        if i > 0 {
                            let prev_tier = &tiers[i - 1];
                            if let Some(prev_end) = prev_tier.tier_end {
                                if prev_end != tier.tier_start {
                                    warnings.push(ValidationWarning {
                                        field: format!(
                                            "models.{}.tiered_pricing.{}.tier[{}]",
                                            model_name, currency, i
                                        ),
                                        message: format!(
                                            "Gap between tiers: previous ends at {}, current starts at {}",
                                            prev_end, tier.tier_start
                                        ),
                                        suggestion:
                                            "Tiers should be contiguous for accurate billing".to_string(),
                                    });
                                }
                            }
                        }
                    }
                }
            }

            if model_pricing.pricing.is_empty() {
                warnings.push(ValidationWarning {
                    field: format!("models.{}", model_name),
                    message: "Model has no pricing configured".to_string(),
                    suggestion: "Add pricing for at least one currency (USD recommended)"
                        .to_string(),
                });
            }
        }

        Ok(warnings)
    }

    pub fn get_pricing(&self, model: &str, currency: &str) -> Option<&CurrencyPricing> {
        self.models.get(model)?.pricing.get(currency)
    }

    pub fn get_tiered_pricing(
        &self,
        model: &str,
        currency: &str,
    ) -> Option<&Vec<TieredPriceConfig>> {
        self.models
            .get(model)?
            .tiered_pricing
            .as_ref()?
            .get(currency)
    }

    pub fn get_cache_pricing(&self, model: &str, currency: &str) -> Option<&CachePricingConfig> {
        self.models
            .get(model)?
            .cache_pricing
            .as_ref()?
            .get(currency)
    }

    pub fn list_models(&self) -> Vec<&String> {
        self.models.keys().collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationWarning {
    pub field: String,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum ValidationError {
    #[error("Invalid tier configuration in {model} at tier {tier_index}: {message}")]
    InvalidTier {
        model: String,
        tier_index: usize,
        message: String,
    },
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    fn to_nano(price: f64) -> i64 {
        dollars_to_nano(price)
    }

    #[test]
    fn test_pricing_config_creation() {
        let config = PricingConfig::new("test");
        assert_eq!(config.version, "1.0");
        assert_eq!(config.source, "test");
        assert!(config.models.is_empty());
    }

    #[test]
    fn test_pricing_config_json_roundtrip() {
        let mut config = PricingConfig::new("test");
        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: to_nano(10.0),
                output_price: to_nano(30.0),
                source: Some("openai".to_string()),
                ..Default::default()
            },
        );
        config.models.insert(
            "gpt-4-turbo".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: None,
                cache_pricing: None,
                batch_pricing: None,
                voices_pricing: None,
                video_pricing: None,
                asr_pricing: None,
                realtime_pricing: None,
                metadata: None,
            },
        );
        let parsed = PricingConfig::from_json(&config.to_json().unwrap()).unwrap();
        assert!(parsed.models.contains_key("gpt-4-turbo"));
    }

    #[test]
    fn specialised_v1_marker_beats_a_v7_version_even_without_pricing() {
        let tiered_only = r#"{
            "version":"7.0",
            "updated_at":"2026-03-29T00:00:00Z",
            "source":"test",
            "models":{"m":{"tiered_pricing":{"USD":[{
                "tier_start":0,"tier_end":1000,"input_price":3.0,"output_price":4.0
            }]}}}
        }"#;
        let tiered = PricingConfig::from_json(tiered_only)
            .unwrap_or_else(|e| panic!("tiered-only normalised v1 layout must parse: {e}"));
        assert_eq!(
            tiered.get_tiered_pricing("m", "USD").unwrap()[0].input_price,
            to_nano(3.0)
        );

        let video_only = r#"{
            "version":"7.0",
            "updated_at":"2026-03-29T00:00:00Z",
            "source":"test",
            "models":{"video-model":{"video_pricing":{"USD":{"1080p":0.10}}}}
        }"#;
        let video = PricingConfig::from_json(video_only)
            .unwrap_or_else(|e| panic!("video-only normalised v1 layout must parse: {e}"));
        assert!(video.models["video-model"].video_pricing.is_some());
    }

    #[test]
    fn test_pricing_config_validation() {
        let mut config = PricingConfig::new("test");
        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: to_nano(10.0),
                output_price: to_nano(30.0),
                source: None,
                ..Default::default()
            },
        );
        config.models.insert(
            "test-model".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: None,
                cache_pricing: None,
                batch_pricing: None,
                voices_pricing: None,
                video_pricing: None,
                asr_pricing: None,
                realtime_pricing: None,
                metadata: None,
            },
        );
        assert!(config.validate().unwrap().is_empty());
    }

    #[test]
    fn test_tiered_pricing_validation() {
        let mut config = PricingConfig::new("test");
        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: to_nano(10.0),
                output_price: to_nano(30.0),
                source: None,
                ..Default::default()
            },
        );
        let mut tiered_map = HashMap::new();
        tiered_map.insert(
            "USD".to_string(),
            vec![
                TieredPriceConfig {
                    tier_start: 0,
                    tier_end: Some(32000),
                    input_price: to_nano(1.2),
                    output_price: to_nano(6.0),
                },
                TieredPriceConfig {
                    tier_start: 32000,
                    tier_end: Some(128000),
                    input_price: to_nano(2.4),
                    output_price: to_nano(12.0),
                },
            ],
        );
        config.models.insert(
            "qwen-max".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: Some(tiered_map),
                cache_pricing: None,
                batch_pricing: None,
                voices_pricing: None,
                video_pricing: None,
                asr_pricing: None,
                realtime_pricing: None,
                metadata: None,
            },
        );
        assert!(config.validate().unwrap().is_empty());
    }

    #[test]
    fn test_invalid_tier_boundaries() {
        let mut config = PricingConfig::new("test");
        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: to_nano(10.0),
                output_price: to_nano(30.0),
                source: None,
                ..Default::default()
            },
        );
        let mut tiered_map = HashMap::new();
        tiered_map.insert(
            "USD".to_string(),
            vec![TieredPriceConfig {
                tier_start: 100,
                tier_end: Some(50),
                input_price: to_nano(1.2),
                output_price: to_nano(6.0),
            }],
        );
        config.models.insert(
            "invalid-model".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: Some(tiered_map),
                cache_pricing: None,
                batch_pricing: None,
                voices_pricing: None,
                video_pricing: None,
                asr_pricing: None,
                realtime_pricing: None,
                metadata: None,
            },
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_get_pricing_methods() {
        let mut config = PricingConfig::new("test");
        let mut pricing = HashMap::new();
        pricing.insert(
            "USD".to_string(),
            CurrencyPricing {
                input_price: to_nano(10.0),
                output_price: to_nano(30.0),
                source: None,
                ..Default::default()
            },
        );
        let mut cache_map = HashMap::new();
        cache_map.insert(
            "USD".to_string(),
            CachePricingConfig {
                cache_read_input_price: to_nano(1.0),
                cache_creation_input_price: Some(to_nano(1.25)),
            },
        );
        config.models.insert(
            "claude-3".to_string(),
            ModelPricing {
                pricing,
                tiered_pricing: None,
                cache_pricing: Some(cache_map),
                batch_pricing: None,
                voices_pricing: None,
                video_pricing: None,
                asr_pricing: None,
                realtime_pricing: None,
                metadata: None,
            },
        );
        assert_eq!(config.get_pricing("claude-3", "USD").unwrap().input_price, to_nano(10.0));
        assert_eq!(
            config.get_cache_pricing("claude-3", "USD").unwrap().cache_read_input_price,
            to_nano(1.0)
        );
        assert!(config.get_pricing("nonexistent", "USD").is_none());
    }

    fn v7(models_json: &str) -> String {
        format!(
            r#"{{"version":"7.0","updated_at":"2026-03-29T00:00:00Z","source":"test","models":{models_json}}}"#
        )
    }

    #[test]
    fn test_version_major_parsing() {
        assert_eq!(version_major("7.0"), 7);
        assert_eq!(version_major("10.1"), 10);
        assert_eq!(version_major("1.0"), 1);
        assert_eq!(version_major("abc"), 1);
        assert_eq!(version_major(""), 1);
        let json = r#"{"updated_at":"2026-01-01T00:00:00Z","source":"x","models":{}}"#;
        assert!(PricingConfig::from_json(json).is_ok());
    }

    #[test]
    fn test_v7_text_only() {
        let config = PricingConfig::from_json(&v7(
            r#"{"gemini-flash":{"USD":{"text":{"in":0.075,"out":0.30}}}}"#,
        ))
        .unwrap();
        let pricing = config.get_pricing("gemini-flash", "USD").unwrap();
        assert_eq!(pricing.input_price, to_nano(0.075));
        assert_eq!(pricing.output_price, to_nano(0.30));
    }

    #[test]
    fn test_v7_cache_and_batch() {
        let config = PricingConfig::from_json(&v7(
            r#"{"my-model":{"USD":{"text":{"in":3.0,"out":15.0},"cache":{"read":0.75},"batch":{"in":1.5,"out":7.5}}}}"#,
        ))
        .unwrap();
        assert_eq!(
            config.get_cache_pricing("my-model", "USD").unwrap().cache_read_input_price,
            to_nano(0.75)
        );
        assert_eq!(
            config.models["my-model"].batch_pricing.as_ref().unwrap()["USD"].batch_input_price,
            to_nano(1.5)
        );
    }

    #[test]
    fn test_v7_tiered() {
        let config = PricingConfig::from_json(&v7(
            r#"{"gemini-pro":{"USD":{"text":{"in":1.25,"out":10.0},"tiered":[{"tier_start":0,"tier_end":200000,"in":1.25,"out":10.0},{"tier_start":200000,"in":2.5,"out":15.0}]}}}"#,
        ))
        .unwrap();
        let tiers = config.get_tiered_pricing("gemini-pro", "USD").unwrap();
        assert_eq!(tiers.len(), 2);
        assert_eq!(tiers[0].input_price, to_nano(1.25));
    }

    #[test]
    fn test_v7_tts_model() {
        let config = PricingConfig::from_json(&v7(
            r#"{"tts-model":{"USD":{"text":{"in":15.0},"audio":{"out":10.0}}}}"#,
        ))
        .unwrap();
        let pricing = config.get_pricing("tts-model", "USD").unwrap();
        assert_eq!(pricing.input_price, to_nano(15.0));
        assert_eq!(pricing.output_price, 0);
        assert_eq!(pricing.audio_output_price, Some(to_nano(10.0)));
    }

    #[test]
    fn test_v7_image_model() {
        let config = PricingConfig::from_json(&v7(
            r#"{"image-model":{"USD":{"text":{"in":0.0,"out":0.0},"image":{"out":120.0}}}}"#,
        ))
        .unwrap();
        assert_eq!(
            config.get_pricing("image-model", "USD").unwrap().image_output_price,
            Some(to_nano(120.0))
        );
    }

    #[test]
    fn test_v7_multicurrency() {
        let config = PricingConfig::from_json(&v7(
            r#"{"model":{"USD":{"text":{"in":3.0,"out":15.0}},"CNY":{"text":{"in":21.0,"out":105.0}}}}"#,
        ))
        .unwrap();
        assert!(config.get_pricing("model", "USD").is_some());
        assert_eq!(config.get_pricing("model", "CNY").unwrap().input_price, to_nano(21.0));
    }

    #[test]
    fn test_v1_backward_compat() {
        let json = r#"{
            "version":"1.0","updated_at":"2024-01-15T10:00:00Z","source":"test",
            "models":{"gpt-4":{"pricing":{"USD":{"input_price":10.0,"output_price":30.0}}}}
        }"#;
        let config = PricingConfig::from_json(json).unwrap();
        assert_eq!(config.get_pricing("gpt-4", "USD").unwrap().input_price, to_nano(10.0));
    }

    #[test]
    fn test_nanodollar_precision() {
        let price = 0.28_f64;
        let nano = to_nano(price);
        let back = nano as f64 / 1_000_000_000.0;
        assert!((back - price).abs() < 1e-9);
    }

    #[test]
    fn test_v7_batch_image_out_ignored() {
        let config = PricingConfig::from_json(&v7(
            r#"{"img-model":{"USD":{"text":{"in":0.0,"out":0.0},"batch":{"in":5.0,"out":30.0,"image_out":60.0}}}}"#,
        ))
        .unwrap();
        assert_eq!(
            config.models["img-model"].batch_pricing.as_ref().unwrap()["USD"].batch_input_price,
            to_nano(5.0)
        );
        assert!(config.get_pricing("img-model", "USD").unwrap().image_output_price.is_none());
    }

    #[test]
    fn test_v7_text_none_with_cache() {
        let config = PricingConfig::from_json(&v7(
            r#"{"no-text-model":{"USD":{"cache":{"read":1.0}}}}"#,
        ))
        .unwrap();
        assert!(config.get_pricing("no-text-model", "USD").is_none());
        assert!(config.get_cache_pricing("no-text-model", "USD").is_none());
    }

    fn v8(models_json: &str) -> String {
        format!(
            r#"{{"version":"8.0","updated_at":"2026-03-30T00:00:00Z","source":"test","models":{models_json}}}"#
        )
    }

    #[test]
    fn test_v8_music_per() {
        let config = PricingConfig::from_json(&v8(
            r#"{"lyria-3":{"USD":{"text":{"in":0.0,"out":0.0},"music":{"per":0.08}}}}"#,
        ))
        .unwrap();
        assert_eq!(config.get_pricing("lyria-3", "USD").unwrap().music_price, Some(80_000_000));
    }

    #[test]
    fn test_v8_cache_creation_input() {
        let config = PricingConfig::from_json(&v8(
            r#"{"cached-model":{"USD":{"text":{"in":3.0,"out":15.0},"cache":{"read":0.75,"write":1.25}}}}"#,
        ))
        .unwrap();
        let cache = config.get_cache_pricing("cached-model", "USD").unwrap();
        assert_eq!(cache.cache_read_input_price, to_nano(0.75));
        assert_eq!(cache.cache_creation_input_price, Some(to_nano(1.25)));
    }

    #[test]
    fn test_v8_video_sec_ignored() {
        let config = PricingConfig::from_json(&v8(
            r#"{"video-model":{"USD":{"text":{"in":0.0,"out":0.0},"video":{"sec":0.025}}}}"#,
        ))
        .unwrap();
        assert_eq!(config.get_pricing("video-model", "USD").unwrap().input_price, 0);
    }
}
