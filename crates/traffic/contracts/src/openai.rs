// Two struct fields legitimately need HashMap<String, Value>:
//   OpenAIChatRequest::extra      — generic LLM API passthrough fields
// All other Value uses in this codebase must go through typed structs.
#![allow(
    clippy::disallowed_types,
    reason = "OpenAI passthrough must preserve provider-specific JSON fields in the flattened extra map"
)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// A single message in an OpenAI-compatible chat request.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChatMessage {
    pub role: String,
    pub content: String,
}

/// An OpenAI-compatible chat completion request.
///
/// `extra` uses `#[serde(flatten)]` so that provider-specific fields survive the round trip
/// (generic passthrough). Losing that attribute would silently drop unknown fields.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChatRequest {
    pub model: String,
    pub messages: Vec<OpenAIChatMessage>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub max_tokens: Option<u32>,
    #[serde(default)]
    pub stream: bool,

    // Capture all other fields (Generic Passthrough)
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// An OpenAI-compatible chat completion response.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<OpenAIChatChoice>,
}

/// One choice inside an [`OpenAIChatResponse`].
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OpenAIChatChoice {
    pub index: u32,
    pub message: OpenAIChatMessage,
    pub finish_reason: Option<String>,
}
