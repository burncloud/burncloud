use reqwest::Client;
use serde::{Deserialize, Serialize};
use anyhow::Result;
use once_cell::sync::Lazy;

// Global singleton for API Client, configured for localhost by default
pub static API_CLIENT: Lazy<ApiClient> = Lazy::new(|| ApiClient::new("http://localhost:4000/console"));

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    client: Client,
}

// DTOs mirroring Server structs
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct ChannelDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: String,
    pub priority: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TokenDto {
    pub token: String,
    pub user_id: String,
    pub status: String,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
        }
    }

    // --- Channel Operations ---

    pub async fn list_channels(&self) -> Result<Vec<ChannelDto>> {
        let url = format!("{}/channels", self.base_url);
        let resp = self.client.get(&url).send().await?;
        
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list channels: {}", resp.status()));
        }
        
        let channels = resp.json::<Vec<ChannelDto>>().await?;
        Ok(channels)
    }

    pub async fn create_channel(&self, channel: ChannelDto) -> Result<()> {
        let url = format!("{}/channels", self.base_url);
        let resp = self.client.post(&url).json(&channel).send().await?;
        
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to create channel: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn delete_channel(&self, id: &str) -> Result<()> {
        let url = format!("{}/channels/{}", self.base_url, id);
        let resp = self.client.delete(&url).send().await?;
        
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to delete channel: {}", resp.status()));
        }
        Ok(())
    }

    // --- Token Operations ---

    pub async fn list_tokens(&self) -> Result<Vec<TokenDto>> {
        let url = format!("{}/tokens", self.base_url);
        let resp = self.client.get(&url).send().await?;
        
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list tokens: {}", resp.status()));
        }
        Ok(resp.json().await?)
    }
    
    pub async fn create_token(&self, user_id: &str) -> Result<String> {
        let url = format!("{}/tokens", self.base_url);
        let body = serde_json::json!({ "user_id": user_id });
        let resp = self.client.post(&url).json(&body).send().await?;
        
        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to create token: {}", resp.status()));
        }
        
        let json: serde_json::Value = resp.json().await?;
        let token = json["token"].as_str()
            .ok_or_else(|| anyhow::anyhow!("Response missing token field"))?
            .to_string();
            
        Ok(token)
    }
}
