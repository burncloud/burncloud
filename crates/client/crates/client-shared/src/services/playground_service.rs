use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::api_client::{
    ChatMessageRequest, ChatRequest, ChatUsage, RouteTrace, API_CLIENT,
};
use crate::services::token_service::TokenService;

// --- Public types ---

/// A single message in the playground conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaygroundMessage {
    pub role: String,
    pub content: String,
}

/// Configuration for a chat completion request.
#[derive(Debug, Clone)]
pub struct PlaygroundConfig {
    pub model: String,
    pub channel_id: Option<i64>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<i64>,
    pub stream: bool,
}

/// The result of a non-streaming chat completion.
#[derive(Debug, Clone)]
pub struct SendMessageResult {
    pub content: String,
    pub usage: ChatUsage,
    pub trace: RouteTrace,
}

/// Per-model pricing entry (USD per 1K tokens).
#[derive(Debug, Clone, Copy)]
struct ModelPricing {
    prompt_per_1k: f64,
    completion_per_1k: f64,
}

/// Fixed pricing table for cost estimation.
/// Rates are in USD per 1,000 tokens.
static PRICING_TABLE: &[(&str, ModelPricing)] = &[
    (
        "gpt-4o",
        ModelPricing {
            prompt_per_1k: 0.0025,
            completion_per_1k: 0.01,
        },
    ),
    (
        "gpt-4o-mini",
        ModelPricing {
            prompt_per_1k: 0.00015,
            completion_per_1k: 0.0006,
        },
    ),
    (
        "gpt-4-turbo",
        ModelPricing {
            prompt_per_1k: 0.01,
            completion_per_1k: 0.03,
        },
    ),
    (
        "gpt-3.5-turbo",
        ModelPricing {
            prompt_per_1k: 0.0005,
            completion_per_1k: 0.0015,
        },
    ),
    (
        "claude-sonnet-4-6",
        ModelPricing {
            prompt_per_1k: 0.003,
            completion_per_1k: 0.015,
        },
    ),
    (
        "claude-haiku-4-5",
        ModelPricing {
            prompt_per_1k: 0.0008,
            completion_per_1k: 0.004,
        },
    ),
    (
        "claude-opus-4-7",
        ModelPricing {
            prompt_per_1k: 0.015,
            completion_per_1k: 0.075,
        },
    ),
];

/// Default pricing for models not in the table.
const DEFAULT_PRICING: ModelPricing = ModelPricing {
    prompt_per_1k: 0.002,
    completion_per_1k: 0.008,
};

/// Export format for conversation history.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExportFormat {
    Json,
    Markdown,
}

/// Playground-specific error categories for user-facing messages.
#[derive(Debug, Clone, PartialEq)]
pub enum PlaygroundError {
    /// No API token available (user hasn't selected one or none exist).
    NoToken,
    /// The selected channel is unavailable or misconfigured.
    ChannelUnavailable(String),
    /// Quota exceeded (402/403 from backend).
    QuotaExceeded,
    /// Network or connection error.
    NetworkError(String),
    /// Upstream LLM provider error.
    UpstreamError(String),
    /// Stream interrupted mid-response.
    StreamInterrupted,
    /// Generic error.
    Other(String),
}

impl std::fmt::Display for PlaygroundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoToken => write!(f, "未选择 API Token，请先在配置中选择一个 Token"),
            Self::ChannelUnavailable(ch) => write!(f, "渠道不可用: {}", ch),
            Self::QuotaExceeded => write!(f, "Token 配额已用尽，请充值或更换 Token"),
            Self::NetworkError(msg) => write!(f, "网络错误: {}", msg),
            Self::UpstreamError(msg) => write!(f, "上游服务错误: {}", msg),
            Self::StreamInterrupted => write!(f, "流式响应中断"),
            Self::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<anyhow::Error> for PlaygroundError {
    fn from(err: anyhow::Error) -> Self {
        let msg = err.to_string();
        if msg.contains("402") || msg.contains("403") {
            Self::QuotaExceeded
        } else if msg.contains("channel") || msg.contains("Channel") {
            Self::ChannelUnavailable(msg)
        } else if msg.contains("stream failed") || msg.contains("connection") {
            Self::NetworkError(msg)
        } else {
            Self::Other(msg)
        }
    }
}

// --- PlaygroundService (stateless, Mode B) ---

/// Stateless service for playground chat operations.
/// All methods take explicit parameters — no internal mutable state.
pub struct PlaygroundService;

impl PlaygroundService {
    /// Send a non-streaming chat completion request.
    pub async fn send_message(
        messages: &[PlaygroundMessage],
        config: &PlaygroundConfig,
        bearer_token: &str,
    ) -> Result<SendMessageResult, PlaygroundError> {
        let request = Self::build_request(messages, config, false);
        let (response, trace) = API_CLIENT
            .chat_completions(&request, bearer_token)
            .await
            .map_err(Self::map_error)?;

        let content = response
            .choices
            .first()
            .and_then(|c| c.message.as_ref())
            .and_then(|m| m.content.clone())
            .unwrap_or_default();

        Ok(SendMessageResult {
            content,
            usage: response.usage,
            trace,
        })
    }

    /// Send a streaming chat completion request.
    /// `on_chunk` is called for each delta content fragment received.
    /// Returns final usage and route trace when the stream completes.
    pub async fn send_message_stream<F>(
        messages: &[PlaygroundMessage],
        config: &PlaygroundConfig,
        bearer_token: &str,
        on_chunk: F,
    ) -> Result<(ChatUsage, RouteTrace), PlaygroundError>
    where
        F: FnMut(&str),
    {
        let request = Self::build_request(messages, config, true);
        API_CLIENT
            .chat_completions_stream(&request, bearer_token, on_chunk)
            .await
            .map_err(Self::map_error)
    }

    /// Calculate cost in USD based on token usage and model pricing.
    pub fn calculate_cost(usage: &ChatUsage, model: &str) -> f64 {
        let pricing = PRICING_TABLE
            .iter()
            .find(|(name, _)| model.contains(name))
            .map(|(_, p)| p)
            .unwrap_or(&DEFAULT_PRICING);

        let prompt_cost = (usage.prompt_tokens as f64 / 1000.0) * pricing.prompt_per_1k;
        let completion_cost = (usage.completion_tokens as f64 / 1000.0) * pricing.completion_per_1k;
        prompt_cost + completion_cost
    }

    /// Export conversation history to the specified format.
    pub fn export_conversation(
        messages: &[PlaygroundMessage],
        format: ExportFormat,
    ) -> String {
        match format {
            ExportFormat::Json => serde_json::to_string_pretty(messages).unwrap_or_default(),
            ExportFormat::Markdown => {
                let mut md = String::from("# 对话记录\n\n");
                for msg in messages {
                    let label = match msg.role.as_str() {
                        "user" => "👤 用户",
                        "assistant" => "🤖 助手",
                        "system" => "⚙️ 系统",
                        _ => &msg.role,
                    };
                    md.push_str(&format!("### {}\n\n{}\n\n---\n\n", label, msg.content));
                }
                md
            }
        }
    }

    /// Resolve a usable bearer token. Returns the token string if one is available.
    pub async fn resolve_token() -> Option<String> {
        let tokens = TokenService::list().await.ok()?;
        tokens.into_iter().find(|t| t.status == "active").map(|t| t.token)
    }

    // --- Private helpers ---

    fn build_request(
        messages: &[PlaygroundMessage],
        config: &PlaygroundConfig,
        stream: bool,
    ) -> ChatRequest {
        ChatRequest {
            model: config.model.clone(),
            messages: messages
                .iter()
                .map(|m| ChatMessageRequest {
                    role: m.role.clone(),
                    content: m.content.clone(),
                })
                .collect(),
            stream: Some(stream),
            temperature: config.temperature,
            max_tokens: config.max_tokens,
        }
    }

    fn map_error(err: anyhow::Error) -> PlaygroundError {
        let msg = err.to_string();
        if msg.contains("402") || msg.contains("403") {
            PlaygroundError::QuotaExceeded
        } else if msg.contains("401") {
            PlaygroundError::NoToken
        } else if msg.contains("502") || msg.contains("503") || msg.contains("504") {
            PlaygroundError::UpstreamError(msg)
        } else if msg.contains("stream failed")
            || msg.contains("connection")
            || msg.contains("timed out")
        {
            PlaygroundError::NetworkError(msg)
        } else {
            PlaygroundError::Other(msg)
        }
    }
}
