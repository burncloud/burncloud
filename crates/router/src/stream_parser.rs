use crate::token_counter::StreamingTokenCounter;
use serde_json::Value;

/// Parses streaming responses and extracts token usage from various providers.
pub struct StreamingTokenParser;

impl StreamingTokenParser {
    /// Parse an OpenAI streaming SSE line and extract token usage.
    /// OpenAI sends usage in the final chunk before [DONE] when stream_options.include_usage is true.
    /// Format: `data: {"choices":[...], "usage":{"prompt_tokens":X, "completion_tokens":Y}}`
    ///
    /// Also extracts cache tokens for Prompt Caching:
    /// - `prompt_tokens_details.cached_tokens`: tokens served from cache
    /// - `completion_tokens_details.reasoning_tokens`: reasoning tokens (for o1 models)
    pub fn parse_openai_chunk(line: &str, counter: &StreamingTokenCounter) {
        // Skip if not a data line
        let line = line.trim();
        if !line.starts_with("data: ") {
            return;
        }

        let data = &line[6..]; // Skip "data: "

        // Skip [DONE] marker
        if data.trim() == "[DONE]" {
            return;
        }

        // Parse JSON
        if let Ok(json) = serde_json::from_str::<Value>(data) {
            if let Some(usage) = json.get("usage") {
                if let Some(prompt) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
                    counter.set_prompt_tokens(prompt as u32);
                }
                if let Some(completion) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
                    counter.set_completion_tokens(completion as u32);
                }

                // Extract cache tokens from prompt_tokens_details (OpenAI Prompt Caching)
                if let Some(details) = usage.get("prompt_tokens_details") {
                    if let Some(cached) = details.get("cached_tokens").and_then(|v| v.as_u64()) {
                        counter.set_cache_read_tokens(cached as u32);
                    }
                }
            }
        }
    }

    /// Parse an Anthropic streaming SSE line and extract token usage.
    /// Anthropic sends usage in two events:
    /// - `message_start`: contains `input_tokens`
    /// - `message_delta`: contains cumulative `output_tokens`
    ///
    /// Also extracts cache tokens for Prompt Caching:
    /// - `cache_read_input_tokens`: tokens served from cache
    /// - `cache_creation_input_tokens`: tokens written to cache
    pub fn parse_anthropic_chunk(line: &str, counter: &StreamingTokenCounter) {
        let line = line.trim();

        // Parse event type
        if line.starts_with("data: ") {
            let data = &line[6..];
            if let Ok(json) = serde_json::from_str::<Value>(data) {
                // Handle message_start event
                if json.get("type").and_then(|v| v.as_str()) == Some("message_start") {
                    if let Some(message) = json.get("message") {
                        if let Some(usage) = message.get("usage") {
                            if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_u64())
                            {
                                counter.set_prompt_tokens(input as u32);
                            }

                            // Extract cache tokens (Anthropic Prompt Caching)
                            if let Some(cache_read) = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()) {
                                counter.set_cache_read_tokens(cache_read as u32);
                            }
                            if let Some(cache_creation) = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()) {
                                counter.set_cache_creation_tokens(cache_creation as u32);
                            }
                        }
                    }
                }
                // Handle message_delta event
                else if json.get("type").and_then(|v| v.as_str()) == Some("message_delta") {
                    if let Some(usage) = json.get("usage") {
                        if let Some(output) = usage.get("output_tokens").and_then(|v| v.as_u64()) {
                            counter.set_completion_tokens(output as u32);
                        }

                        // Cache tokens can also appear in message_delta
                        if let Some(cache_read) = usage.get("cache_read_input_tokens").and_then(|v| v.as_u64()) {
                            counter.set_cache_read_tokens(cache_read as u32);
                        }
                        if let Some(cache_creation) = usage.get("cache_creation_input_tokens").and_then(|v| v.as_u64()) {
                            counter.set_cache_creation_tokens(cache_creation as u32);
                        }
                    }
                }
            }
        }
    }

    /// Parse a Gemini streaming response chunk and extract token usage.
    /// Gemini sends usageMetadata in response chunks.
    /// Format: `{"candidates":[...], "usageMetadata":{"promptTokenCount":X, "candidatesTokenCount":Y}}`
    pub fn parse_gemini_chunk(chunk: &str, counter: &StreamingTokenCounter) {
        // Handle array format "[{...}," or ",{...}]" which happens in some stream outputs
        let clean_chunk = chunk
            .trim()
            .trim_start_matches('[')
            .trim_start_matches(',')
            .trim_end_matches(',')
            .trim_end_matches(']');

        if clean_chunk.is_empty() {
            return;
        }

        if let Ok(json) = serde_json::from_str::<Value>(clean_chunk) {
            if let Some(metadata) = json.get("usageMetadata") {
                if let Some(prompt) = metadata.get("promptTokenCount").and_then(|v| v.as_u64()) {
                    counter.set_prompt_tokens(prompt as u32);
                }
                if let Some(completion) = metadata
                    .get("candidatesTokenCount")
                    .and_then(|v| v.as_u64())
                {
                    counter.set_completion_tokens(completion as u32);
                }

                // Gemini also supports cached_content_token_count for context caching
                if let Some(cached) = metadata.get("cachedContentTokenCount").and_then(|v| v.as_u64()) {
                    counter.set_cache_read_tokens(cached as u32);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openai_chunk_with_usage() {
        let counter = StreamingTokenCounter::new();
        let line = r#"data: {"choices":[{"delta":{"content":"Hello"}}], "usage":{"prompt_tokens":10, "completion_tokens":20}}"#;

        StreamingTokenParser::parse_openai_chunk(line, &counter);

        assert_eq!(counter.get_usage(), (10, 20));
    }

    #[test]
    fn test_parse_openai_chunk_without_usage() {
        let counter = StreamingTokenCounter::new();
        let line = r#"data: {"choices":[{"delta":{"content":"Hello"}}]}"#;

        StreamingTokenParser::parse_openai_chunk(line, &counter);

        assert_eq!(counter.get_usage(), (0, 0));
    }

    #[test]
    fn test_parse_openai_done_marker() {
        let counter = StreamingTokenCounter::new();
        counter.set_prompt_tokens(100);

        StreamingTokenParser::parse_openai_chunk("data: [DONE]", &counter);

        // Should not modify counter
        assert_eq!(counter.get_usage(), (100, 0));
    }

    #[test]
    fn test_parse_anthropic_message_start() {
        let counter = StreamingTokenCounter::new();
        let line = r#"data: {"type":"message_start","message":{"id":"msg_123","usage":{"input_tokens":50}}}"#;

        StreamingTokenParser::parse_anthropic_chunk(line, &counter);

        assert_eq!(counter.get_usage(), (50, 0));
    }

    #[test]
    fn test_parse_anthropic_message_delta() {
        let counter = StreamingTokenCounter::new();
        counter.set_prompt_tokens(50);

        let line = r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":75}}"#;

        StreamingTokenParser::parse_anthropic_chunk(line, &counter);

        assert_eq!(counter.get_usage(), (50, 75));
    }

    #[test]
    fn test_parse_gemini_chunk_with_usage() {
        let counter = StreamingTokenCounter::new();
        let chunk = r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}]}}], "usageMetadata":{"promptTokenCount":30, "candidatesTokenCount":40, "totalTokenCount":70}}"#;

        StreamingTokenParser::parse_gemini_chunk(chunk, &counter);

        assert_eq!(counter.get_usage(), (30, 40));
    }

    #[test]
    fn test_parse_gemini_array_format() {
        let counter = StreamingTokenCounter::new();
        let chunk = r#"[{"candidates":[],"usageMetadata":{"promptTokenCount":20,"candidatesTokenCount":30}}]"#;

        StreamingTokenParser::parse_gemini_chunk(chunk, &counter);

        assert_eq!(counter.get_usage(), (20, 30));
    }

    #[test]
    fn test_parse_openai_cache_tokens() {
        let counter = StreamingTokenCounter::new();
        let line = r#"data: {"choices":[],"usage":{"prompt_tokens":100,"completion_tokens":50,"prompt_tokens_details":{"cached_tokens":60}}}"#;

        StreamingTokenParser::parse_openai_chunk(line, &counter);

        assert_eq!(counter.get_usage(), (100, 50));
        assert_eq!(counter.get_cache_usage(), (60, 0));
    }

    #[test]
    fn test_parse_anthropic_cache_tokens() {
        let counter = StreamingTokenCounter::new();
        let line = r#"data: {"type":"message_start","message":{"id":"msg_123","usage":{"input_tokens":100,"cache_read_input_tokens":50,"cache_creation_input_tokens":20}}}"#;

        StreamingTokenParser::parse_anthropic_chunk(line, &counter);

        assert_eq!(counter.get_usage(), (100, 0));
        assert_eq!(counter.get_cache_usage(), (50, 20));
    }

    #[test]
    fn test_parse_anthropic_cache_tokens_in_delta() {
        let counter = StreamingTokenCounter::new();
        counter.set_prompt_tokens(100);

        let line = r#"data: {"type":"message_delta","delta":{"stop_reason":"end_turn"},"usage":{"output_tokens":75,"cache_read_input_tokens":30}}"#;

        StreamingTokenParser::parse_anthropic_chunk(line, &counter);

        assert_eq!(counter.get_usage(), (100, 75));
        assert_eq!(counter.get_cache_usage(), (30, 0));
    }

    #[test]
    fn test_parse_gemini_cached_tokens() {
        let counter = StreamingTokenCounter::new();
        let chunk = r#"{"candidates":[],"usageMetadata":{"promptTokenCount":100,"candidatesTokenCount":50,"cachedContentTokenCount":40}}"#;

        StreamingTokenParser::parse_gemini_chunk(chunk, &counter);

        assert_eq!(counter.get_usage(), (100, 50));
        assert_eq!(counter.get_cache_usage(), (40, 0));
    }
}
