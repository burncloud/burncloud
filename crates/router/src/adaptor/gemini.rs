// LLM protocol adaptor — dynamic JSON transformation — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use super::current_unix_timestamp;
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
    pub fn convert_response(mut gemini_resp: Value, model: &str) -> Value {
        // Handle Array (from streamGenerateContent)
        if gemini_resp.is_array() {
            if let Value::Array(mut arr) = gemini_resp {
                gemini_resp = if !arr.is_empty() {
                    arr.remove(0)
                } else {
                    Value::Null
                };
            }
        }

        // Check for Gemini error response
        if let Some(error) = gemini_resp.get("error") {
            let message = error
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Gemini API error");
            let status = error
                .get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("error");
            let code = error.get("code").and_then(|c| c.as_i64()).unwrap_or(500);

            return json!({
                "error": {
                    "message": message,
                    "type": status,
                    "code": code
                }
            });
        }

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
            "created": current_unix_timestamp(),
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

    pub fn convert_stream_response(chunk: &str) -> Option<String> {
        // Handle SSE format from Gemini API: "data: {...}"
        // Strip the "data: " prefix if present
        let chunk = chunk.trim();
        let clean_chunk = chunk.strip_prefix("data: ").unwrap_or(chunk);

        // Handle array format "[{...}," or ",{...}]" which happens in some stream outputs
        let clean_chunk = clean_chunk
            .trim()
            .trim_start_matches('[')
            .trim_start_matches(',')
            .trim_end_matches(',')
            .trim_end_matches(']');
        if clean_chunk.is_empty() {
            return None;
        }

        let root: Value = match serde_json::from_str(clean_chunk) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let candidate = root.get("candidates").and_then(|c| c.get(0));

        let text = candidate
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str());

        let finish_reason = candidate
            .and_then(|c| c.get("finishReason"))
            .and_then(|s| s.as_str());

        let openai_finish_reason = match finish_reason {
            Some("STOP") => Some("stop"),
            Some("MAX_TOKENS") => Some("length"),
            Some("SAFETY") => Some("content_filter"),
            Some(_) => Some("stop"),
            None => None,
        };

        // Extract usageMetadata from Gemini response (sent in final chunk)
        let usage_metadata = root.get("usageMetadata");
        let usage = usage_metadata.map(|m| {
            let candidates = m
                .get("candidatesTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let thoughts = m
                .get("thoughtsTokenCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            json!({
                "prompt_tokens": m.get("promptTokenCount").and_then(|v| v.as_u64()).unwrap_or(0),
                "completion_tokens": candidates + thoughts,
                "total_tokens": m.get("totalTokenCount").and_then(|v| v.as_u64()).unwrap_or(0)
            })
        });

        // Skip if no content and no usage (truly empty chunk)
        if text.is_none() && openai_finish_reason.is_none() && usage.is_none() {
            return None;
        }

        // Build response - include usage in final chunk (when finish_reason is present)
        let mut chunk_json = json!({
            "id": "chatcmpl-stream",
            "object": "chat.completion.chunk",
            "created": current_unix_timestamp(),
            "model": "gemini-model",
            "choices": [
                {
                    "index": 0,
                    "delta": {
                        "content": text
                    },
                    "finish_reason": openai_finish_reason
                }
            ]
        });

        // Include usage stats in the final chunk (OpenAI spec: usage in final chunk before [DONE])
        if let Some(usage_value) = usage {
            chunk_json["usage"] = usage_value;
        }

        Some(format!("data: {}\n\n", chunk_json))
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::disallowed_types,
    clippy::unnecessary_cast,
    clippy::let_and_return,
    clippy::redundant_pattern_matching
)]
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

    #[test]
    fn test_gemini_error_response() {
        let gemini_error = json!({
            "error": {
                "code": 400,
                "message": "API key not valid",
                "status": "INVALID_ARGUMENT"
            }
        });

        let openai_val = GeminiAdaptor::convert_response(gemini_error, "gemini-pro");

        assert!(openai_val.get("error").is_some());
        assert_eq!(openai_val["error"]["message"], "API key not valid");
        assert_eq!(openai_val["error"]["type"], "INVALID_ARGUMENT");
        assert_eq!(openai_val["error"]["code"], 400);
    }

    #[test]
    fn test_convert_stream_response() {
        let chunk = r#"[{"candidates": [{"content": {"parts": [{"text": "Hello stream"}]}, "finishReason": null, "index": 0}]}]"#;

        let sse = GeminiAdaptor::convert_stream_response(chunk).unwrap();
        assert!(sse.starts_with("data: "));
        assert!(sse.contains("Hello stream"));
        assert!(sse.contains(r#""finish_reason":null"#));

        // Test dirty chunk with STOP
        let dirty_chunk = r#",{"candidates": [{"finishReason": "STOP", "index": 0}]},"#;
        let sse2 = GeminiAdaptor::convert_stream_response(dirty_chunk).unwrap();
        assert!(sse2.contains(r#""finish_reason":"stop""#));

        // Test invalid/empty
        assert!(GeminiAdaptor::convert_stream_response("").is_none());
        assert!(GeminiAdaptor::convert_stream_response("[]").is_none());

        // Test usageMetadata extraction (sent in final chunk)
        let usage_chunk = r#"{"candidates": [{"finishReason": "STOP", "index": 0}], "usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 25, "totalTokenCount": 35}}"#;
        let sse3 = GeminiAdaptor::convert_stream_response(usage_chunk).unwrap();
        assert!(sse3.contains(r#""prompt_tokens":10"#));
        assert!(sse3.contains(r#""completion_tokens":25"#));
        assert!(sse3.contains(r#""total_tokens":35"#));
        assert!(sse3.contains(r#""finish_reason":"stop""#));

        // Test chunk with content and usageMetadata together
        let full_chunk = r#"{"candidates": [{"content": {"parts": [{"text": "Hello"}]}, "finishReason": "STOP", "index": 0}], "usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 10, "totalTokenCount": 15}}"#;
        let sse4 = GeminiAdaptor::convert_stream_response(full_chunk).unwrap();
        assert!(sse4.contains("Hello"));
        assert!(sse4.contains(r#""prompt_tokens":5"#));
        assert!(sse4.contains(r#""completion_tokens":10"#));
    }
}
