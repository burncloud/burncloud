pub mod providers;

use crate::error::ParseError;
use crate::types::UnifiedUsage;
use burncloud_common::types::ChannelType;
use serde_json::Value;

pub use providers::{
    AnthropicParser, DeepSeekParser, GeminiParser, GenericParser, OpenAIParser,
};

/// Parses usage information from an LLM provider response.
///
/// Each provider implements its own token-counting format.
/// Parse failures are non-fatal: log a warning and return `UnifiedUsage::default()`.
pub trait UsageParser: Send + Sync {
    /// Parse a complete (non-streaming) API response body.
    fn parse_response(&self, response: &Value) -> Result<UnifiedUsage, ParseError>;

    /// Parse a single streaming chunk (SSE line or raw JSON fragment).
    /// Returns `None` when the chunk carries no usage information.
    fn parse_streaming_chunk(&self, chunk: &str) -> Result<Option<UnifiedUsage>, ParseError>;

    /// Provider identifier for logging.
    fn provider_name(&self) -> &'static str;
}

/// Return the appropriate [`UsageParser`] for the given channel type.
/// The router always knows `channel_type`, so no auto-detection is needed.
pub fn get_parser(channel_type: ChannelType) -> Box<dyn UsageParser> {
    match channel_type {
        ChannelType::OpenAI
        | ChannelType::Azure
        | ChannelType::Moonshot
        | ChannelType::OpenAIMax => Box::new(OpenAIParser),
        ChannelType::Anthropic | ChannelType::Zai => Box::new(AnthropicParser),
        ChannelType::Gemini | ChannelType::VertexAi => Box::new(GeminiParser),
        ChannelType::DeepSeek => Box::new(DeepSeekParser),
        _ => Box::new(GenericParser),
    }
}

/// Attempt to parse a response, falling back to `UnifiedUsage::default()` on error.
/// Logs a `tracing::warn!` with the provider name and request_id on parse failure.
pub fn parse_response_or_default(
    parser: &dyn UsageParser,
    response: &Value,
    request_id: &str,
) -> UnifiedUsage {
    match parser.parse_response(response) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(
                request_id = %request_id,
                provider = %parser.provider_name(),
                error = %e,
                "Usage parse failed for non-streaming response; using default (0)"
            );
            UnifiedUsage::default()
        }
    }
}

/// Attempt to parse a streaming chunk, falling back to `None` on error.
/// Logs a `tracing::warn!` with the provider, request_id, and the raw chunk.
pub fn parse_chunk_or_default(
    parser: &dyn UsageParser,
    chunk: &str,
    request_id: &str,
) -> Option<UnifiedUsage> {
    match parser.parse_streaming_chunk(chunk) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(
                request_id = %request_id,
                provider = %parser.provider_name(),
                error = %e,
                chunk = %chunk,
                "Usage parse failed for streaming chunk; skipping"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_parser_openai() {
        assert_eq!(get_parser(ChannelType::OpenAI).provider_name(), "openai");
        assert_eq!(get_parser(ChannelType::Azure).provider_name(), "openai");
        assert_eq!(get_parser(ChannelType::Moonshot).provider_name(), "openai");
    }

    #[test]
    fn test_get_parser_anthropic() {
        assert_eq!(get_parser(ChannelType::Anthropic).provider_name(), "anthropic");
        // z.ai uses Anthropic-compatible protocol
        assert_eq!(get_parser(ChannelType::Zai).provider_name(), "anthropic");
    }

    #[test]
    fn test_get_parser_gemini() {
        assert_eq!(get_parser(ChannelType::Gemini).provider_name(), "gemini");
        assert_eq!(get_parser(ChannelType::VertexAi).provider_name(), "gemini");
    }

    #[test]
    fn test_get_parser_deepseek() {
        assert_eq!(get_parser(ChannelType::DeepSeek).provider_name(), "deepseek");
    }
}
