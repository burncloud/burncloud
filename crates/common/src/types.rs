use serde::{Deserialize, Serialize};

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
        Self {
            models_dir: "models".to_string(),
            server_port: 8080,
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
