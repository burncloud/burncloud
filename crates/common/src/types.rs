use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
            14 => ChannelType::Anthropic,
            15 => ChannelType::Baidu,
            16 => ChannelType::Zhipu,
            17 => ChannelType::Ali,
            24 => ChannelType::Gemini,
            25 => ChannelType::Moonshot,
            43 => ChannelType::DeepSeek,
            _ => ChannelType::Unknown, // Simplify for now, can expand later
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
    // ChannelInfo fields from New API are flattened or handled separately in logic
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

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct User {
    pub id: i32,
    pub username: String,
    #[serde(skip_serializing)] // Don't expose password hash
    pub password: String,
    pub display_name: String,
    pub role: i32,   // 1: Common, 10: Admin, 100: Root
    pub status: i32, // 1: Enabled, 2: Disabled
    pub email: Option<String>,
    pub github_id: Option<String>,
    pub wechat_id: Option<String>,
    pub access_token: Option<String>,
    pub quota: i64,      // 500000 = $1
    pub used_quota: i64,
    pub request_count: i32,
    pub group: String,
    pub aff_code: Option<String>,
    pub aff_count: i32,
    pub aff_quota: i64,
    pub inviter_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Token {
    pub id: i32,
    pub user_id: i32,
    pub key: String,
    pub status: i32,
    pub name: String,
    pub remain_quota: i64, // -1 for unlimited
    pub unlimited_quota: bool,
    pub used_quota: i64,
    pub created_time: i64,
    pub accessed_time: i64,
    pub expired_time: i64, // -1 for never
}
