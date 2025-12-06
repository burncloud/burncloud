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
    Qwen,               // Alibaba Cloud Qwen (Bearer token)
    Claude,             // Anthropic Claude (x-api-key header)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub upstream_id: String,
    pub weight: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub match_path: String,
    pub members: Vec<GroupMember>,
}

#[derive(Debug, Clone)]
pub enum RouteTarget<'a> {
    Upstream(&'a Upstream),
    Group(&'a Group),
}

impl<'a> RouteTarget<'a> {
    pub fn match_path(&self) -> &str {
        match self {
            RouteTarget::Upstream(u) => &u.match_path,
            RouteTarget::Group(g) => &g.match_path,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouterConfig {
    pub upstreams: Vec<Upstream>,
    #[serde(default)]
    pub groups: Vec<Group>,
}

impl RouterConfig {
    pub fn find_route(&self, path: &str) -> Option<RouteTarget<'_>> {
        // Collect all candidates (Upstreams and Groups)
        let mut candidates: Vec<RouteTarget> = Vec::new();

        for u in &self.upstreams {
            if path.starts_with(&u.match_path) {
                candidates.push(RouteTarget::Upstream(u));
            }
        }

        for g in &self.groups {
            if path.starts_with(&g.match_path) {
                candidates.push(RouteTarget::Group(g));
            }
        }

        if candidates.is_empty() {
            return None;
        }

        // Sort candidates:
        // 1. Match Length (Descending) - More specific path wins
        // 2. Priority (Descending) - Higher priority wins
        // Note: Groups don't currently have explicit priority field in DB, assuming 0 or default behavior.
        // We prioritize Upstreams over Groups if length is equal for now, or just by order.
        candidates.sort_by(|a, b| {
            let len_cmp = b.match_path().len().cmp(&a.match_path().len());
            if len_cmp != std::cmp::Ordering::Equal {
                return len_cmp;
            }
            // If lengths equal, prefer Upstream (legacy behavior) or arbitrary stability.
            // Let's use ID for stability if types differ.
            std::cmp::Ordering::Equal
        });

        candidates.first().cloned()
    }
    
    // Helper to find upstream by ID (for Group resolution)
    pub fn get_upstream(&self, id: &str) -> Option<&Upstream> {
        self.upstreams.iter().find(|u| u.id == id)
    }
}
