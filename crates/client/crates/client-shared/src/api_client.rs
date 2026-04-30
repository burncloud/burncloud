// HTTP client — raw API response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

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

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct GroupDto {
    pub id: String,
    pub name: String,
    pub strategy: String,
    pub match_path: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
pub struct GroupMemberDto {
    pub upstream_id: String,
    pub weight: i32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginResponse {
    pub id: String,
    pub username: String,
    pub roles: Vec<String>,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResult {
    pub success: bool,
    pub message: Option<String>,
    pub data: Option<LoginResponse>,
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

    /// Server root URL derived from base_url (strips `/console` suffix).
    /// Group routes are mounted at server root without the `/console/api` prefix.
    fn server_root(&self) -> &str {
        self.base_url.strip_suffix("/console").unwrap_or(&self.base_url)
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

    pub async fn update_token_status(&self, token: &str, status: &str) -> Result<()> {
        let url = format!("{}/api/tokens/{}", self.base_url, token);
        let body = serde_json::json!({
            "status": status
        });
        let resp = self.client.put(&url).json(&body).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to update token: {}", resp.status()));
        }
        Ok(())
    }

    // --- Group Operations ---
    // Group routes are mounted at server root (/groups), not under /console/api.

    pub async fn list_groups(&self) -> Result<Vec<GroupDto>> {
        let url = format!("{}/groups", self.server_root());
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list groups: {}", resp.status()));
        }
        Ok(resp.json().await?)
    }

    pub async fn create_group(&self, group: &GroupDto) -> Result<()> {
        let url = format!("{}/groups", self.server_root());
        let resp = self.client.post(&url).json(group).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to create group: {}",
                resp.status()
            ));
        }
        Ok(())
    }

    pub async fn delete_group(&self, id: &str) -> Result<()> {
        let url = format!("{}/groups/{}", self.server_root(), id);
        let resp = self.client.delete(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to delete group: {}",
                resp.status()
            ));
        }
        Ok(())
    }

    pub async fn get_group_members(&self, group_id: &str) -> Result<Vec<GroupMemberDto>> {
        let url = format!("{}/groups/{}/members", self.server_root(), group_id);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to get group members: {}",
                resp.status()
            ));
        }
        Ok(resp.json().await?)
    }

    pub async fn update_group_members(
        &self,
        group_id: &str,
        members: &[GroupMemberDto],
    ) -> Result<()> {
        let url = format!("{}/groups/{}/members", self.server_root(), group_id);
        let resp = self.client.put(&url).json(members).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!(
                "Failed to update group members: {}",
                resp.status()
            ));
        }
        Ok(())
    }

    // --- Auth Operations ---

    pub async fn login(&self, username: &str, password: &str) -> Result<LoginResponse, String> {
        let url = format!("{}/api/user/login", self.base_url);
        let body = serde_json::json!({
            "username": username,
            "password": password
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let json: AuthResult = resp.json().await.map_err(|e| e.to_string())?;

        if json.success {
            json.data.ok_or_else(|| "No data received".to_string())
        } else {
            Err(json.message.unwrap_or_else(|| "Login failed".to_string()))
        }
    }

    pub async fn register(
        &self,
        username: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<LoginResponse, String> {
        let url = format!("{}/api/user/register", self.base_url);
        let body = serde_json::json!({
            "username": username,
            "password": password,
            "email": email
        });

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("网络错误: {}", e))?;

        let json: AuthResult = resp
            .json()
            .await
            .map_err(|e| format!("响应解析错误: {}", e))?;

        if json.success {
            json.data.ok_or_else(|| "注册成功但未返回用户数据".to_string())
        } else {
            Err(json.message.unwrap_or_else(|| "注册失败".to_string()))
        }
    }

    pub async fn check_username_availability(&self, username: &str) -> Result<bool, String> {
        let url = format!(
            "{}/api/user/check_username?username={}",
            self.base_url, username
        );

        let resp = self.client.get(&url).send().await.map_err(|e| e.to_string())?;

        let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

        if json["success"].as_bool().unwrap_or(false) {
            Ok(json["data"]["available"].as_bool().unwrap_or(false))
        } else {
            Err(json["message"].as_str().unwrap_or("检查失败").to_string())
        }
    }

    // --- Generic CRUD Operations ---
    // Used by StandardCrudPage for any RESTful endpoint.

    pub async fn crud_list(&self, endpoint: &str) -> Result<Vec<serde_json::Value>> {
        let url = format!("{}/api/{}", self.base_url, endpoint);
        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to list: {}", resp.status()));
        }
        Ok(resp.json().await?)
    }

    pub async fn crud_create(&self, endpoint: &str, data: &serde_json::Value) -> Result<()> {
        let url = format!("{}/api/{}", self.base_url, endpoint);
        let resp = self.client.post(&url).json(data).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to create: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn crud_update(
        &self,
        endpoint: &str,
        id: &str,
        data: &serde_json::Value,
    ) -> Result<()> {
        let url = format!("{}/api/{}/{}", self.base_url, endpoint, id);
        let resp = self.client.put(&url).json(data).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to update: {}", resp.status()));
        }
        Ok(())
    }

    pub async fn crud_delete(&self, endpoint: &str, id: &str) -> Result<()> {
        let url = format!("{}/api/{}/{}", self.base_url, endpoint, id);
        let resp = self.client.delete(&url).send().await?;

        if !resp.status().is_success() {
            return Err(anyhow::anyhow!("Failed to delete: {}", resp.status()));
        }
        Ok(())
    }
}
