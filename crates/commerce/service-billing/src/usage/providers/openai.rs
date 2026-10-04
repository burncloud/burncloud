// LLM provider response parsing requires Value — all providers implement
// UsageParser whose trait method signature uses `&Value`.
#![allow(clippy::disallowed_types)]

use crate::error::ParseError;
use crate::types::UnifiedUsage;
use crate::usage::{read_count, UsageParser};
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

        // Every count goes through `read_count`, which tolerates the shapes an upstream may send and reports
        // a field that is present but unreadable. See its documentation and #664.
        let mut u = UnifiedUsage {
            input_tokens: read_count(usage, "prompt_tokens", self.provider_name())?,
            output_tokens: read_count(usage, "completion_tokens", self.provider_name())?,
            ..Default::default()
        };

        // Prompt Caching. The details object may be absent or `null`; `get(..).and_then(get)` handles both,
        // and an absent field inside it stays zero.
        if let Some(details) = usage.get("prompt_tokens_details") {
            u.cache_read_tokens = read_count(details, "cached_tokens", self.provider_name())?;
            u.audio_input_tokens = read_count(details, "audio_tokens", self.provider_name())?;
        }
        if let Some(details) = usage.get("completion_tokens_details") {
            u.audio_output_tokens = read_count(details, "audio_tokens", self.provider_name())?;
        }

        Ok(u)
    }

    fn parse_streaming_chunk(&self, chunk: &str) -> Result<Option<UnifiedUsage>, ParseError> {
        // Handle multi-line SSE chunks - each line may contain a "data: {...}" entry
        for line in chunk.lines() {
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let data = &line[6..];
            if data.trim() == "[DONE]" {
                continue;
            }

            // Try to parse JSON, skip lines that don't parse
            let json: Value = match serde_json::from_str(data) {
                Ok(v) => v,
                Err(_) => continue, // Skip malformed lines
            };

            let Some(usage) = json.get("usage") else {
                continue;
            };

            // Only process chunks that carry usage info. A count that is present but unreadable is reported
            // rather than folded into the `input == 0 && output == 0` skip below, which exists for chunks
            // that genuinely carry no usage.
            let input = read_count(usage, "prompt_tokens", self.provider_name())?;
            let output = read_count(usage, "completion_tokens", self.provider_name())?;
            if input == 0 && output == 0 {
                continue;
            }

            let mut u = UnifiedUsage {
                input_tokens: input,
                output_tokens: output,
                ..Default::default()
            };

            if let Some(details) = usage.get("prompt_tokens_details") {
                u.cache_read_tokens = read_count(details, "cached_tokens", self.provider_name())?;
                u.audio_input_tokens = read_count(details, "audio_tokens", self.provider_name())?;
            }
            if let Some(details) = usage.get("completion_tokens_details") {
                u.audio_output_tokens = read_count(details, "audio_tokens", self.provider_name())?;
            }

            return Ok(Some(u));
        }

        Ok(None)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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
        let _parser = OpenAIParser;
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

    #[test]
    fn test_parse_streaming_chunk_multiline() {
        // SSE chunk may contain multiple lines
        let parser = OpenAIParser;
        let chunk = r#"data: {"choices":[{"delta":{"content":"hi"}}]}
data: {"usage":{"prompt_tokens":10,"completion_tokens":20}}"#;
        let u = parser.parse_streaming_chunk(chunk).unwrap().unwrap();
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 20);
    }

    #[test]
    fn test_parse_streaming_chunk_multiline_with_malformed() {
        // Should skip malformed lines and find usage in valid line
        let parser = OpenAIParser;
        let chunk = r#"data: {"choices":[{"delta":{"content":"hi"}}]}
data: malformed json here
data: {"usage":{"prompt_tokens":15,"completion_tokens":25}}"#;
        let u = parser.parse_streaming_chunk(chunk).unwrap().unwrap();
        assert_eq!(u.input_tokens, 15);
        assert_eq!(u.output_tokens, 25);
    }
}
