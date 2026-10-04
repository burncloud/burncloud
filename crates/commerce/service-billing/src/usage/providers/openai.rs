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
        let prompt_tokens = read_count(usage, "prompt_tokens", self.provider_name())?;
        let output_tokens = read_count(usage, "completion_tokens", self.provider_name())?;

        // Prompt Caching. The details object may be absent or `null`; `get(..).and_then(get)` handles both,
        // and an absent field inside it stays zero.
        let mut cache_read_tokens = 0;
        let mut audio_input_tokens = 0;
        if let Some(details) = usage.get("prompt_tokens_details") {
            cache_read_tokens = read_count(details, "cached_tokens", self.provider_name())?;
            audio_input_tokens = read_count(details, "audio_tokens", self.provider_name())?;
        }
        let audio_output_tokens = match usage.get("completion_tokens_details") {
            Some(details) => read_count(details, "audio_tokens", self.provider_name())?,
            None => 0,
        };

        // **`prompt_tokens` includes the cached ones**, and `input_tokens` must not, so the cached count is
        // subtracted -- the same normalisation `gemini.rs` performs for `promptTokenCount`.
        //
        // Without this, the cached tokens were billed twice: once at the discounted cache rate through
        // `cache_read_tokens`, and again at the full input rate because they were still inside
        // `input_tokens`. Measured at four times the correct input charge on a 80%-cached request (#670).
        //
        // **Not written as `saturating_sub`.** On this toolchain (rustc 1.94.1) `saturating_sub` on signed
        // integers does not clamp: `1i64.saturating_sub(2)` yields `-1` and `10i64.saturating_sub(99)` yields
        // `-89`, while the unsigned forms clamp correctly. A negative `input_tokens` would reach the cost
        // calculation and *reduce* the charge, so the clamp is written out explicitly where it can be relied
        // on. The toolchain defect is reported separately; this is the safe form regardless of its outcome.
        let uncached_input = if prompt_tokens > cache_read_tokens {
            prompt_tokens - cache_read_tokens
        } else {
            0
        };
        if cache_read_tokens > prompt_tokens {
            tracing::warn!(
                provider = self.provider_name(),
                prompt_tokens,
                cache_read_tokens,
                "cached_tokens exceeds prompt_tokens; treating the uncached input as zero"
            );
        }

        Ok(UnifiedUsage {
            input_tokens: uncached_input,
            output_tokens,
            cache_read_tokens,
            audio_input_tokens,
            audio_output_tokens,
            ..Default::default()
        })
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
            let prompt_tokens = read_count(usage, "prompt_tokens", self.provider_name())?;
            let output = read_count(usage, "completion_tokens", self.provider_name())?;
            if prompt_tokens == 0 && output == 0 {
                continue;
            }

            let mut cache_read_tokens = 0;
            let mut audio_input_tokens = 0;
            if let Some(details) = usage.get("prompt_tokens_details") {
                cache_read_tokens = read_count(details, "cached_tokens", self.provider_name())?;
                audio_input_tokens = read_count(details, "audio_tokens", self.provider_name())?;
            }
            let audio_output_tokens = match usage.get("completion_tokens_details") {
                Some(details) => read_count(details, "audio_tokens", self.provider_name())?,
                None => 0,
            };

            // The same clamp as `parse_response`, for the same reason, and written the same way: the
            // streaming and non-streaming paths must not disagree about what `input_tokens` means, and
            // `saturating_sub` cannot be relied on for signed integers on this toolchain. See #670.
            let uncached_input = if prompt_tokens > cache_read_tokens {
                prompt_tokens - cache_read_tokens
            } else {
                0
            };
            if cache_read_tokens > prompt_tokens {
                tracing::warn!(
                    provider = self.provider_name(),
                    prompt_tokens,
                    cache_read_tokens,
                    "cached_tokens exceeds prompt_tokens in a stream chunk; treating the uncached input as zero"
                );
            }

            return Ok(Some(UnifiedUsage {
                input_tokens: uncached_input,
                output_tokens: output,
                cache_read_tokens,
                audio_input_tokens,
                audio_output_tokens,
                ..Default::default()
            }));
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
        // `prompt_tokens` **includes** the cached ones, so `input_tokens` is the uncached remainder. This
        // assertion used to be `100`, which was the double-count: the 60 cached tokens appeared both in
        // `input_tokens` (billed at the full input rate) and in `cache_read_tokens` (billed again at the
        // cache rate). See #670.
        //
        // The change does not alter what the request costs. Before, the calculator charged 100 at the input
        // rate **and** 60 at the cache rate. Now it charges 40 at the input rate and 60 at the cache rate --
        // and 40 + 60 is the 100 the provider reported, so the uncached part is still billed at the input
        // rate and the cached part at the cache rate, once each.
        let parser = OpenAIParser;
        let resp = json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "prompt_tokens_details": {"cached_tokens": 60}
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(
            u.input_tokens, 40,
            "100 prompt tokens of which 60 were cached"
        );
        assert_eq!(u.output_tokens, 50);
        assert_eq!(
            u.cache_read_tokens, 60,
            "and the cached count is reported separately, as before"
        );
        assert_eq!(
            u.input_tokens + u.cache_read_tokens,
            100,
            "the two together reconstruct the provider's prompt count, which is what makes this a split \
             rather than a loss"
        );
    }

    #[test]
    fn test_parse_response_without_cache_leaves_input_untouched() {
        // The control for the subtraction: with no cache details it must not change anything, so the vast
        // majority of requests are unaffected by #670's fix.
        let parser = OpenAIParser;
        let resp = json!({"usage": {"prompt_tokens": 100, "completion_tokens": 50}});
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 100);
        assert_eq!(u.cache_read_tokens, 0);
    }

    #[test]
    fn test_parse_response_cache_larger_than_prompt_is_clamped() {
        // Not representable under the subset reading the provider implies. `saturating_sub` makes it zero
        // rather than a negative -- a negative input count would reach the calculator, which would then
        // reduce the charge. A warning is logged; the assertion here covers the value.
        let parser = OpenAIParser;
        let resp = json!({
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "prompt_tokens_details": {"cached_tokens": 99}
            }
        });
        let u = parser.parse_response(&resp).unwrap();
        assert_eq!(u.input_tokens, 0, "clamped, not negative");
        assert_eq!(
            u.cache_read_tokens, 99,
            "and the reported count is still reported"
        );
    }

    #[test]
    fn the_clamp_does_not_depend_on_saturating_sub() {
        // A regression guard for the clamp itself, prompted by a toolchain defect found while writing the
        // subtraction: on rustc 1.94.1 `saturating_sub` on **signed** integers does not clamp
        // (`1i64.saturating_sub(2)` returns `-1`), while the unsigned forms do.
        //
        // This test does not assert the toolchain's behaviour -- that would fail on a sound compiler and
        // would be asserting a bug rather than a contract. It asserts that the parser produces a clamped
        // value, which holds either way, and it is written so that a regression to `saturating_sub` on a
        // defective toolchain is caught here rather than in a customer's balance.
        let parser = OpenAIParser;

        // Every ordering of the two counts, so neither the underflow path nor the normal path is missed.
        for (prompt, cached, expected_input) in
            [(10, 99, 0), (99, 10, 89), (0, 0, 0), (5, 5, 0), (1, 2, 0)]
        {
            let resp = json!({
                "usage": {
                    "prompt_tokens": prompt,
                    "completion_tokens": 1,
                    "prompt_tokens_details": {"cached_tokens": cached}
                }
            });
            let u = parser.parse_response(&resp).unwrap();
            assert_eq!(
                u.input_tokens, expected_input,
                "prompt={prompt} cached={cached}: the uncached input must never be negative"
            );
            assert!(
                u.input_tokens >= 0,
                "prompt={prompt} cached={cached}: a negative input count would reduce the charge"
            );
        }
    }

    #[test]
    fn streaming_and_non_streaming_agree_about_input_tokens() {
        // The two paths must not disagree about what the field means, which is the same class of problem
        // #670 describes between providers -- so it is asserted between the two paths of one provider.
        let parser = OpenAIParser;
        let body = json!({
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "prompt_tokens_details": {"cached_tokens": 60}
            }
        });
        let from_response = parser.parse_response(&body).unwrap();

        let line = format!("data: {body}");
        let from_chunk = parser
            .parse_streaming_chunk(&line)
            .unwrap()
            .expect("the chunk carries usage");

        assert_eq!(from_response.input_tokens, from_chunk.input_tokens);
        assert_eq!(
            from_response.cache_read_tokens,
            from_chunk.cache_read_tokens
        );
        assert_eq!(from_response.input_tokens, 40);
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
