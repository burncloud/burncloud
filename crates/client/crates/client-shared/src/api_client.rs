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
