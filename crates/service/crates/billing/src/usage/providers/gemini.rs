// LLM provider response parsing requires Value — all providers implement
// UsageParser whose trait method signature uses `&Value`.
#![allow(clippy::disallowed_types)]

use crate::error::ParseError;
use crate::types::UnifiedUsage;
use crate::usage::UsageParser;
use serde_json::Value;

/// Usage parser for Google Gemini and Vertex AI.
///
/// Non-streaming format: `response["usageMetadata"]["promptTokenCount"]`
/// Streaming format: Gemini chunks contain `usageMetadata` in each chunk (final chunk is authoritative).
/// Gemini 2.5 thinking tokens: `usageMetadata["thoughtsTokenCount"]` billed at output rate.
/// Context caching: `usageMetadata["cachedContentTokenCount"]`
pub struct GeminiParser;

/// Strip SSE `data: ` prefix and array brackets/commas from Gemini stream chunks.
fn clean_gemini_chunk(chunk: &str) -> &str {
    let s = chunk.trim();
    // Strip SSE "data: " prefix used by Gemini's streamGenerateContent endpoint
    let s = s.strip_prefix("data: ").unwrap_or(s).trim();
    s.trim_start_matches('[')
        .trim_start_matches(',')
        .trim_end_matches(',')
        .trim_end_matches(']')
}

fn parse_usage_metadata(metadata: &Value) -> UnifiedUsage {
    let prompt = metadata
        .get("promptTokenCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let candidates = metadata
        .get("candidatesTokenCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    // Gemini 2.5: thoughtsTokenCount billed at output rate
    let thoughts = metadata
        .get("thoughtsTokenCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let cached = metadata
        .get("cachedContentTokenCount")
        .and_then(|v| v.as_i64())
        .unwrap_or(0);

    UnifiedUsage {
        input_tokens: prompt - cached,
        output_tokens: candidates,
        reasoning_tokens: thoughts,
        cache_read_tokens: cached,
        ..Default::default()
    }
}

impl UsageParser for GeminiParser {
    fn provider_name(&self) -> &'static str {
        "gemini"
    }

    fn parse_response(&self, response: &Value) -> Result<UnifiedUsage, ParseError> {
        let Some(metadata) = response.get("usageMetadata") else {
            return Ok(UnifiedUsage::default());
        };
        Ok(parse_usage_metadata(metadata))
    }

    fn parse_streaming_chunk(&self, chunk: &str) -> Result<Option<UnifiedUsage>, ParseError> {
        let clean = clean_gemini_chunk(chunk);
        if clean.is_empty() {
            return Ok(None);
        }

        let json: Value = serde_json::from_str(clean)?;
        let Some(metadata) = json.get("usageMetadata") else {
            return Ok(None);
        };

        let u = parse_usage_metadata(metadata);
        if u.is_empty() {
            Ok(None)
        } else {
            Ok(Some(u))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_response_basic() {
        let parser = GeminiParser;
        let resp = json!({"usageMetadata": {"promptTokenCount": 10, "candidatesTokenCount": 25}});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 25);
    }

    #[test]
    fn test_parse_response_missing_metadata() {
        let parser = GeminiParser;
        let u = parser.parse_response(&json!({"candidates": []})).unwrap();
        assert!(u.is_empty());
    }

    #[test]
    fn test_parse_response_thoughts_token_count() {
        // Gemini 2.5: thoughtsTokenCount stored separately as reasoning_tokens
        let parser = GeminiParser;
        let resp = json!({
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 25,
                "thoughtsTokenCount": 15
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 25);
        assert_eq!(u.reasoning_tokens, 15);
    }

    #[test]
    fn test_parse_response_cached_content() {
        let parser = GeminiParser;
        let resp = json!({
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 50,
                "cachedContentTokenCount": 40
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        // input_tokens = promptTokenCount - cachedContentTokenCount (avoid double-billing)
        assert_eq!(u.input_tokens, 60);
        assert_eq!(u.output_tokens, 50);
        assert_eq!(u.cache_read_tokens, 40);
    }

    #[test]
    fn test_parse_response_full_cache() {
        // Edge case: entire prompt is cached — input_tokens should be 0
        let parser = GeminiParser;
        let resp = json!({
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 50,
                "cachedContentTokenCount": 100
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 0);
        assert_eq!(u.cache_read_tokens, 100);
        assert_eq!(u.output_tokens, 50);
    }

    #[test]
    fn test_parse_response_thinking_split() {
        // Verify reasoning_tokens is separate from output_tokens
        let parser = GeminiParser;
        let resp = json!({
            "usageMetadata": {
                "promptTokenCount": 20,
                "candidatesTokenCount": 30,
                "thoughtsTokenCount": 50
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 20);
        assert_eq!(u.output_tokens, 30); // candidates only
        assert_eq!(u.reasoning_tokens, 50); // thoughts separate
        assert_eq!(u.cache_read_tokens, 0);
    }

    #[test]
    fn test_parse_streaming_chunk_basic() {
        let parser = GeminiParser;
        let chunk = r#"{"candidates":[],"usageMetadata":{"promptTokenCount":30,"candidatesTokenCount":40}}"#;
        let u = parser.parse_streaming_chunk(chunk).unwrap().unwrap();
        assert_eq!(u.input_tokens, 30);
        assert_eq!(u.output_tokens, 40);
    }

    #[test]
    fn test_parse_streaming_chunk_array_format() {
        let parser = GeminiParser;
        let chunk = r#"[{"candidates":[],"usageMetadata":{"promptTokenCount":20,"candidatesTokenCount":30}}]"#;
        let u = parser.parse_streaming_chunk(chunk).unwrap().unwrap();
        assert_eq!(u.input_tokens, 20);
        assert_eq!(u.output_tokens, 30);
    }

    #[test]
    fn test_parse_streaming_chunk_with_thoughts() {
        let parser = GeminiParser;
        let chunk = r#"{"usageMetadata":{"promptTokenCount":5,"candidatesTokenCount":10,"thoughtsTokenCount":8}}"#;
        let u = parser.parse_streaming_chunk(chunk).unwrap().unwrap();
        assert_eq!(u.input_tokens, 5);
        assert_eq!(u.output_tokens, 10);
        assert_eq!(u.reasoning_tokens, 8);
    }

    #[test]
    fn test_parse_streaming_chunk_with_cache() {
        let parser = GeminiParser;
        let chunk = r#"{"usageMetadata":{"promptTokenCount":100,"candidatesTokenCount":50,"cachedContentTokenCount":40}}"#;
        let u = parser.parse_streaming_chunk(chunk).unwrap().unwrap();
        assert_eq!(u.input_tokens, 60); // 100 - 40
        assert_eq!(u.cache_read_tokens, 40);
        assert_eq!(u.output_tokens, 50);
    }

    #[test]
    fn test_parse_streaming_chunk_no_metadata() {
        let parser = GeminiParser;
        let chunk = r#"{"candidates":[{"content":{"parts":[{"text":"hi"}]}}]}"#;
        assert!(parser.parse_streaming_chunk(chunk).unwrap().is_none());
    }

    #[test]
    fn test_parse_streaming_chunk_empty() {
        let parser = GeminiParser;
        assert!(parser.parse_streaming_chunk("").unwrap().is_none());
        assert!(parser.parse_streaming_chunk("  ").unwrap().is_none());
    }

    #[test]
    fn test_parse_streaming_chunk_sse_prefix() {
        // Real Gemini SSE chunks arrive as "data: {...}"
        let parser = GeminiParser;
        let chunk = r#"data: {"usageMetadata":{"promptTokenCount":10,"candidatesTokenCount":20}}"#;
        let u = parser.parse_streaming_chunk(chunk).unwrap().unwrap();
        assert_eq!(u.input_tokens, 10);
        assert_eq!(u.output_tokens, 20);
    }
}
