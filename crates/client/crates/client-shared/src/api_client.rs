use anyhow::Result;
use once_cell::sync::Lazy;
use reqwest::Client;
use serde::{Deserialize, Serialize};

// Global singleton for API Client, configured for localhost by default
pub static API_CLIENT: Lazy<ApiClient> = Lazy::new(|| {
    let port = burncloud_common::constants::DEFAULT_PORT;
    ApiClient::new(&format!("http://localhost:{}/console", port))
});

#[derive(Clone)]
pub struct ApiClient {
    base_url: String,
    client: Client,
}

// DTOs mirroring Server structs
#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct ChannelDto {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: String,
    pub priority: i32,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct TokenDto {
    pub token: String,
    pub user_id: String,
    pub status: String,
    #[serde(default = "default_quota")]
    pub quota_limit: i64,
    #[serde(default)]
    pub used_quota: i64,
}

fn default_quota() -> i64 {
    -1
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
            return Err(anyhow::anyhow!(
                "Failed to list channels: {}",
                resp.status()
            ));
        }

        let channels = resp.json::<Vec<ChannelDto>>().await?;
        Ok(channels)
    }

    pub async fn create_channel(&self, channel: ChannelDto) -> Result<()> {
        let url = format!("{}/channels", self.base_url);
        let resp = self.client.post(&url).json(&channel).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to create channel: {}",
                resp.status()
            ));
        }
        Ok(())
    }

    pub async fn delete_channel(&self, id: &str) -> Result<()> {
        let url = format!("{}/channels/{}", self.base_url, id);
        let resp = self.client.delete(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to delete channel: {}",
                resp.status()
            ));
        }
        Ok(())
    }

    // --- Token Operations ---

    pub async fn list_tokens(&self) -> Result<Vec<TokenDto>> {
        let url = format!("{}/api/tokens", self.base_url);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list tokens: {}", resp.status()));
        }
        Ok(resp.json().await?)
    }

    pub async fn create_token(&self, user_id: &str, quota_limit: Option<i64>) -> Result<String> {
        let url = format!("{}/api/tokens", self.base_url);
        let body = serde_json::json!({
            "user_id": user_id,
            "quota_limit": quota_limit
        });
        let resp = self.client.post(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to create token: {}", resp.status()));
        }

        let json: serde_json::Value = resp.json().await?;
        let token = json["token"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Response missing token field"))?
            .to_string();

        Ok(token)
    }

    pub async fn delete_token(&self, token: &str) -> Result<()> {
        let url = format!("{}/api/tokens/{}", self.base_url, token);
        let resp = self.client.delete(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to delete token: {}", resp.status()));
        }
        Ok(())
    }
}
