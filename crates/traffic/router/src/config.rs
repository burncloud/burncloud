use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    Bearer,         // Authorization: Bearer <key>
    Header(String), // <custom-header>: <key>
    Query(String),  // ?<param>=<key>
    AwsSigV4,       // AWS Signature Version 4
    Azure,          // Azure OpenAI (api-key header)
    GoogleAI,       // Google AI Studio (x-goog-api-key header)
    Vertex,         // Google Vertex AI (Bearer token, usually short-lived)
    DeepSeek,       // DeepSeek API (Bearer token)
    Qwen,           // Alibaba Cloud Qwen (Bearer token)
    Claude,         // Anthropic Claude (x-api-key header)
}

impl From<&str> for AuthType {
    fn from(s: &str) -> Self {
        match s {
            "Bearer" => AuthType::Bearer,
            "XApiKey" => AuthType::Header("x-api-key".to_string()), // Alias for backward compatibility
            "AwsSigV4" => AuthType::AwsSigV4,
            "Azure" => AuthType::Azure,
            "GoogleAI" => AuthType::GoogleAI,
            "Vertex" => AuthType::Vertex,
            "DeepSeek" => AuthType::DeepSeek,
            "Qwen" => AuthType::Qwen,
            "Claude" => AuthType::Claude,
            s if s.starts_with("Header:") => {
                let header_name = s.trim_start_matches("Header:").trim();
                AuthType::Header(header_name.to_string())
            }
            s if s.starts_with("Query:") => {
                let param = s.trim_start_matches("Query:").trim();
                AuthType::Query(param.to_string())
            }
            _ => AuthType::Bearer, // Default
        }
    }
}

/// Internal representation of an upstream channel for request routing.
/// Populated from `Channel` objects returned by `ModelRouter`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Upstream {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: AuthType,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub param_override: Option<String>,
    #[serde(default)]
    pub header_override: Option<String>,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub pricing_region: Option<String>,
}
