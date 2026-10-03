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

/// Channel Type Enum (Compatible with New API constants)
/// See constant/channel.go in New API
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub enum ChannelType {
    Unknown = 0,
    OpenAI = 1,
    Midjourney = 2,
    Azure = 3,
    Ollama = 4,
    MidjourneyPlus = 5,
    OpenAIMax = 6,
    OhMyGPT = 7,
    Custom = 8,
    AILS = 9,
    AIProxy = 10,
    PaLM = 11,
    API2GPT = 12,
    AIGC2D = 13,
    Anthropic = 14,
    Baidu = 15,
    Zhipu = 16,
    Ali = 17,
    Xunfei = 18,
    Qihoo360 = 19,
    OpenRouter = 20,
    AIProxyLibrary = 21,
    FastGPT = 22,
    Tencent = 23,
    Gemini = 24,
    Moonshot = 25,
    ZhipuV4 = 26,
    Perplexity = 27,
    LingYiWanWu = 31,
    Aws = 33,
    Cohere = 34,
    MiniMax = 35,
    SunoAPI = 36,
    Dify = 37,
    Jina = 38,
    Cloudflare = 39,
    SiliconFlow = 40,
    VertexAi = 41,
    Mistral = 42,
    DeepSeek = 43,
    MokaAI = 44,
    VolcEngine = 45,
    BaiduV2 = 46,
    Xinference = 47,
    Xai = 48,
    Coze = 49,
    Kling = 50,
    Jimeng = 51,
    Vidu = 52,
    Submodel = 53,
    DoubaoVideo = 54,
    Sora = 55,
    Replicate = 56,
    Zai = 57,
    NewApi = 58,
    Dummy,
}

impl From<i32> for ChannelType {
    fn from(i: i32) -> Self {
        match i {
            1 => ChannelType::OpenAI,
            2 => ChannelType::Midjourney,
            3 => ChannelType::Azure,
            4 => ChannelType::Ollama,
            5 => ChannelType::MidjourneyPlus,
            6 => ChannelType::OpenAIMax,
            7 => ChannelType::OhMyGPT,
            8 => ChannelType::Custom,
            9 => ChannelType::AILS,
            10 => ChannelType::AIProxy,
            11 => ChannelType::PaLM,
            12 => ChannelType::API2GPT,
            13 => ChannelType::AIGC2D,
            14 => ChannelType::Anthropic,
            15 => ChannelType::Baidu,
            16 => ChannelType::Zhipu,
            17 => ChannelType::Ali,
            18 => ChannelType::Xunfei,
            19 => ChannelType::Qihoo360,
            20 => ChannelType::OpenRouter,
            21 => ChannelType::AIProxyLibrary,
            22 => ChannelType::FastGPT,
            23 => ChannelType::Tencent,
            24 => ChannelType::Gemini,
            25 => ChannelType::Moonshot,
            26 => ChannelType::ZhipuV4,
            27 => ChannelType::Perplexity,
            31 => ChannelType::LingYiWanWu,
            33 => ChannelType::Aws,
            34 => ChannelType::Cohere,
            35 => ChannelType::MiniMax,
            36 => ChannelType::SunoAPI,
            37 => ChannelType::Dify,
            38 => ChannelType::Jina,
            39 => ChannelType::Cloudflare,
            40 => ChannelType::SiliconFlow,
            41 => ChannelType::VertexAi,
            42 => ChannelType::Mistral,
            43 => ChannelType::DeepSeek,
            44 => ChannelType::MokaAI,
            45 => ChannelType::VolcEngine,
            46 => ChannelType::BaiduV2,
            47 => ChannelType::Xinference,
            48 => ChannelType::Xai,
            49 => ChannelType::Coze,
            50 => ChannelType::Kling,
            51 => ChannelType::Jimeng,
            52 => ChannelType::Vidu,
            53 => ChannelType::Submodel,
            54 => ChannelType::DoubaoVideo,
            55 => ChannelType::Sora,
            56 => ChannelType::Replicate,
            57 => ChannelType::Zai,
            58 => ChannelType::NewApi,
            _ => ChannelType::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Channel {
    pub id: i32,
    #[serde(rename = "type")]
    pub type_: i32, // Use i32 for raw compatibility, or ChannelType
    pub key: String,
    pub status: i32, // 1: Enabled, 2: Manually Disabled, 3: Auto Disabled
    pub name: String,
    pub weight: i32,
    pub created_time: Option<i64>,
    pub test_time: Option<i64>,
    pub response_time: Option<i32>, // ms
    pub base_url: Option<String>,
    pub models: String, // Comma separated
    pub group: String,  // Comma separated, default "default"
    pub used_quota: i64,
    pub model_mapping: Option<String>, // JSON string
    pub priority: i64,
    pub auto_ban: i32, // 0 or 1
    pub other_info: Option<String>,
    pub tag: Option<String>,
    pub setting: Option<String>,
    pub param_override: Option<String>,
    pub header_override: Option<String>,
    pub remark: Option<String>,
    pub api_version: Option<String>, // API version for protocol adaptation
    pub pricing_region: Option<String>, // Pricing region: 'cn', 'international', or NULL for universal
    // L2 Shaper inputs (migration 0011). NULL → shaper fail-open for this channel.
    pub rpm_cap: Option<i32>,
    pub tpm_cap: Option<i64>,
    pub reservation_green: Option<f64>,
    pub reservation_yellow: Option<f64>,
    pub reservation_red: Option<f64>,
    // ChannelInfo fields from New API are flattened or handled separately in logic
}

/// Recharge record. Amount is stored as i64 nanodollars (9 decimal precision).
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Recharge {
    pub id: i32,
    pub user_id: String,
    /// Amount in nanodollars (9 decimal precision)
    pub amount: i64,
    pub description: Option<String>,
    pub created_at: Option<i64>, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Ability {
    pub group: String,
    pub model: String,
    pub channel_id: i32,
    pub enabled: i32,
    pub priority: i64,
    pub weight: i32,
}

impl Ability {
    pub fn enabled_bool(&self) -> bool {
        self.enabled != 0
    }
}

/// Spec-aligned alias: `channel_abilities` row type.
pub type ChannelAbility = Ability;

/// Spec-aligned alias: `channel_providers` row type.
pub type ChannelProvider = Channel;

/// User with dual-currency wallet for regional pricing support.
/// Balance fields use i64 nanodollars (9 decimal precision) for PostgreSQL BIGINT compatibility.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    #[serde(skip_serializing)] // Don't expose password hash
    pub password: String, // Mapped to password_hash in query if aliased?
    pub display_name: String,
    pub role: i32,   // 1: Common, 10: Admin, 100: Root
    pub status: i32, // 1: Enabled, 2: Disabled
    pub email: Option<String>,
    pub github_id: Option<String>,
    pub wechat_id: Option<String>,
    pub access_token: Option<String>,
    /// USD balance in nanodollars (9 decimal precision)
    /// For display: divide by 1_000_000_000 to get dollars
    #[sqlx(default)]
    pub balance_usd: i64,
    /// CNY balance in nanodollars (9 decimal precision)
    /// For display: divide by 1_000_000_000 to get yuan
    #[sqlx(default)]
    pub balance_cny: i64,
    pub request_count: i32,
    pub group: String,
    pub aff_code: Option<String>,
    pub aff_count: i32,
    pub inviter_id: Option<String>,
    /// User's preferred currency for display (USD, CNY, EUR)
    #[sqlx(default)]
    pub preferred_currency: Option<String>,
    #[sqlx(default)]
    pub created_at: Option<i64>, // Unix timestamp
}

/// Token without quota tracking - quota is now managed at user level via dual-currency wallet.
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Token {
    pub id: i32,
    pub user_id: String,
    pub key: String,
    pub status: i32,
    pub name: String,
    pub created_time: i64,
    pub accessed_time: i64,
    pub expired_time: i64, // -1 for never
}

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
