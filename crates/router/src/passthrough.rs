//! Gemini Passthrough Detection Module
//!
//! This module provides detection logic for determining when to use passthrough mode
//! (direct forwarding without protocol conversion) for Gemini API requests.
//!
//! ## Detection Strategy
//!
//! Passthrough is triggered when EITHER condition is met:
//! 1. Path matches Gemini native API pattern (e.g., `/v1beta/models/...`)
//! 2. Request body contains Gemini native format (has `contents` field)
//!
//! ## Response Handling
//!
//! When passthrough mode is active:
//! - Request is forwarded as-is (no conversion)
//! - Response is returned as-is (no conversion)
//! - Streaming responses are passed through directly
//! - Token counting uses Gemini's `usageMetadata` field

use burncloud_common::types::ChannelType;
use serde_json::Value;

/// Detection result for passthrough mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassthroughDecision {
    /// Use passthrough mode - forward request/response as-is
    Passthrough,
    /// Use protocol conversion - convert between OpenAI and provider format
    Convert,
}

/// Checks if the request should be handled in passthrough mode.
///
/// # Arguments
///
/// * `path` - The request path (e.g., "/v1/chat/completions" or "/v1beta/models/gemini-pro:generateContent")
/// * `body` - The parsed JSON request body
/// * `channel_type` - The type of channel being routed to
///
/// # Returns
///
/// `PassthroughDecision::Passthrough` if the request should be forwarded as-is,
/// `PassthroughDecision::Convert` if protocol conversion should be applied.
pub fn should_passthrough(path: &str, body: &Value, channel_type: ChannelType) -> PassthroughDecision {
    // Only Gemini channels support passthrough
    if channel_type != ChannelType::Gemini && channel_type != ChannelType::VertexAi {
        return PassthroughDecision::Convert;
    }

    // Condition 1: Gemini native path patterns
    let is_gemini_path = is_gemini_native_path(path);

    // Condition 2: Gemini native content format (has "contents" field)
    let is_gemini_content = is_gemini_native_content(body);

    if is_gemini_path || is_gemini_content {
        PassthroughDecision::Passthrough
    } else {
        PassthroughDecision::Convert
    }
}

/// Checks if the path matches Gemini native API patterns.
///
/// Gemini API paths typically follow these patterns:
/// - `/v1beta/models/{model}:generateContent`
/// - `/v1beta/models/{model}:streamGenerateContent`
/// - `/v1/models/{model}:generateContent`
/// - `/v1/models/{model}:streamGenerateContent`
/// - `/v1beta/models/{model}:countTokens`
/// - `/v1beta/models/{model}:embedContent`
fn is_gemini_native_path(path: &str) -> bool {
    // Check for Gemini native API paths
    path.starts_with("/v1beta/models/")
        || path.starts_with("/v1/models/")
        // Also match paths without leading slash (in case of edge cases)
        || path.starts_with("v1beta/models/")
        || path.starts_with("v1/models/")
}

/// Checks if the request body contains Gemini native format.
///
/// Gemini native format is identified by the presence of `contents` field
/// (as opposed to OpenAI's `messages` field).
fn is_gemini_native_content(body: &Value) -> bool {
    // Check for Gemini-specific "contents" field
    body.get("contents").is_some()
        && body.get("contents").map_or(false, |c| c.is_array())
}

/// Builds the target URL for Gemini passthrough mode.
///
/// # Arguments
///
/// * `base_url` - The channel's base URL (e.g., "https://generativelanguage.googleapis.com")
/// * `path` - The original request path
/// * `body` - The request body (used to extract model name if needed)
///
/// # Returns
///
/// The complete target URL for the Gemini API request.
pub fn build_gemini_passthrough_url(base_url: &str, path: &str, body: &Value) -> String {
    // Normalize base_url - remove trailing /v1beta or /v1 if present
    // because the path already contains the version prefix
    let clean_base = base_url
        .trim_end_matches('/')
        .trim_end_matches("/v1beta")
        .trim_end_matches("/v1");

    // If path is already a Gemini native path, use it directly
    if is_gemini_native_path(path) {
        return format!("{}{}", clean_base, path);
    }

    // Otherwise, construct URL from model name in body
    let model = body
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("gemini-2.0-flash");

    // Check if streaming is requested
    let is_stream = body
        .get("stream")
        .and_then(|s| s.as_bool())
        .unwrap_or(false);

    let method = if is_stream {
        "streamGenerateContent"
    } else {
        "generateContent"
    };

    format!(
        "{}/v1beta/models/{}:{}",
        clean_base,
        model,
        method
    )
}

/// Extracts the model name from a Gemini native path.
///
/// # Arguments
///
/// * `path` - The Gemini API path (e.g., "/v1beta/models/gemini-pro:generateContent")
///
/// # Returns
///
/// The model name if found, or `None` if the path doesn't match the expected pattern.
pub fn extract_model_from_gemini_path(path: &str) -> Option<String> {
    // Pattern: /v1beta/models/{model}:{method}
    let path = path.trim_start_matches('/');

    for prefix in &["v1beta/models/", "v1/models/"] {
        if let Some(rest) = path.strip_prefix(prefix) {
            // Extract model name before the colon
            if let Some(pos) = rest.find(':') {
                return Some(rest[..pos].to_string());
            } else {
                // No colon, entire rest is the model name
                return Some(rest.to_string());
            }
        }
    }
    None
}

/// Parses token usage from Gemini response's `usageMetadata` field.
///
/// # Arguments
///
/// * `response` - The Gemini API response JSON
///
/// # Returns
///
/// A tuple of (prompt_tokens, completion_tokens) if found, or (0, 0) if not present.
pub fn parse_gemini_usage(response: &Value) -> (u32, u32) {
    let usage = response.get("usageMetadata");

    if let Some(usage) = usage {
        let prompt_tokens = usage
            .get("promptTokenCount")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        let completion_tokens = usage
            .get("candidatesTokenCount")
            .and_then(|t| t.as_u64())
            .unwrap_or(0) as u32;

        (prompt_tokens, completion_tokens)
    } else {
        (0, 0)
    }
}

/// Parses token usage from a Gemini streaming chunk.
///
/// The final chunk in a streaming response contains `usageMetadata`.
///
/// # Arguments
///
/// * `chunk` - The streaming chunk text
///
/// # Returns
///
/// A tuple of (prompt_tokens, completion_tokens) if found, or (0, 0) if not present.
pub fn parse_gemini_streaming_usage(chunk: &str) -> (u32, u32) {
    // Handle array format "[{...}," or ",{...}]"
    let clean_chunk = chunk
        .trim()
        .trim_start_matches('[')
        .trim_start_matches(',')
        .trim_end_matches(',')
        .trim_end_matches(']');

    if clean_chunk.is_empty() {
        return (0, 0);
    }

    if let Ok(value) = serde_json::from_str::<Value>(clean_chunk) {
        parse_gemini_usage(&value)
    } else {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_is_gemini_native_path() {
        assert!(is_gemini_native_path("/v1beta/models/gemini-pro:generateContent"));
        assert!(is_gemini_native_path("/v1/models/gemini-pro:streamGenerateContent"));
        assert!(is_gemini_native_path("/v1beta/models/gemini-2.0-flash:countTokens"));
        assert!(!is_gemini_native_path("/v1/chat/completions"));
        assert!(!is_gemini_native_path("/v1/embeddings"));
    }

    #[test]
    fn test_is_gemini_native_content() {
        // Gemini format
        let gemini_body = json!({
            "contents": [
                {"role": "user", "parts": [{"text": "Hello"}]}
            ]
        });
        assert!(is_gemini_native_content(&gemini_body));

        // OpenAI format
        let openai_body = json!({
            "messages": [
                {"role": "user", "content": "Hello"}
            ]
        });
        assert!(!is_gemini_native_content(&openai_body));

        // Empty contents array (still Gemini format)
        let empty_gemini = json!({
            "contents": []
        });
        assert!(is_gemini_native_content(&empty_gemini));

        // contents is not an array
        let invalid_gemini = json!({
            "contents": "invalid"
        });
        assert!(!is_gemini_native_content(&invalid_gemini));
    }

    #[test]
    fn test_should_passthrough() {
        // Gemini path -> passthrough
        assert_eq!(
            should_passthrough(
                "/v1beta/models/gemini-pro:generateContent",
                &json!({}),
                ChannelType::Gemini
            ),
            PassthroughDecision::Passthrough
        );

        // Gemini content -> passthrough
        assert_eq!(
            should_passthrough(
                "/v1/chat/completions",
                &json!({"contents": [{"role": "user", "parts": [{"text": "Hi"}]}]}),
                ChannelType::Gemini
            ),
            PassthroughDecision::Passthrough
        );

        // OpenAI content to Gemini -> convert
        assert_eq!(
            should_passthrough(
                "/v1/chat/completions",
                &json!({"messages": [{"role": "user", "content": "Hi"}]}),
                ChannelType::Gemini
            ),
            PassthroughDecision::Convert
        );

        // Non-Gemini channel -> always convert
        assert_eq!(
            should_passthrough(
                "/v1beta/models/gemini-pro:generateContent",
                &json!({"contents": []}),
                ChannelType::OpenAI
            ),
            PassthroughDecision::Convert
        );

        // Vertex AI also supports passthrough
        assert_eq!(
            should_passthrough(
                "/v1beta/models/gemini-pro:generateContent",
                &json!({}),
                ChannelType::VertexAi
            ),
            PassthroughDecision::Passthrough
        );
    }

    #[test]
    fn test_build_gemini_passthrough_url() {
        // Native path preserved
        let url = build_gemini_passthrough_url(
            "https://generativelanguage.googleapis.com",
            "/v1beta/models/gemini-pro:generateContent",
            &json!({}),
        );
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent"
        );

        // Non-native path -> construct from model
        let url = build_gemini_passthrough_url(
            "https://generativelanguage.googleapis.com",
            "/v1/chat/completions",
            &json!({"model": "gemini-2.0-flash", "contents": []}),
        );
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.0-flash:generateContent"
        );

        // Streaming request
        let url = build_gemini_passthrough_url(
            "https://generativelanguage.googleapis.com",
            "/v1/chat/completions",
            &json!({"model": "gemini-pro", "contents": [], "stream": true}),
        );
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:streamGenerateContent"
        );
    }

    #[test]
    fn test_extract_model_from_gemini_path() {
        assert_eq!(
            extract_model_from_gemini_path("/v1beta/models/gemini-pro:generateContent"),
            Some("gemini-pro".to_string())
        );
        assert_eq!(
            extract_model_from_gemini_path("/v1/models/gemini-2.0-flash:streamGenerateContent"),
            Some("gemini-2.0-flash".to_string())
        );
        assert_eq!(
            extract_model_from_gemini_path("/v1/chat/completions"),
            None
        );
    }

    #[test]
    fn test_parse_gemini_usage() {
        let response = json!({
            "candidates": [],
            "usageMetadata": {
                "promptTokenCount": 10,
                "candidatesTokenCount": 25,
                "totalTokenCount": 35
            }
        });

        let (prompt, completion) = parse_gemini_usage(&response);
        assert_eq!(prompt, 10);
        assert_eq!(completion, 25);

        // Missing usageMetadata
        let (prompt, completion) = parse_gemini_usage(&json!({"candidates": []}));
        assert_eq!(prompt, 0);
        assert_eq!(completion, 0);
    }

    #[test]
    fn test_parse_gemini_streaming_usage() {
        let chunk = r#"{"candidates":[{"content":{"parts":[{"text":"Hello"}]}}],"usageMetadata":{"promptTokenCount":5,"candidatesTokenCount":10}}"#;

        let (prompt, completion) = parse_gemini_streaming_usage(chunk);
        assert_eq!(prompt, 5);
        assert_eq!(completion, 10);

        // Array format
        let chunk = r#"[{"candidates":[],"usageMetadata":{"promptTokenCount":3,"candidatesTokenCount":7}}]"#;
        let (prompt, completion) = parse_gemini_streaming_usage(chunk);
        assert_eq!(prompt, 3);
        assert_eq!(completion, 7);
    }
}
