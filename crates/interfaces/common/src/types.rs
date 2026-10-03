// Two struct fields legitimately need HashMap<String, Value>:
//   OpenAIChatRequest::extra      — generic LLM API passthrough fields
//   RequestMapping::add_fields    — dynamic request field injection
// All other Value uses in this codebase must go through typed structs.
#![allow(clippy::disallowed_types)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::collections::HashMap;
use std::fmt;

// Re-export nanodollar conversion utilities owned by Commerce, keeping the legacy
// alias names used by existing consumers of this module.
pub use burncloud_commerce_contracts::price_u64::{
    dollars_to_nano as dollars_to_nanodollars, nano_to_dollars as nanodollars_to_dollars,
    NANO_PER_DOLLAR as NANODOLLAR_SCALE,
};

// ---------------------------------------------------------------------------
// S1-B: the price and pricing value objects moved to the Commerce contract crate
// (`burncloud_commerce_contracts::pricing`). They are re-exported here under the same
// names and module path, so existing `burncloud_common::types::*` consumers keep
// compiling. `ModelMetadata` and the `pricing_config` document schema stay in
// `burncloud_common::pricing_config`.
// ---------------------------------------------------------------------------
pub use burncloud_commerce_contracts::pricing::{
    BillingExchangeRate, BillingPrice, BillingPriceInput, BillingTieredPrice,
    BillingTieredPriceInput, Currency, ExchangeRate, FullPricing, MultiCurrencyPrice, Price,
    PriceInput, TieredPrice, TieredPriceInput,
};

/// DiffServ-style three-color traffic class for the L2 Shaper.
///
/// `Green` / `Yellow` / `Red` map to 3-tier reservation buckets per channel.
/// Higher-priority colors are preferred; lower-priority colors may borrow
/// idle capacity upward. See `crates/traffic/router/src/rate_budget.rs` and
/// `docs/code/GLOSSARY.md` § 2.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrafficColor {
    /// Highest priority — matches Enterprise order type / reserved capacity.
    Green,
    /// Default priority — matches Value order type.
    #[default]
    Yellow,
    /// Lowest priority — matches Budget / best-effort traffic.
    Red,
}

impl TrafficColor {
    /// Static label for `router_logs.traffic_color` (single-character DB value).
    pub fn as_char(&self) -> char {
        match self {
            TrafficColor::Green => 'G',
            TrafficColor::Yellow => 'Y',
            TrafficColor::Red => 'R',
        }
    }
}

impl fmt::Display for TrafficColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_char())
    }
}

// OpenAI Compatible Types
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,

    // Capture all other fields (Generic Passthrough)
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChatChoice>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChatChoice {
    pub index: u32,
    pub message: OpenAIChatMessage,
    pub finish_reason: Option<String>,
}

// --- Ported from New API ---

// ---------------------------------------------------------------------------
// S1-C: the Supply-owned channel and ability types moved to the Supply contract crate
// (`burncloud_supply_contracts`). They are re-exported here under the same names and module path
// so existing `burncloud_common::types::*` and root-level consumers keep compiling.
// S1-D removed the legacy `Recharge`/`User`/`Token` DTOs that used to live here: they had no
// consumer anywhere in the workspace and duplicated the Identity types.
// ---------------------------------------------------------------------------
pub use burncloud_supply_contracts::{
    Ability, Channel, ChannelAbility, ChannelProvider, ChannelType,
};

/// Protocol Configuration for dynamic protocol adapters
/// Allows runtime configuration of API endpoints and request/response mappings
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ProtocolConfig {
    pub id: i32,
    /// Channel type (1=OpenAI, 2=Anthropic, 3=Azure, etc.)
    pub channel_type: i32,
    /// API version string (e.g., "2024-02-01", "v1")
    pub api_version: String,
    /// Whether this is the default config for the channel type
    pub is_default: i32,
    /// Chat completions endpoint (supports placeholders like {deployment_id})
    pub chat_endpoint: Option<String>,
    /// Embeddings endpoint
    pub embed_endpoint: Option<String>,
    /// Models list endpoint
    pub models_endpoint: Option<String>,
    /// Request field mapping rules (JSON)
    pub request_mapping: Option<String>,
    /// Response field mapping rules (JSON)
    pub response_mapping: Option<String>,
    /// Detection rules for auto-detection (JSON)
    pub detection_rules: Option<String>,
    /// Creation timestamp
    pub created_at: Option<i64>,
    /// Update timestamp
    pub updated_at: Option<i64>,
}

impl Default for ProtocolConfig {
    fn default() -> Self {
        Self {
            id: 0,
            channel_type: 1, // OpenAI
            api_version: "default".to_string(),
            is_default: 1,
            chat_endpoint: Some("/v1/chat/completions".to_string()),
            embed_endpoint: Some("/v1/embeddings".to_string()),
            models_endpoint: Some("/v1/models".to_string()),
            request_mapping: None,
            response_mapping: None,
            detection_rules: None,
            created_at: None,
            updated_at: None,
        }
    }
}

/// Request mapping configuration for protocol adaptation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RequestMapping {
    /// Field mappings: "target_field" => "source_field"
    #[serde(default)]
    pub field_map: HashMap<String, String>,
    /// Field renames: "old_name" => "new_name"
    #[serde(default)]
    pub rename: HashMap<String, String>,
    /// Fields to add to the request
    #[serde(default)]
    pub add_fields: HashMap<String, Value>,
}

/// Response mapping configuration for protocol adaptation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResponseMapping {
    /// Path to extract content from response (e.g., "choices[0].message.content")
    pub content_path: Option<String>,
    /// Path to extract token usage
    pub usage_path: Option<String>,
    /// Path to extract error message
    pub error_path: Option<String>,
}
