use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuthType {
    Bearer,     // Authorization: Bearer <key>
    XApiKey,    // x-api-key: <key>
    Query(String), // ?key=<key> or ?<param>=<key>
    AwsSigV4,   // AWS Signature Version 4 (Bedrock)
}

impl From<&str> for AuthType {
    fn from(s: &str) -> Self {
        match s {
            "Bearer" => AuthType::Bearer,
            "XApiKey" => AuthType::XApiKey,
            "AwsSigV4" => AuthType::AwsSigV4,
            s if s.starts_with("Query:") => {
                let param = s.trim_start_matches("Query:");
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouterConfig {
    pub upstreams: Vec<Upstream>,
}

impl RouterConfig {
    /// Find the best matching upstream for a given path.
    /// Implements Longest Prefix Match.
    pub fn find_upstream(&self, path: &str) -> Option<&Upstream> {
        let mut best_match: Option<&Upstream> = None;
        let mut max_len = 0;

        for upstream in &self.upstreams {
            if path.starts_with(&upstream.match_path) {
                let len = upstream.match_path.len();
                if len > max_len {
                    max_len = len;
                    best_match = Some(upstream);
                }
            }
        }

        best_match
    }
}
