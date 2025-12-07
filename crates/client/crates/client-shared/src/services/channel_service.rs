use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Channel {
    #[serde(default)]
    pub id: i64,
    #[serde(rename = "type")]
    pub type_: i32,
    #[serde(default)]
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub models: String,
    pub group: Option<String>,
    #[serde(default)]
    pub status: i32,
    #[serde(default)]
    pub priority: i32,
    #[serde(default)]
    pub weight: i32,
}

pub struct ChannelService;

impl ChannelService {
    pub async fn list() -> Result<Vec<Channel>, String> {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        let url = format!("http://127.0.0.1:{}/console/api/channel", port);

        let client = reqwest::Client::new();
        let resp = client.get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

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
