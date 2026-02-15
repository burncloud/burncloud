use crate::token_counter::StreamingTokenCounter;
use serde_json::Value;

/// Parses streaming responses and extracts token usage from various providers.
pub struct StreamingTokenParser;

impl StreamingTokenParser {
    /// Parse an OpenAI streaming SSE line and extract token usage.
    /// OpenAI sends usage in the final chunk before [DONE] when stream_options.include_usage is true.
    /// Format: `data: {"choices":[...], "usage":{"prompt_tokens":X, "completion_tokens":Y}}`
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
            }
        }
    }

    /// Parse an Anthropic streaming SSE line and extract token usage.
    /// Anthropic sends usage in two events:
    /// - `message_start`: contains `input_tokens`
    /// - `message_delta`: contains cumulative `output_tokens`
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
                            if let Some(input) = usage.get("input_tokens").and_then(|v| v.as_u64()) {
                                counter.set_prompt_tokens(input as u32);
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
                if let Some(completion) = metadata.get("candidatesTokenCount").and_then(|v| v.as_u64()) {
                    counter.set_completion_tokens(completion as u32);
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
}
