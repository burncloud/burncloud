//! Supply-owned value objects for channels (providers) and their abilities.
//!
//! Moved here by S1-C from `burncloud_common::types`. The public surface is unchanged: the same
//! names are re-exported from `burncloud_common`, so existing consumers keep compiling.
//!
//! Two compatibility promises live in this file and must not change silently:
//!   * [`ChannelType`] discriminants are the New API wire encoding, and `From<i32>` falls back to
//!     `Unknown` for values that are not listed (there is no `Err` case).
//!   * [`Channel`] serializes its `type_` field as `"type"` (the historical JSON name).
//!
//! This is a pure contract: it has no database framework. The `channel_providers` and
//! `channel_abilities` row mapping lives in `burncloud-supply-channel`, which owns the SQL.

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
