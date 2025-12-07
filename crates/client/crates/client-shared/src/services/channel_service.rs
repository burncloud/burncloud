use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
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
    fn get_base_url() -> String {
        let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
        format!("http://127.0.0.1:{}/console/api/channel", port)
    }

    pub async fn list() -> Result<Vec<Channel>, String> {
        let url = Self::get_base_url();
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

    pub async fn create(channel: &Channel) -> Result<(), String> {
        let url = Self::get_base_url();
        let client = reqwest::Client::new();
        let resp = client.post(&url)
            .json(channel)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
             let text = resp.text().await.unwrap_or_default();
             return Err(format!("Create failed: {}", text));
        }
        Ok(())
    }

    pub async fn update(channel: &Channel) -> Result<(), String> {
        let url = Self::get_base_url();
        let client = reqwest::Client::new();
        let resp = client.put(&url)
            .json(channel)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
             return Err(format!("Update failed: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn delete(id: i64) -> Result<(), String> {
        let url = format!("{}/{}", Self::get_base_url(), id);
        let client = reqwest::Client::new();
        let resp = client.delete(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        if !resp.status().is_success() {
             return Err(format!("Delete failed: {}", resp.status()));
        }
        Ok(())
    }
}