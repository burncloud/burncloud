use burncloud_common::types::OpenAIChatRequest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// Claude Specific Types
#[derive(Serialize, Deserialize, Debug)]
pub struct ClaudeRequest {
    pub model: String,
    pub messages: Vec<ClaudeMessage>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClaudeMessage {
    pub role: String,
    pub content: String,
}

pub struct ClaudeAdaptor;

impl ClaudeAdaptor {
    pub fn convert_request(req: OpenAIChatRequest) -> Value {
        let mut system_prompt = None;
        let mut claude_messages = Vec::new();

        for msg in req.messages {
            if msg.role == "system" {
                system_prompt = Some(msg.content);
            } else {
                // Map "user"/"assistant" -> "user"/"assistant" (Same)
                claude_messages.push(ClaudeMessage {
                    role: msg.role,
                    content: msg.content,
                });
            }
        }

        let claude_req = ClaudeRequest {
            model: req.model, // Often needs mapping, but we'll pass through for now
            messages: claude_messages,
            max_tokens: req.max_tokens.unwrap_or(4096),
            system: system_prompt,
            temperature: req.temperature,
        };

        json!(claude_req)
    }

    pub fn convert_response(claude_resp: Value, model: &str) -> Value {
        // Claude Response: { "content": [ { "text": "..." } ], ... }
        let text = claude_resp.get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
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
    fn test_openai_to_claude_request() {
        let req = OpenAIChatRequest {
            model: "gpt-4".to_string(),
            messages: vec![
                OpenAIChatMessage { role: "system".to_string(), content: "Be helpful".to_string() },
                OpenAIChatMessage { role: "user".to_string(), content: "Hi".to_string() }
            ],
            temperature: Some(0.5),
            max_tokens: Some(200),
            stream: false,
        };

        let claude_val = ClaudeAdaptor::convert_request(req);
        
        // Validate extraction of system prompt
        assert_eq!(claude_val["system"], "Be helpful");
        // Validate messages (system should be removed from messages array)
        let messages = claude_val["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
        assert_eq!(messages[0]["content"], "Hi");
        assert_eq!(claude_val["max_tokens"], 200);
    }

    #[test]
    fn test_claude_to_openai_response() {
        let claude_resp = json!({
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "content": [
                {
                    "type": "text",
                    "text": "Hello from Claude"
                }
            ],
            "model": "claude-3-opus",
            "stop_reason": "end_turn",
            "usage": { "input_tokens": 10, "output_tokens": 20 }
        });

        let openai_val = ClaudeAdaptor::convert_response(claude_resp, "claude-3");

        assert_eq!(openai_val["choices"][0]["message"]["content"], "Hello from Claude");
        assert_eq!(openai_val["model"], "claude-3");
    }
}
