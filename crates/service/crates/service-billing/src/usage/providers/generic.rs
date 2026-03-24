use crate::error::ParseError;
use crate::types::UnifiedUsage;
use crate::usage::UsageParser;
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
                input_tokens: usage
                    .get("prompt_tokens")
                    .or_else(|| usage.get("input_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                output_tokens: usage
                    .get("completion_tokens")
                    .or_else(|| usage.get("output_tokens"))
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                ..Default::default()
            });
        }

        // Try Gemini-style `usageMetadata` block
        if let Some(meta) = response.get("usageMetadata") {
            return Ok(UnifiedUsage {
                input_tokens: meta
                    .get("promptTokenCount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
                output_tokens: meta
                    .get("candidatesTokenCount")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0),
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
