use burncloud_common::types::{OpenAIChatRequest, OpenAIChatMessage};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Gemini Specific Types
#[derive(Serialize, Debug)]
pub struct GeminiContent {
    pub role: String,
    pub parts: Vec<GeminiPart>,
}

#[derive(Serialize, Debug)]
pub struct GeminiPart {
    pub text: String,
}

#[derive(Serialize, Debug)]
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generationConfig: Option<GeminiGenerationConfig>,
}

#[derive(Serialize, Debug)]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maxOutputTokens: Option<u32>,
}

pub struct GeminiAdaptor;

impl GeminiAdaptor {
    pub fn convert_request(req: OpenAIChatRequest) -> Value {
        let contents: Vec<GeminiContent> = req.messages.into_iter().map(|msg| {
            // Gemini roles: "user" or "model" (OpenAI "assistant" -> "model")
            let role = if msg.role == "assistant" { "model" } else { "user" };
            GeminiContent {
                role: role.to_string(),
                parts: vec![GeminiPart { text: msg.content }],
            }
        }).collect();

        let config = GeminiGenerationConfig {
            temperature: req.temperature,
            maxOutputTokens: req.max_tokens,
        };

        let gemini_req = GeminiRequest {
            contents,
            generationConfig: Some(config),
        };

        json!(gemini_req)
    }

    // Convert Gemini Response JSON -> OpenAI Response JSON
    // NOTE: This handles non-streaming response only for now.
    pub fn convert_response(gemini_resp: Value, model: &str) -> Value {
        // TODO: Robust error handling if gemini_resp is error
        let candidate = gemini_resp.get("candidates")
            .and_then(|c| c.get(0));
        
        let text = candidate
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        json!({
            "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
            "object": "chat.completion",
            "created": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
            "model": model,
            "choices": [
                {
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": text
                    },
                    "finish_reason": "stop"
                }
            ]
        })
    }
}
