use burncloud_common::types::OpenAIChatRequest;
use serde::Serialize;
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
#[serde(rename_all = "camelCase")] // This handles generationConfig -> generation_config automatically if we use snake_case in struct? No, rename_all works on fields.
                                   // Wait, Gemini API expects camelCase "generationConfig".
                                   // Rust standard is snake_case "generation_config".
                                   // Serde can map this.
pub struct GeminiRequest {
    pub contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
}

pub struct GeminiAdaptor;

impl GeminiAdaptor {
    pub fn convert_request(req: OpenAIChatRequest) -> Value {
        let contents: Vec<GeminiContent> = req
            .messages
            .into_iter()
            .map(|msg| {
                // Gemini roles: "user" or "model" (OpenAI "assistant" -> "model")
                let role = if msg.role == "assistant" {
                    "model"
                } else {
                    "user"
                };
                GeminiContent {
                    role: role.to_string(),
                    parts: vec![GeminiPart { text: msg.content }],
                }
            })
            .collect();

        let config = GeminiGenerationConfig {
            temperature: req.temperature,
            max_output_tokens: req.max_tokens,
        };

        let gemini_req = GeminiRequest {
            contents,
            generation_config: Some(config),
        };

        json!(gemini_req)
    }

    // Convert Gemini Response JSON -> OpenAI Response JSON
    // NOTE: This handles non-streaming response only for now.
    pub fn convert_response(gemini_resp: Value, model: &str) -> Value {
        // TODO: Robust error handling if gemini_resp is error
        let candidate = gemini_resp.get("candidates").and_then(|c| c.get(0));

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

#[cfg(test)]
mod tests {
    use super::*;
    use burncloud_common::types::{OpenAIChatMessage, OpenAIChatRequest};
    use serde_json::json;

    #[test]
    fn test_openai_to_gemini_request() {
        let req = OpenAIChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![OpenAIChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: Some(0.5),
            max_tokens: Some(100),
            stream: false,
            extra: std::collections::HashMap::new(),
        };

        let gemini_val = GeminiAdaptor::convert_request(req);

        // Validate structure
        assert_eq!(gemini_val["contents"][0]["role"], "user");
        assert_eq!(gemini_val["contents"][0]["parts"][0]["text"], "Hello");
        assert_eq!(gemini_val["generationConfig"]["temperature"], 0.5);
        assert_eq!(gemini_val["generationConfig"]["maxOutputTokens"], 100);
    }

    #[test]
    fn test_gemini_to_openai_response() {
        let gemini_resp = json!({
            "candidates": [
                {
                    "content": {
                        "parts": [ { "text": "Hi there!" } ],
                        "role": "model"
                    },
                    "finishReason": "STOP",
                    "index": 0
                }
            ]
        });

        let openai_val = GeminiAdaptor::convert_response(gemini_resp, "gemini-pro");

        // Validate structure
        assert_eq!(openai_val["object"], "chat.completion");
        assert_eq!(openai_val["model"], "gemini-pro");
        assert_eq!(openai_val["choices"][0]["message"]["content"], "Hi there!");
        assert_eq!(openai_val["choices"][0]["message"]["role"], "assistant");
    }
}
