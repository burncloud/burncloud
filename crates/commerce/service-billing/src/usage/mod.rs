// UsageParser trait uses &Value in its interface — LLM API responses are dynamic JSON.
#![allow(clippy::disallowed_types)]

pub mod providers;

use crate::error::ParseError;
use crate::types::UnifiedUsage;
use burncloud_supply_contracts::ChannelType;
use serde_json::Value;

pub use providers::{AnthropicParser, DeepSeekParser, GeminiParser, GenericParser, OpenAIParser};

/// Read two alternative field names, preferring the first that is **present**.
///
/// For the providers that accept more than one spelling of the same count -- `prompt_tokens` or
/// `input_tokens` -- where the old code used `.or_else(|| usage.get(..))`, which falls through on absence
/// but not on a malformed value. Keeping that distinction matters: a present-but-unreadable
/// `prompt_tokens` must be reported rather than silently skipped in favour of `input_tokens`, because the
/// two are not necessarily the same number and choosing the other would invent a count.
pub fn read_count_either(
    usage: &Value,
    first: &str,
    second: &str,
    provider: &str,
) -> Result<i64, ParseError> {
    if usage.get(first).is_some() {
        return read_count(usage, first, provider);
    }
    read_count(usage, second, provider)
}

/// Read a token count from a value that is **already** the field, rather than from its parent object.
///
/// For the providers that reach a count through two lookups -- `completion_tokens_details` then
/// `reasoning_tokens` -- the second step hands back the value itself, so there is no parent left to pass to
/// [`read_count`]. Both share [`read_count_value`], so the two entry points cannot disagree about which
/// shapes are readable.
pub fn read_count_at(value: &Value, field: &str, provider: &str) -> Result<i64, ParseError> {
    read_count_value(value, field, provider)
}

/// Read a token count from a usage object, tolerating the ways upstreams mistype one.
///
/// ## Why this exists
///
/// Every parser used to read counts as `.get(field).and_then(|v| v.as_i64()).unwrap_or(0)`. `as_i64`
/// returns `None` for anything that is not a JSON integer, so a count sent as a **string**, a **float**,
/// a **boolean** or `null` became `0` and the response was still reported as parsed successfully. Measured
/// end to end (see #664): a request that consumed 2000 tokens was billed **zero**, and the charge was
/// indistinguishable from a free request.
///
/// One helper rather than five copies, so the five providers cannot drift apart on this -- the defect was
/// shared, so the fix has to be too.
///
/// ## What each shape does
///
/// | JSON | Result | Why |
/// | --- | --- | --- |
/// | an integer | the value | the normal case |
/// | `"1500"` | `1500` | a decimal integer string has exactly one numeric reading |
/// | `"1500.0"` | `1500` | same, with a redundant fractional part that is zero |
/// | a float `1500.0` | `1500` | same reasoning; a non-zero fraction is rejected below |
/// | the field is absent | `0` | **not** an error: some responses legitimately carry no usage |
/// | `null` | `Err(MalformedField)` | "unknown count" is not "zero tokens" |
/// | `true` | `Err(MalformedField)` | not a number in any reading |
/// | `"n/a"` | `Err(MalformedField)` | not numeric |
/// | `1500.5` | `Err(MalformedField)` | a token count cannot have a fraction; rounding would invent one |
/// | a negative number | the value | negative counts are diagnosed and zeroed by `calculator.rs:350`, which already warns |
///
/// ## What this does **not** fix
///
/// `parse_response_or_default` maps `Err` to `UnifiedUsage::default()`, which is zero. So the under-count
/// is currently still produced one layer up, and the warning it logs is the only difference from before.
/// Treating an unreadable count as a *failure to bill* rather than as *nothing to bill* needs a decision
/// about what the caller should do instead -- refuse the request, or bill at an estimate -- and that is
/// recorded in #664 rather than guessed at here.
pub fn read_count(usage: &Value, field: &str, provider: &str) -> Result<i64, ParseError> {
    let Some(value) = usage.get(field) else {
        // Absent is not malformed. A response with no `usage` object, or a usage object without this
        // particular field, is the documented shape and stays zero.
        return Ok(0);
    };
    read_count_value(value, field, provider)
}

/// The shape handling shared by [`read_count`] and [`read_count_at`].
fn read_count_value(value: &Value, field: &str, provider: &str) -> Result<i64, ParseError> {
    let malformed = || {
        ParseError::MalformedField(format!(
            "{provider}: `{field}` is present but is not a token count: {value}"
        ))
    };

    if let Some(n) = value.as_i64() {
        return Ok(n);
    }

    // A float is accepted only when its fractional part is zero, so `1500.0` reads as 1500 while `1500.5`
    // is rejected rather than rounded -- rounding would invent a count the upstream did not report.
    if let Some(f) = value.as_f64() {
        if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
            return Ok(f as i64);
        }
        return Err(malformed());
    }

    // A string is accepted when it reads as an integer, with or without a zero fractional part.
    if let Some(s) = value.as_str() {
        let trimmed = s.trim();
        if let Ok(n) = trimmed.parse::<i64>() {
            return Ok(n);
        }
        if let Ok(f) = trimmed.parse::<f64>() {
            if f.fract() == 0.0 && f >= i64::MIN as f64 && f <= i64::MAX as f64 {
                return Ok(f as i64);
            }
        }
        return Err(malformed());
    }

    // `null`, booleans, arrays, objects.
    Err(malformed())
}

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
        assert_eq!(
            get_parser(ChannelType::Anthropic).provider_name(),
            "anthropic"
        );
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
        assert_eq!(
            get_parser(ChannelType::DeepSeek).provider_name(),
            "deepseek"
        );
    }
}
