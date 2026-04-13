// LLM provider response parsing requires Value — all providers implement
// UsageParser whose trait method signature uses `&Value`.
#![allow(clippy::disallowed_types)]

use crate::error::ParseError;
use crate::types::UnifiedUsage;
use crate::usage::UsageParser;
use serde_json::Value;

/// Usage parser for OpenAI and OpenAI-compatible APIs.
/// Handles both streaming SSE and non-streaming responses.
///
/// Non-streaming format: `response["usage"]["prompt_tokens"]`
/// Streaming format: `data: {"usage": {"prompt_tokens": N, "completion_tokens": M}}`
/// Cache tokens: `usage["prompt_tokens_details"]["cached_tokens"]`
pub struct OpenAIParser;

impl UsageParser for OpenAIParser {
    fn provider_name(&self) -> &'static str {
        "openai"
    }

    fn parse_response(&self, response: &Value) -> Result<UnifiedUsage, ParseError> {
        let Some(usage) = response.get("usage") else {
            return Ok(UnifiedUsage::default());
        };

        let mut u = UnifiedUsage {
            input_tokens: usage
                .get("prompt_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            output_tokens: usage
                .get("completion_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            ..Default::default()
        };

        // Prompt Caching
        if let Some(details) = usage.get("prompt_tokens_details") {
            u.cache_read_tokens = details
                .get("cached_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
        }

        // Audio tokens
        if let Some(details) = usage.get("prompt_tokens_details") {
            u.audio_input_tokens = details
                .get("audio_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
        }
        if let Some(details) = usage.get("completion_tokens_details") {
            u.audio_output_tokens = details
                .get("audio_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
        }

        Ok(u)
    }

    fn parse_streaming_chunk(&self, chunk: &str) -> Result<Option<UnifiedUsage>, ParseError> {
        let line = chunk.trim();
        if !line.starts_with("data: ") {
            return Ok(None);
        }
        let data = &line[6..];
        if data.trim() == "[DONE]" {
            return Ok(None);
        }

        let json: Value = serde_json::from_str(data)?;
        let Some(usage) = json.get("usage") else {
            return Ok(None);
        };

        // Only process chunks that carry usage info
        let input = usage
            .get("prompt_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let output = usage
            .get("completion_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        if input == 0 && output == 0 {
            return Ok(None);
        }

        let mut u = UnifiedUsage {
            input_tokens: input,
            output_tokens: output,
            ..Default::default()
        };

        if let Some(details) = usage.get("prompt_tokens_details") {
            u.cache_read_tokens = details
                .get("cached_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            u.audio_input_tokens = details
                .get("audio_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
        }
        if let Some(details) = usage.get("completion_tokens_details") {
            u.audio_output_tokens = details
                .get("audio_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
        }

        Ok(Some(u))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_response_basic() {
        let parser = OpenAIParser;
        let resp = json!({"usage": {"prompt_tokens": 10, "completion_tokens": 20}});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 20);
    }

    #[test]
    fn test_parse_response_missing_usage() {
        let parser = OpenAIParser;
        let resp = json!({"choices": []});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 0);
        assert_eq!(u.output_tokens, 0);
    }

    #[test]
    fn test_parse_response_cache_tokens() {
        let parser = OpenAIParser;
        let resp = json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "prompt_tokens_details": {"cached_tokens": 60}
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.output_tokens, 50);
        assert_eq!(u.cache_read_tokens, 60);
    }

    #[test]
    fn test_parse_streaming_chunk_with_usage() {
        let parser = OpenAIParser;
        let line = r#"data: {"choices":[],"usage":{"prompt_tokens":10,"completion_tokens":20}}"#;
        let u = parser.parse_streaming_chunk(line).unwrap().unwrap();
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 20);
    }

    #[test]
    fn test_parse_streaming_chunk_done() {
        let parser = OpenAIParser;
        assert!(OpenAIParser
            .parse_streaming_chunk("data: [DONE]")
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_parse_streaming_chunk_no_usage() {
        let parser = OpenAIParser;
        let line = r#"data: {"choices":[{"delta":{"content":"hi"}}]}"#;
        assert!(parser.parse_streaming_chunk(line).unwrap().is_none());
    }

    #[test]
    fn test_parse_streaming_chunk_empty_usage() {
        // usage present but all zeros → None (avoids zero-stomping the counter)
        let parser = OpenAIParser;
        let line = r#"data: {"usage":{"prompt_tokens":0,"completion_tokens":0}}"#;
        assert!(parser.parse_streaming_chunk(line).unwrap().is_none());
    }
}
