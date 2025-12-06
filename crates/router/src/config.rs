use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    Bearer,             // Authorization: Bearer <key>
    Header(String),     // <custom-header>: <key>
    Query(String),      // ?<param>=<key>
    AwsSigV4,           // AWS Signature Version 4
    Azure,              // Azure OpenAI (api-key header)
    GoogleAI,           // Google AI Studio (x-goog-api-key header)
    Vertex,             // Google Vertex AI (Bearer token, usually short-lived)
    DeepSeek,           // DeepSeek API (Bearer token)
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouterConfig {
    pub upstreams: Vec<Upstream>,
}

impl RouterConfig {
    pub fn find_upstream(&self, path: &str) -> Option<&Upstream> {
        // Find all candidates
        let mut candidates: Vec<&Upstream> = self.upstreams.iter()
            .filter(|u| path.starts_with(&u.match_path))
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Sort candidates:
        // 1. Match Length (Descending) - More specific path wins
        // 2. Priority (Descending) - Higher priority wins
        candidates.sort_by(|a, b| {
            let len_cmp = b.match_path.len().cmp(&a.match_path.len());
            if len_cmp != std::cmp::Ordering::Equal {
                return len_cmp;
            }
            b.priority.cmp(&a.priority)
        });

        // Return the best match
        candidates.first().map(|u| *u)
    }
}
