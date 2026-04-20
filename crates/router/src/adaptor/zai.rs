// LLM protocol adaptor — dynamic JSON transformation — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

//! z.ai Adaptor
//!
//! z.ai uses an Anthropic-compatible API with Bearer authentication.
//! This adaptor combines Anthropic protocol conversion with Bearer auth.

use super::current_unix_timestamp;
use burncloud_common::types::OpenAIChatRequest;
use serde_json::{json, Value};

pub struct ZaiAdaptor;

impl ZaiAdaptor {
    /// Convert OpenAI Chat request to z.ai (Anthropic-compatible) request
    pub fn convert_request(req: OpenAIChatRequest) -> Value {
        let mut system_prompt = None;
        let mut messages = Vec::new();

        for msg in req.messages {
            if msg.role == "system" {
                system_prompt = Some(msg.content);
            } else {
                messages.push(json!({
                    "role": msg.role,
                    "content": msg.content
                }));
            }
        }

        let mut body = json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": req.max_tokens.unwrap_or(4096),
        });

        if let Some(system) = system_prompt {
            body["system"] = json!(system);
        }

        if let Some(temp) = req.temperature {
            body["temperature"] = json!(temp);
        }

        // Pass stream flag for streaming requests
        if req.stream {
            body["stream"] = json!(true);
        }

        body
    }

    /// Convert z.ai (Anthropic-compatible) response to OpenAI format
    pub fn convert_response(zai_resp: Value, model: &str) -> Value {
        // Extract text from content array
        let text = zai_resp
            .get("content")
            .and_then(|c| c.as_array())
            .and_then(|arr| arr.first())
            .and_then(|item| item.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("");

        // Extract usage if available
        let usage = zai_resp.get("usage").map(|u| {
            json!({
                "prompt_tokens": u.get("input_tokens").and_then(|t| t.as_i64()).unwrap_or(0),
                "completion_tokens": u.get("output_tokens").and_then(|t| t.as_i64()).unwrap_or(0),
                "total_tokens": u.get("input_tokens").and_then(|i| i.as_i64()).unwrap_or(0)
                    + u.get("output_tokens").and_then(|o| o.as_i64()).unwrap_or(0)
            })
        });

        json!({
            "id": zai_resp.get("id").unwrap_or(&json!("")).clone(),
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
            ],
            "usage": usage
        })
    }

    /// Convert z.ai streaming chunk to OpenAI SSE format
    ///
    /// z.ai uses Anthropic-style SSE format with separate event and data lines:
    /// ```text
    /// event: content_block_delta
    /// data: {"type": "content_block_delta", ...}
    ///
    /// ```
    /// This function handles both single-line and multi-line chunks.
    pub fn convert_stream_chunk(chunk: &str, model: &str) -> Option<String> {
        let mut current_event: Option<&str> = None;

        // Process each line in the chunk
        for line in chunk.lines() {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Track event type from event: line
            if let Some(event) = line.strip_prefix("event:") {
                current_event = Some(event.trim());
                continue;
            }

            // Process data lines
            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    return Some("data: [DONE]\n\n".to_string());
                }

                // Check if this is a stop event based on event: line or data content
                if current_event == Some("message_stop")
                    || current_event == Some("content_block_stop")
                {
                    return Some("data: [DONE]\n\n".to_string());
                }

                if let Ok(json) = serde_json::from_str::<Value>(data) {
                    // Handle different event types from z.ai
                    // Use type from JSON data, or fall back to event: line
                    let event_type = json.get("type").and_then(|t| t.as_str()).or(current_event);

                    if let Some(event_type) = event_type {
                        match event_type {
                            "content_block_delta" => {
                                // Extract text from delta.text_delta.text format
                                let delta = json
                                    .get("delta")
                                    .and_then(|d| d.get("text"))
                                    .and_then(|t| t.as_str())
                                    .unwrap_or("");
                                return Some(format!(
                                    "data: {}\n\n",
                                    json!({
                                        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4()),
                                        "object": "chat.completion.chunk",
                                        "created": current_unix_timestamp(),
                                        "model": model,
                                        "choices": [{
                                            "index": 0,
                                            "delta": {"content": delta},
                                            "finish_reason": null
                                        }]
                                    })
                                ));
                            }
                            "message_stop" | "content_block_stop" => {
                                return Some("data: [DONE]\n\n".to_string());
                            }
                            // Skip other event types (message_start, content_block_start, etc.)
                            _ => {}
                        }
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_types, clippy::unnecessary_cast, clippy::let_and_return, clippy::redundant_pattern_matching)]
mod tests {
    use super::*;
    use burncloud_common::types::{OpenAIChatMessage, OpenAIChatRequest};
    use std::collections::HashMap;

    #[test]
    fn test_openai_to_zai_request() {
        let req = OpenAIChatRequest {
            model: "glm-5".to_string(),
            messages: vec![
                OpenAIChatMessage {
                    role: "system".to_string(),
                    content: "Be helpful".to_string(),
                },
                OpenAIChatMessage {
                    role: "user".to_string(),
                    content: "Hello".to_string(),
                },
            ],
            temperature: Some(0.7),
            max_tokens: Some(100),
            stream: false,
            extra: HashMap::new(),
        };

        let zai_req = ZaiAdaptor::convert_request(req);

        assert_eq!(zai_req["model"], "glm-5");
        assert_eq!(zai_req["system"], "Be helpful");
        assert_eq!(zai_req["max_tokens"], 100);
        let messages = zai_req["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
    }

    #[test]
    fn test_zai_to_openai_response() {
        let zai_resp = json!({
            "id": "msg_202603030217431209324c1d944dd5",
            "type": "message",
            "role": "assistant",
            "model": "glm-5",
            "content": [{
                "type": "text",
                "text": "Hello! How can I help you today?"
            }],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 7,
                "output_tokens": 10
            }
        });

        let openai_resp = ZaiAdaptor::convert_response(zai_resp, "glm-5");

        assert_eq!(
            openai_resp["choices"][0]["message"]["content"],
            "Hello! How can I help you today?"
        );
        assert_eq!(openai_resp["model"], "glm-5");
        assert_eq!(openai_resp["usage"]["prompt_tokens"], 7);
        assert_eq!(openai_resp["usage"]["completion_tokens"], 10);
    }

    #[test]
    fn test_stream_request_includes_stream_flag() {
        let req = OpenAIChatRequest {
            model: "glm-5".to_string(),
            messages: vec![OpenAIChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            temperature: None,
            max_tokens: Some(100),
            stream: true,
            extra: HashMap::new(),
        };

        let zai_req = ZaiAdaptor::convert_request(req);
        assert_eq!(zai_req["stream"], true);
    }

    #[test]
    fn test_convert_stream_chunk_with_event_line() {
        // Test chunk with event and data on separate lines
        let chunk = "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n";
        let result = ZaiAdaptor::convert_stream_chunk(chunk, "glm-5");

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.starts_with("data: "));
        assert!(output.contains("\"delta\":{\"content\":\"Hello\"}"));
    }

    #[test]
    fn test_convert_stream_chunk_data_only() {
        // Test chunk with only data line
        let chunk = "data: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"World\"}}\n";
        let result = ZaiAdaptor::convert_stream_chunk(chunk, "glm-5");

        assert!(result.is_some());
        let output = result.unwrap();
        assert!(output.contains("\"delta\":{\"content\":\"World\"}"));
    }

    #[test]
    fn test_convert_stream_chunk_message_stop() {
        let chunk = "event: message_stop\ndata: {}\n";
        let result = ZaiAdaptor::convert_stream_chunk(chunk, "glm-5");

        assert_eq!(result, Some("data: [DONE]\n\n".to_string()));
    }

    #[test]
    fn test_convert_stream_chunk_skip_event_line() {
        // Event-only line should return None
        let chunk = "event: message_start\n";
        let result = ZaiAdaptor::convert_stream_chunk(chunk, "glm-5");

        assert!(result.is_none());
    }
}
