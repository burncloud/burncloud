use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    pub request_id: String,
    pub user_id: Option<String>,
    pub path: String,
    pub status_code: u16,
    pub latency_ms: i64,
}

pub struct LogService;

impl LogService {
    pub async fn list(limit: usize) -> Result<Vec<LogEntry>, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/logs?limit={}", port, limit);

        let client = reqwest::Client::new();
        let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
            return Err(format!("API Error: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        if let Some(data) = json.get("data") {
            serde_json::from_value(data.clone()).map_err(|e| e.to_string())
        } else {
            Ok(vec![])
        }
    }
}
