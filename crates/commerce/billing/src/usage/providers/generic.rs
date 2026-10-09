// LLM provider response parsing requires Value — all providers implement
// UsageParser whose trait method signature uses `&Value`.
#![allow(clippy::disallowed_types)]

use crate::error::ParseError;
use crate::types::UnifiedUsage;
use crate::usage::{read_count, read_count_either, UsageParser};
use serde_json::Value;

/// Generic fallback parser — tries OpenAI format first.
/// Returns `UnifiedUsage::default()` when the format is unrecognized.
pub struct GenericParser;

impl UsageParser for GenericParser {
    fn provider_name(&self) -> &'static str {
        "generic"
    }

    fn parse_response(&self, response: &Value) -> Result<UnifiedUsage, ParseError> {
        // Try OpenAI-style `usage` block
        if let Some(usage) = response.get("usage") {
            return Ok(UnifiedUsage {
                // Two spellings of the same count. `read_count_either` prefers whichever is present and
                // reports an unreadable one rather than falling through to the other spelling -- the two are
                // not necessarily the same number, so choosing the other would invent a count.
                input_tokens: read_count_either(
                    usage,
                    "prompt_tokens",
                    "input_tokens",
                    self.provider_name(),
                )?,
                output_tokens: read_count_either(
                    usage,
                    "completion_tokens",
                    "output_tokens",
                    self.provider_name(),
                )?,
                ..Default::default()
            });
        }

        // Try Gemini-style `usageMetadata` block
        if let Some(meta) = response.get("usageMetadata") {
            return Ok(UnifiedUsage {
                input_tokens: read_count(meta, "promptTokenCount", self.provider_name())?,
                output_tokens: read_count(meta, "candidatesTokenCount", self.provider_name())?,
                ..Default::default()
            });
        }

        Ok(UnifiedUsage::default())
    }

    fn parse_streaming_chunk(&self, _chunk: &str) -> Result<Option<UnifiedUsage>, ParseError> {
        // Generic parser does not attempt streaming chunk parsing
        Ok(None)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_openai_style() {
        let parser = GenericParser;
        let resp = json!({"usage": {"prompt_tokens": 5, "completion_tokens": 10}});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 5);
        assert_eq!(u.output_tokens, 10);
    }

    #[test]
    fn test_parse_anthropic_style() {
        let parser = GenericParser;
        let resp = json!({"usage": {"input_tokens": 5, "output_tokens": 10}});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 5);
        assert_eq!(u.output_tokens, 10);
    }

    #[test]
    fn test_parse_gemini_style() {
        let parser = GenericParser;
        let resp = json!({
            "usageMetadata": {"promptTokenCount": 3, "candidatesTokenCount": 7}
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 3);
        assert_eq!(u.output_tokens, 7);
    }

    #[test]
    fn test_parse_unknown_format() {
        let parser = GenericParser;
        let u = parser.parse_response(&json!({"result": "ok"})).unwrap();
        assert!(u.is_empty());
    }
}
