// HTTP client — raw API response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use anyhow::Result;
use futures_util::StreamExt;
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

// --- DTOs: Channel / Token (management API) ---

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

// --- DTOs: Chat Completions (data-plane API) ---

#[derive(Serialize, Debug, Clone)]
pub struct ChatMessageRequest {
    pub role: String,
    pub content: String,
}

#[derive(Serialize, Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessageRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<i64>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatChoice {
    pub index: Option<i64>,
    pub message: Option<ChatChoiceMessage>,
    pub delta: Option<ChatChoiceDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatChoiceMessage {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatChoiceDelta {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Copy, Default, PartialEq)]
pub struct ChatUsage {
    #[serde(default)]
    pub prompt_tokens: i64,
    #[serde(default)]
    pub completion_tokens: i64,
    #[serde(default)]
    pub total_tokens: i64,
}

#[derive(Deserialize, Debug, Clone)]
pub struct ChatResponse {
    pub id: Option<String>,
    pub object: Option<String>,
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub usage: ChatUsage,
    pub model: Option<String>,
}

/// Route-tracing metadata extracted from response headers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RouteTrace {
    pub channel_id: Option<String>,
    pub model_id: Option<String>,
}

/// A parsed SSE chunk from a streaming chat completion response.
#[derive(Deserialize, Debug, Clone)]
pub struct ChatChunk {
    pub id: Option<String>,
    pub object: Option<String>,
    pub choices: Vec<ChatChoice>,
    #[serde(default)]
    pub usage: ChatUsage,
}

impl ChatChunk {
    /// Extract the delta content from the first choice, if present.
    pub fn delta_content(&self) -> Option<String> {
        self.choices.first().and_then(|c| {
            c.delta
                .as_ref()
                .and_then(|d| d.content.clone())
        })
    }

    /// Check if this is the [DONE] sentinel.
    pub fn is_done(text: &str) -> bool {
        text.trim() == "data: [DONE]" || text.trim() == "[DONE]"
    }
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

    /// Build the data-plane URL for chat completions (no /console prefix).
    fn chat_completions_url(&self) -> String {
        let port = burncloud_common::constants::DEFAULT_PORT;
        format!("http://localhost:{}/v1/chat/completions", port)
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

    // --- Chat Completions (Data Plane) ---

    /// Non-streaming chat completion. Returns the full response plus route-tracing headers.
    pub async fn chat_completions(
        &self,
        request: &ChatRequest,
        bearer_token: &str,
    ) -> Result<(ChatResponse, RouteTrace)> {
        let mut req = request.clone();
        req.stream = Some(false);

        let resp = self
            .client
            .post(self.chat_completions_url())
            .header("Authorization", format!("Bearer {}", bearer_token))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Chat completions failed ({}): {}",
                status,
                body
            ));
        }

        let trace = RouteTrace {
            channel_id: resp
                .headers()
                .get("X-Channel-Id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            model_id: resp
                .headers()
                .get("X-Model-Id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        };

        let chat_resp = resp.json::<ChatResponse>().await?;
        Ok((chat_resp, trace))
    }

    /// Streaming chat completion. Returns SSE chunks via a callback, plus route-tracing headers.
    ///
    /// The `on_chunk` callback is invoked for each parsed SSE chunk (delta content).
    /// The `on_done` callback is invoked when the stream ends, with the final usage and route trace.
    pub async fn chat_completions_stream<F>(
        &self,
        request: &ChatRequest,
        bearer_token: &str,
        mut on_chunk: F,
    ) -> Result<(ChatUsage, RouteTrace)>
    where
        F: FnMut(&str),
    {
        let mut req = request.clone();
        req.stream = Some(true);

        let resp = self
            .client
            .post(self.chat_completions_url())
            .header("Authorization", format!("Bearer {}", bearer_token))
            .header("Content-Type", "application/json")
            .json(&req)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(anyhow::anyhow!(
                "Chat completions stream failed ({}): {}",
                status,
                body
            ));
        }

        let trace = RouteTrace {
            channel_id: resp
                .headers()
                .get("X-Channel-Id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            model_id: resp
                .headers()
                .get("X-Model-Id")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        };

        let mut stream = resp.bytes_stream();
        let mut usage = ChatUsage::default();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            let bytes = chunk_result?;
            buffer.push_str(&String::from_utf8_lossy(&bytes));

            // Process complete SSE lines from the buffer
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].to_string();
                buffer = buffer[pos + 1..].to_string();
                let line = line.trim();

                if line.is_empty() || line.starts_with(':') {
                    continue;
                }

                if ChatChunk::is_done(line) {
                    return Ok((usage, trace));
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    if let Ok(chunk) = serde_json::from_str::<ChatChunk>(data) {
                        // Accumulate usage from the final chunk
                        if chunk.usage.total_tokens > 0 {
                            usage = chunk.usage;
                        }
                        if let Some(content) = chunk.delta_content() {
                            on_chunk(&content);
                        }
                    }
                }
            }
        }

        Ok((usage, trace))
    }
}
