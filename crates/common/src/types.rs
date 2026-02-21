use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::FromRow;
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

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
            Currency::CNY => "¥",
            Currency::EUR => "€",
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
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct ExchangeRate {
    pub id: i32,
    pub from_currency: String,
    pub to_currency: String,
    /// Exchange rate scaled by 10^9 (e.g., 7.24 CNY/USD = 7_240_000_000)
    pub rate: i64,
    pub updated_at: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelInfo {
    pub name: String,
    pub size: u64,
    pub downloaded: bool,
    pub path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub models_dir: String,
    pub server_port: u16,
    pub max_memory: u64,
    pub gpu_enabled: bool,
}

impl Default for Config {
    fn default() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(3000);

        Self {
            models_dir: "models".to_string(),
            server_port: port,
            max_memory: 8192,
            gpu_enabled: false,
        }
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
                                     // ChannelInfo fields from New API are flattened or handled separately in logic
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Recharge {
    pub id: i32,
    pub user_id: String,
    pub amount: f64,
    pub description: Option<String>,
    pub created_at: Option<i64>, // Unix timestamp
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Ability {
    pub group: String,
    pub model: String,
    pub channel_id: i32,
    pub enabled: bool,
    pub priority: i64,
    pub weight: i32,
}

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
    pub is_default: bool,
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
            is_default: true,
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

/// Tiered pricing configuration for models with usage-based pricing tiers
/// (e.g., Qwen models with different prices based on context length)
/// Prices are stored as i64 nanodollars (9 decimal precision)
/// Note: Using i64 instead of u64 for PostgreSQL BIGINT compatibility
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TieredPrice {
    pub id: i32,
    /// Model name
    pub model: String,
    /// Region for pricing (e.g., "cn", "international", NULL for universal)
    pub region: Option<String>,
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

/// Multi-currency price entry for prices_v2 table
/// Supports USD, CNY, EUR and advanced pricing fields
/// All prices are stored in nanodollars (i64, 9 decimal precision)
/// Note: Using i64 instead of u64 for PostgreSQL BIGINT compatibility
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PriceV2 {
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
    /// Source of pricing data (litellm, manual, community, etc.)
    pub source: Option<String>,
    /// Region for pricing (cn, international, NULL for universal)
    pub region: Option<String>,
    /// Context window for the model
    pub context_window: Option<i64>,
    /// Maximum output tokens
    pub max_output_tokens: Option<i64>,
    /// Whether the model supports vision/image input
    pub supports_vision: Option<bool>,
    /// Whether the model supports function calling
    pub supports_function_calling: Option<bool>,
    /// Last sync timestamp
    pub synced_at: Option<i64>,
    /// Creation timestamp
    pub created_at: Option<i64>,
    /// Update timestamp
    pub updated_at: Option<i64>,
}

/// Input for creating/updating a PriceV2 entry
/// All prices are in nanodollars (i64, 9 decimal precision)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceV2Input {
    pub model: String,
    pub currency: String,
    /// Input price per 1M tokens in nanodollars (i64 for DB compatibility)
    pub input_price: i64,
    /// Output price per 1M tokens in nanodollars (i64 for DB compatibility)
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
    pub source: Option<String>,
    pub region: Option<String>,
    pub context_window: Option<i64>,
    pub max_output_tokens: Option<i64>,
    pub supports_vision: Option<bool>,
    pub supports_function_calling: Option<bool>,
}

impl Default for PriceV2Input {
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
            source: None,
            region: None,
            context_window: None,
            max_output_tokens: None,
            supports_vision: None,
            supports_function_calling: None,
        }
    }
}

#[cfg(test)]
mod currency_tests {
    use super::*;

    #[test]
    fn test_currency_default() {
        let currency = Currency::default();
        assert_eq!(currency, Currency::USD);
    }

    #[test]
    fn test_currency_symbol() {
        assert_eq!(Currency::USD.symbol(), "$");
        assert_eq!(Currency::CNY.symbol(), "¥");
        assert_eq!(Currency::EUR.symbol(), "€");
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
        assert_eq!(Currency::from_str("USD").unwrap(), Currency::USD);
        assert_eq!(Currency::from_str("usd").unwrap(), Currency::USD);
        assert_eq!(Currency::from_str("CNY").unwrap(), Currency::CNY);
        assert_eq!(Currency::from_str("eur").unwrap(), Currency::EUR);
        assert!(Currency::from_str("GBP").is_err());
    }

    #[test]
    fn test_currency_serde() {
        // Test serialization
        let json = serde_json::to_string(&Currency::USD).unwrap();
        assert_eq!(json, "\"usd\"");

        let json = serde_json::to_string(&Currency::CNY).unwrap();
        assert_eq!(json, "\"cny\"");

        // Test deserialization (lowercase)
        let currency: Currency = serde_json::from_str("\"usd\"").unwrap();
        assert_eq!(currency, Currency::USD);

        let currency: Currency = serde_json::from_str("\"eur\"").unwrap();
        assert_eq!(currency, Currency::EUR);
    }

    #[test]
    fn test_multi_currency_price() {
        // Prices are in nanodollars: $0.002 = 2_000_000 nanodollars
        let price = MultiCurrencyPrice {
            currency: Currency::CNY,
            input_price: 2_000_000,    // $0.002 in nanodollars
            output_price: 6_000_000,   // $0.006 in nanodollars
        };

        assert_eq!(price.currency, Currency::CNY);
        assert_eq!(price.input_price, 2_000_000);
        assert_eq!(price.output_price, 6_000_000);

        // Test serialization
        let json = serde_json::to_string(&price).unwrap();
        assert!(json.contains("\"currency\":\"cny\""));
    }
}
