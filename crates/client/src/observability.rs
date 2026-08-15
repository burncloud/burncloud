use reqwest::Client;
use serde::Deserialize;

use crate::backend::{server_root, ClientState};

#[derive(Debug, Clone, Deserialize, PartialEq, Default)]
pub struct FullRouterLog {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub request_id: String,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub upstream_id: Option<String>,
    #[serde(default)]
    pub status_code: i32,
    #[serde(default)]
    pub latency_ms: i64,
    #[serde(default)]
    pub prompt_tokens: i32,
    #[serde(default)]
    pub completion_tokens: i32,
    #[serde(default)]
    pub cost: i64,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub cache_read_tokens: i32,
    #[serde(default)]
    pub reasoning_tokens: i32,
    #[serde(default)]
    pub pricing_region: Option<String>,
    #[serde(default)]
    pub video_tokens: i32,
    #[serde(default)]
    pub cache_write_tokens: i32,
    #[serde(default)]
    pub audio_input_tokens: i32,
    #[serde(default)]
    pub audio_output_tokens: i32,
    #[serde(default)]
    pub image_tokens: i32,
    #[serde(default)]
    pub embedding_tokens: i32,
    #[serde(default)]
    pub input_cost: i64,
    #[serde(default)]
    pub output_cost: i64,
    #[serde(default)]
    pub cache_read_cost: i64,
    #[serde(default)]
    pub cache_write_cost: i64,
    #[serde(default)]
    pub audio_cost: i64,
    #[serde(default)]
    pub image_cost: i64,
    #[serde(default)]
    pub video_cost: i64,
    #[serde(default)]
    pub reasoning_cost: i64,
    #[serde(default)]
    pub embedding_cost: i64,
    #[serde(default)]
    pub layer_decision: Option<String>,
    #[serde(default)]
    pub traffic_color: Option<String>,
    #[serde(default)]
    pub cost_status: Option<String>,
    #[serde(default)]
    pub error_type: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

impl FullRouterLog {
    pub fn total_tokens(&self) -> i64 {
        self.prompt_tokens as i64
            + self.completion_tokens as i64
            + self.cache_read_tokens as i64
            + self.cache_write_tokens as i64
            + self.reasoning_tokens as i64
            + self.video_tokens as i64
            + self.audio_input_tokens as i64
            + self.audio_output_tokens as i64
            + self.image_tokens as i64
            + self.embedding_tokens as i64
    }

    pub fn cost_usd(&self) -> f64 {
        self.cost as f64 / 1_000_000_000.0
    }

    pub fn cost_component_usd(value: i64) -> f64 {
        value as f64 / 1_000_000_000.0
    }

    pub fn status_label(&self) -> &'static str {
        if self.status_code >= 500 || self.error_type.as_deref() == Some("timeout") {
            "Timeout"
        } else if self
            .layer_decision
            .as_deref()
            .unwrap_or("")
            .contains("failover")
        {
            "Fallback"
        } else if self.status_code >= 400 {
            "Error"
        } else {
            "Success"
        }
    }
}

#[derive(Deserialize)]
struct LogPage {
    #[serde(default)]
    data: Vec<FullRouterLog>,
}

pub async fn full_logs(limit: usize) -> Result<Vec<FullRouterLog>, String> {
    let token = ClientState::load()
        .auth_token
        .ok_or_else(|| "No authenticated BurnCloud session".to_string())?;
    let page_size = limit.clamp(1, 500);
    let url = format!(
        "{}/console/api/logs?page=1&page_size={page_size}",
        server_root()
    );
    let response = Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = response.status();
    let text = response.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("Logs API failed ({status}): {text}"));
    }
    serde_json::from_str::<LogPage>(&text)
        .map(|page| page.data)
        .map_err(|e| format!("Invalid full router log response: {e}"))
}
