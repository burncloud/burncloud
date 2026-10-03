// The two struct fields that legitimately needed `HashMap<String, Value>` left with S1-E:
// `OpenAIChatRequest::extra` moved to the Traffic contract, and the duplicate `RequestMapping`
// was deleted with its local counterpart in `traffic/router/src/adaptor/mapping.rs` as the owner.
// All `Value` uses in this codebase must go through typed structs.
#![allow(clippy::disallowed_types)]

use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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

// ---------------------------------------------------------------------------
// S1-E: the Traffic-owned protocol DTOs and the scheduling colour moved to the
// Traffic contract crate (`burncloud_traffic_contracts`). They are re-exported here under the
// same names and module path, so existing `burncloud_common::types::*` consumers keep
// compiling. `RequestMapping`/`ResponseMapping` were NOT moved: the copies that used to live
// here had no consumer and duplicated `traffic/router/src/adaptor/mapping.rs`, so they were
// deleted instead.
// ---------------------------------------------------------------------------
pub use burncloud_traffic_contracts::{
    OpenAIChatChoice, OpenAIChatMessage, OpenAIChatRequest, OpenAIChatResponse, TrafficColor,
};

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
