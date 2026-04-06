use crate::error::ParseError;
use crate::types::UnifiedUsage;
use crate::usage::UsageParser;
use serde_json::Value;

/// Usage parser for Anthropic (Claude) APIs.
///
/// Non-streaming: `response["usage"]["input_tokens"]` / `["output_tokens"]`
/// Streaming — two events:
///   - `message_start`: `{"type":"message_start","message":{"usage":{"input_tokens":N}}}`
///   - `message_delta`: `{"type":"message_delta","usage":{"output_tokens":M}}`
/// Cache tokens (Prompt Caching):
///   - `usage["cache_read_input_tokens"]`
///   - `usage["cache_creation_input_tokens"]`
pub struct AnthropicParser;

impl UsageParser for AnthropicParser {
    fn provider_name(&self) -> &'static str {
        "anthropic"
    }

    fn parse_response(&self, response: &Value) -> Result<UnifiedUsage, ParseError> {
        let Some(usage) = response.get("usage") else {
            return Ok(UnifiedUsage::default());
        };

        Ok(UnifiedUsage {
            input_tokens: usage
                .get("input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            output_tokens: usage
                .get("output_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            cache_read_tokens: usage
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            cache_write_tokens: usage
                .get("cache_creation_input_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            ..Default::default()
        })
    }

    /// Parses a single SSE line from an Anthropic streaming response.
    /// Returns `Some(UnifiedUsage)` for `message_start` and `message_delta` events,
    /// `None` for all other chunk types.
    fn parse_streaming_chunk(&self, chunk: &str) -> Result<Option<UnifiedUsage>, ParseError> {
        let line = chunk.trim();
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        let json: Value = serde_json::from_str(data)?;

        let event_type = json.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match event_type {
            "message_start" => {
                let usage = json.get("message").and_then(|m| m.get("usage"));
                let Some(usage) = usage else {
                    return Ok(None);
                };
                Ok(Some(UnifiedUsage {
                    input_tokens: usage
                        .get("input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    cache_read_tokens: usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    cache_write_tokens: usage
                        .get("cache_creation_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    ..Default::default()
                }))
            }
            "message_delta" => {
                let usage = json.get("usage");
                let Some(usage) = usage else {
                    return Ok(None);
                };
                Ok(Some(UnifiedUsage {
                    output_tokens: usage
                        .get("output_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    // Some delta events also carry updated cache counts
                    cache_read_tokens: usage
                        .get("cache_read_input_tokens")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0),
                    ..Default::default()
                }))
            }
            _ => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_response_basic() {
        let parser = AnthropicParser;
        let resp = json!({"usage": {"input_tokens": 50, "output_tokens": 75}});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 50);
        assert_eq!(u.output_tokens, 75);
    }

    #[test]
    fn test_parse_response_with_cache() {
        let parser = AnthropicParser;
        let resp = json!({
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50,
                "cache_read_input_tokens": 30,
                "cache_creation_input_tokens": 20
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.cache_read_tokens, 30);
        assert_eq!(u.cache_write_tokens, 20);
    }

    #[test]
    fn test_parse_response_missing_usage() {
        let parser = AnthropicParser;
        let u = parser.parse_response(&json!({"content": []})).unwrap();
        assert!(u.is_empty());
    }

    #[test]
    fn test_parse_streaming_message_start() {
        let parser = AnthropicParser;
        let line = r#"data: {"type":"message_start","message":{"id":"msg_1","usage":{"input_tokens":50}}}"#;
        let u = parser.parse_streaming_chunk(line).unwrap().unwrap();
        assert_eq!(u.input_tokens, 50);
        assert_eq!(u.output_tokens, 0);
    }

    #[test]
    fn test_parse_streaming_message_delta() {
        let parser = AnthropicParser;
        let line = r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":75}}"#;
        let u = parser.parse_streaming_chunk(line).unwrap().unwrap();
        assert_eq!(u.output_tokens, 75);
    }

    #[test]
    fn test_parse_streaming_message_start_no_usage() {
        let parser = AnthropicParser;
        // message_start without usage block → None
        let line = r#"data: {"type":"message_start","message":{"id":"msg_1"}}"#;
        assert!(parser.parse_streaming_chunk(line).unwrap().is_none());
    }

    #[test]
    fn test_parse_streaming_content_block_ignored() {
        let parser = AnthropicParser;
        let line =
            r#"data: {"type":"content_block_delta","delta":{"type":"text_delta","text":"hi"}}"#;
        assert!(parser.parse_streaming_chunk(line).unwrap().is_none());
    }

    #[test]
    fn test_parse_streaming_cache_in_message_start() {
        let parser = AnthropicParser;
        let line = r#"data: {"type":"message_start","message":{"usage":{"input_tokens":100,"cache_read_input_tokens":50,"cache_creation_input_tokens":20}}}"#;
        let u = parser.parse_streaming_chunk(line).unwrap().unwrap();
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.cache_read_tokens, 50);
        assert_eq!(u.cache_write_tokens, 20);
    }
}
