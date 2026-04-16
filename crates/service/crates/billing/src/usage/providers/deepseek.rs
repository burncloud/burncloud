// LLM provider response parsing requires Value — all providers implement
// UsageParser whose trait method signature uses `&Value`.
#![allow(clippy::disallowed_types)]

use crate::error::ParseError;
use crate::types::UnifiedUsage;
use crate::usage::UsageParser;
use serde_json::Value;

/// Usage parser for DeepSeek models.
///
/// DeepSeek uses an OpenAI-compatible format but adds `reasoning_tokens` inside
/// `completion_tokens_details` for DeepSeek-R1 chain-of-thought reasoning.
pub struct DeepSeekParser;

impl UsageParser for DeepSeekParser {
    fn provider_name(&self) -> &'static str {
        "deepseek"
    }

    fn parse_response(&self, response: &Value) -> Result<UnifiedUsage, ParseError> {
        let Some(usage) = response.get("usage") else {
            return Ok(UnifiedUsage::default());
        };

        let reasoning = usage
            .get("completion_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let cache_hit = usage
            .get("prompt_cache_hit_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(UnifiedUsage {
            input_tokens: usage
                .get("prompt_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            output_tokens: usage
                .get("completion_tokens")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            cache_read_tokens: cache_hit,
            reasoning_tokens: reasoning,
            ..Default::default()
        })
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

        let reasoning = usage
            .get("completion_tokens_details")
            .and_then(|d| d.get("reasoning_tokens"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        let cache_hit = usage
            .get("prompt_cache_hit_tokens")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Ok(Some(UnifiedUsage {
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache_hit,
            reasoning_tokens: reasoning,
            ..Default::default()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_response_basic() {
        let parser = DeepSeekParser;
        let resp = json!({"usage": {"prompt_tokens": 10, "completion_tokens": 20}});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 20);
        assert_eq!(u.reasoning_tokens, 0);
    }

    #[test]
    fn test_parse_response_with_reasoning() {
        let parser = DeepSeekParser;
        let resp = json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 200,
                "completion_tokens_details": {"reasoning_tokens": 150}
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.output_tokens, 200);
        assert_eq!(u.reasoning_tokens, 150);
    }

    #[test]
    fn test_parse_response_reasoning_tokens_absent() {
        // reasoning_tokens absent → 0, not an error
        let parser = DeepSeekParser;
        let resp = json!({"usage": {"prompt_tokens": 5, "completion_tokens": 10}});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.reasoning_tokens, 0);
    }

    #[test]
    fn test_parse_response_cache_hit() {
        let parser = DeepSeekParser;
        let resp = json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "prompt_cache_hit_tokens": 80
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.cache_read_tokens, 80);
    }

    #[test]
    fn test_parse_streaming_with_reasoning() {
        let parser = DeepSeekParser;
        let line = r#"data: {"usage":{"prompt_tokens":10,"completion_tokens":20,"completion_tokens_details":{"reasoning_tokens":15}}}"#;
        let u = parser.parse_streaming_chunk(line).unwrap().unwrap();
        assert_eq!(u.reasoning_tokens, 15);
    }
}
