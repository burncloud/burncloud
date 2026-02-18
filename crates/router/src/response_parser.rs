//! Response Parser Module
//!
//! This module provides functionality for parsing rate limit information and errors
//! from various LLM provider API responses.

use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};

use crate::circuit_breaker::RateLimitScope;

/// Information about rate limits extracted from API response headers.
///
/// Different providers expose rate limit information in different ways,
/// this struct normalizes that information into a common format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Maximum number of requests allowed per time window
    pub request_limit: Option<u32>,
    /// Maximum number of tokens allowed per time window
    pub token_limit: Option<u32>,
    /// Number of requests/tokens remaining in current window
    pub remaining: Option<u32>,
    /// Unix timestamp when the rate limit resets
    pub reset: Option<u64>,
    /// Seconds until retry is allowed (from Retry-After header)
    pub retry_after: Option<u64>,
    /// Scope of the rate limit (account or model level)
    pub scope: RateLimitScope,
}

/// Information about an error extracted from an API response.
///
/// Normalizes error information from different providers into a common format.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorInfo {
    /// Type of error (e.g., "rate_limit_exceeded", "invalid_api_key")
    pub error_type: Option<String>,
    /// Human-readable error message
    pub message: Option<String>,
    /// Error code from the API
    pub code: Option<String>,
    /// Scope of the error (if applicable, e.g., for rate limits)
    pub scope: Option<RateLimitScope>,
}

/// Parse rate limit information from API response headers and body.
///
/// Dispatches to the appropriate provider-specific parser based on channel type.
///
/// # Arguments
/// * `headers` - HTTP response headers
/// * `body` - Optional response body (used by some providers like Gemini)
/// * `channel_type` - The type of channel/provider (e.g., "openai", "anthropic", "azure", "gemini")
///
/// # Returns
/// Normalized `RateLimitInfo` struct with extracted rate limit data
pub fn parse_rate_limit_info(
    headers: &HeaderMap,
    body: Option<&str>,
    channel_type: &str,
) -> RateLimitInfo {
    match channel_type.to_lowercase().as_str() {
        "openai" => parse_openai_rate_limit(headers),
        "anthropic" | "claude" => parse_anthropic_rate_limit(headers),
        "azure" => parse_azure_rate_limit(headers),
        "gemini" | "vertex" => parse_gemini_rate_limit(headers, body),
        _ => RateLimitInfo::default(),
    }
}

/// Parse rate limit information from OpenAI API response headers.
///
/// OpenAI headers:
/// - `x-ratelimit-limit-requests`: Maximum requests per minute
/// - `x-ratelimit-limit-tokens`: Maximum tokens per minute
/// - `x-ratelimit-remaining-requests`: Remaining requests
/// - `x-ratelimit-remaining-tokens`: Remaining tokens
/// - `x-ratelimit-reset-requests`: Time until request limit resets
/// - `x-ratelimit-reset-tokens`: Time until token limit resets
/// - `retry-after`: Seconds until retry (on 429)
pub fn parse_openai_rate_limit(headers: &HeaderMap) -> RateLimitInfo {
    let mut info = RateLimitInfo::default();

    // Parse request limit
    if let Some(limit) = headers
        .get("x-ratelimit-limit-requests")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.request_limit = Some(limit);
    }

    // Parse token limit
    if let Some(limit) = headers
        .get("x-ratelimit-limit-tokens")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.token_limit = Some(limit);
    }

    // Parse remaining (prefer requests, fallback to tokens)
    if let Some(remaining) = headers
        .get("x-ratelimit-remaining-requests")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.remaining = Some(remaining);
    } else if let Some(remaining) = headers
        .get("x-ratelimit-remaining-tokens")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.remaining = Some(remaining);
    }

    // Parse reset time (prefer requests)
    if let Some(reset) = headers
        .get("x-ratelimit-reset-requests")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| parse_reset_time(v))
    {
        info.reset = Some(reset);
    } else if let Some(reset) = headers
        .get("x-ratelimit-reset-tokens")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| parse_reset_time(v))
    {
        info.reset = Some(reset);
    }

    // Parse retry-after
    if let Some(retry_after) = headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
    {
        info.retry_after = Some(retry_after);
    }

    // Determine scope (OpenAI typically doesn't specify, default to unknown)
    info.scope = RateLimitScope::Unknown;

    info
}

/// Parse rate limit information from Anthropic API response headers.
///
/// Anthropic headers:
/// - `anthropic-ratelimit-requests-limit`: Maximum requests per minute
/// - `anthropic-ratelimit-requests-remaining`: Remaining requests
/// - `anthropic-ratelimit-requests-reset`: ISO timestamp when limit resets
/// - `anthropic-ratelimit-tokens-limit`: Maximum tokens per minute
/// - `anthropic-ratelimit-tokens-remaining`: Remaining tokens
/// - `anthropic-ratelimit-tokens-reset`: ISO timestamp when limit resets
/// - `retry-after`: Seconds until retry (on 429)
pub fn parse_anthropic_rate_limit(headers: &HeaderMap) -> RateLimitInfo {
    let mut info = RateLimitInfo::default();

    // Parse request limit
    if let Some(limit) = headers
        .get("anthropic-ratelimit-requests-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.request_limit = Some(limit);
    }

    // Parse token limit
    if let Some(limit) = headers
        .get("anthropic-ratelimit-tokens-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.token_limit = Some(limit);
    }

    // Parse remaining requests
    if let Some(remaining) = headers
        .get("anthropic-ratelimit-requests-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.remaining = Some(remaining);
    }

    // Parse reset time from ISO timestamp
    if let Some(reset_str) = headers
        .get("anthropic-ratelimit-requests-reset")
        .and_then(|v| v.to_str().ok())
    {
        info.reset = parse_iso_timestamp(reset_str);
    }

    // Parse retry-after
    if let Some(retry_after) = headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
    {
        info.retry_after = Some(retry_after);
    }

    // Determine scope (Anthropic typically doesn't specify, default to unknown)
    info.scope = RateLimitScope::Unknown;

    info
}

/// Parse rate limit information from Azure OpenAI API response headers.
///
/// Azure headers:
/// - `x-ratelimit-limit`: Maximum requests per time window
/// - `x-ratelimit-remaining`: Remaining requests
/// - `x-ratelimit-reset`: Unix timestamp when limit resets
pub fn parse_azure_rate_limit(headers: &HeaderMap) -> RateLimitInfo {
    let mut info = RateLimitInfo::default();

    // Parse request limit
    if let Some(limit) = headers
        .get("x-ratelimit-limit")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.request_limit = Some(limit);
    }

    // Parse remaining
    if let Some(remaining) = headers
        .get("x-ratelimit-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u32>().ok())
    {
        info.remaining = Some(remaining);
    }

    // Parse reset time (Unix timestamp)
    if let Some(reset) = headers
        .get("x-ratelimit-reset")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
    {
        info.reset = Some(reset);
    }

    // Parse retry-after
    if let Some(retry_after) = headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
    {
        info.retry_after = Some(retry_after);
    }

    // Determine scope (Azure typically doesn't specify, default to unknown)
    info.scope = RateLimitScope::Unknown;

    info
}

/// Parse rate limit information from Gemini API response.
///
/// Gemini returns rate limit information in the response body as JSON errors
/// rather than in headers. The error type is typically "RESOURCE_EXHAUSTED".
pub fn parse_gemini_rate_limit(headers: &HeaderMap, body: Option<&str>) -> RateLimitInfo {
    let mut info = RateLimitInfo::default();

    // Check for retry-after header
    if let Some(retry_after) = headers
        .get("retry-after")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
    {
        info.retry_after = Some(retry_after);
    }

    // Parse body for error details
    if let Some(body_str) = body {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body_str) {
            // Check for RESOURCE_EXHAUSTED error
            if let Some(error) = json.get("error") {
                if let Some(code) = error.get("code").and_then(|c| c.as_i64()) {
                    if code == 429 || code == 8 {
                        // HTTP 429 or gRPC RESOURCE_EXHAUSTED
                        // Try to extract retry delay from details
                        if let Some(details) = error.get("details").and_then(|d| d.as_array()) {
                            for detail in details {
                                if let Some(reason) = detail.get("@type").and_then(|t| t.as_str()) {
                                    if reason.contains("QuotaFailure")
                                        || reason.contains("RateLimitExceeded")
                                    {
                                        // Try to extract retry delay
                                        if let Some(retry_delay) = detail
                                            .get("retryDelay")
                                            .and_then(|d| d.as_str())
                                        {
                                            info.retry_after =
                                                parse_duration_string(retry_delay);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Determine scope (Gemini typically doesn't specify, default to unknown)
    info.scope = RateLimitScope::Unknown;

    info
}

/// Parse error response from various API providers.
///
/// # Arguments
/// * `body` - Response body as string
/// * `channel_type` - The type of channel/provider
///
/// # Returns
/// Normalized `ErrorInfo` struct with extracted error data
pub fn parse_error_response(body: &str, channel_type: &str) -> ErrorInfo {
    match channel_type.to_lowercase().as_str() {
        "openai" => parse_openai_error(body),
        "anthropic" | "claude" => parse_anthropic_error(body),
        "azure" => parse_azure_error(body),
        "gemini" | "vertex" => parse_gemini_error(body),
        _ => parse_generic_error(body),
    }
}

/// Parse error response from OpenAI API.
fn parse_openai_error(body: &str) -> ErrorInfo {
    let mut info = ErrorInfo::default();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(error) = json.get("error") {
            info.error_type = error.get("type").and_then(|t| t.as_str()).map(|s| s.to_string());
            info.message = error.get("message").and_then(|m| m.as_str()).map(|s| s.to_string());
            info.code = error.get("code").and_then(|c| c.as_str()).map(|s| s.to_string());
        }
    }

    info.scope = parse_rate_limit_scope_from_error(body);

    info
}

/// Parse error response from Anthropic API.
fn parse_anthropic_error(body: &str) -> ErrorInfo {
    let mut info = ErrorInfo::default();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        info.error_type = json.get("type").and_then(|t| t.as_str()).map(|s| s.to_string());
        info.message = json.get("message").and_then(|m| m.as_str()).map(|s| s.to_string());
    }

    info.scope = parse_rate_limit_scope_from_error(body);

    info
}

/// Parse error response from Azure OpenAI API.
fn parse_azure_error(body: &str) -> ErrorInfo {
    // Azure uses similar format to OpenAI
    parse_openai_error(body)
}

/// Parse error response from Gemini API.
fn parse_gemini_error(body: &str) -> ErrorInfo {
    let mut info = ErrorInfo::default();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(error) = json.get("error") {
            info.error_type = error.get("status").and_then(|s| s.as_str()).map(|s| s.to_string());
            info.message = error.get("message").and_then(|m| m.as_str()).map(|s| s.to_string());
            info.code = error.get("code").and_then(|c| c.as_i64()).map(|c| c.to_string());
        }
    }

    info.scope = parse_rate_limit_scope_from_error(body);

    info
}

/// Parse generic error response (fallback).
fn parse_generic_error(body: &str) -> ErrorInfo {
    let mut info = ErrorInfo::default();

    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        // Try common error field patterns
        if let Some(error) = json.get("error") {
            if error.is_string() {
                info.message = error.as_str().map(|s| s.to_string());
            } else {
                info.message = error.get("message").and_then(|m| m.as_str()).map(|s| s.to_string());
                info.error_type = error.get("type").and_then(|t| t.as_str()).map(|s| s.to_string());
                info.code = error.get("code").and_then(|c| c.as_str()).map(|s| s.to_string());
            }
        } else {
            info.message = json.get("message").and_then(|m| m.as_str()).map(|s| s.to_string());
        }
    } else {
        // If not JSON, use the body as the message
        info.message = Some(body.to_string());
    }

    info
}

/// Parse rate limit scope from error response body.
///
/// Looks for keywords like "account", "api key", "model" to determine scope.
pub fn parse_rate_limit_scope_from_error(body: &str) -> Option<RateLimitScope> {
    let body_lower = body.to_lowercase();

    if body_lower.contains("account")
        || body_lower.contains("api key")
        || body_lower.contains("apikey")
        || body_lower.contains("organization")
    {
        Some(RateLimitScope::Account)
    } else if body_lower.contains("model") || body_lower.contains("per minute") {
        Some(RateLimitScope::Model)
    } else {
        None
    }
}

/// Parse a reset time string (e.g., "6m0s", "30s") into seconds.
fn parse_reset_time(s: &str) -> Option<u64> {
    // Handle format like "6m0s" or "30s"
    let mut total_seconds: u64 = 0;
    let mut current_number = String::new();

    for c in s.chars() {
        if c.is_ascii_digit() {
            current_number.push(c);
        } else if !current_number.is_empty() {
            let value: u64 = current_number.parse().ok()?;
            match c {
                'h' => total_seconds += value * 3600,
                'm' => total_seconds += value * 60,
                's' => total_seconds += value,
                _ => {}
            }
            current_number.clear();
        }
    }

    if total_seconds > 0 {
        Some(total_seconds)
    } else {
        // Try parsing as plain number (seconds)
        s.parse().ok()
    }
}

/// Parse an ISO timestamp and return the Unix timestamp.
fn parse_iso_timestamp(s: &str) -> Option<u64> {
    // Parse ISO 8601 timestamp like "2024-01-15T10:30:00Z"
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|dt| dt.timestamp() as u64)
}

/// Parse a duration string like "32.123s" into seconds.
fn parse_duration_string(s: &str) -> Option<u64> {
    // Remove 's' suffix if present and parse
    let s = s.trim_end_matches('s');
    s.parse::<f64>().ok().map(|v| v.ceil() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_reset_time() {
        assert_eq!(parse_reset_time("30s"), Some(30));
        assert_eq!(parse_reset_time("1m30s"), Some(90));
        assert_eq!(parse_reset_time("2h15m30s"), Some(8130));
        assert_eq!(parse_reset_time("60"), Some(60));
    }

    #[test]
    fn test_parse_rate_limit_scope() {
        let body = r#"{"error": {"message": "Rate limit exceeded for account"}}"#;
        assert_eq!(
            parse_rate_limit_scope_from_error(body),
            Some(RateLimitScope::Account)
        );

        let body = r#"{"error": {"message": "Rate limit exceeded for model gpt-4"}}"#;
        assert_eq!(
            parse_rate_limit_scope_from_error(body),
            Some(RateLimitScope::Model)
        );

        let body = r#"{"error": {"message": "Unknown error"}}"#;
        assert_eq!(parse_rate_limit_scope_from_error(body), None);
    }

    #[test]
    fn test_parse_openai_rate_limit() {
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit-requests", "100".parse().unwrap());
        headers.insert("x-ratelimit-limit-tokens", "100000".parse().unwrap());
        headers.insert("retry-after", "30".parse().unwrap());

        let info = parse_openai_rate_limit(&headers);

        assert_eq!(info.request_limit, Some(100));
        assert_eq!(info.token_limit, Some(100000));
        assert_eq!(info.retry_after, Some(30));
    }

    #[test]
    fn test_parse_anthropic_rate_limit() {
        let mut headers = HeaderMap::new();
        headers.insert("anthropic-ratelimit-requests-limit", "50".parse().unwrap());
        headers.insert("anthropic-ratelimit-requests-reset", "1m".parse().unwrap());
        headers.insert("retry-after", "60".parse().unwrap());

        let info = parse_anthropic_rate_limit(&headers);

        assert_eq!(info.request_limit, Some(50));
        assert_eq!(info.retry_after, Some(60));
    }

    #[test]
    fn test_parse_azure_rate_limit() {
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit", "1000".parse().unwrap());
        headers.insert("x-ratelimit-remaining", "500".parse().unwrap());
        headers.insert("x-ratelimit-reset", "3600".parse().unwrap());

        let info = parse_azure_rate_limit(&headers);

        assert_eq!(info.request_limit, Some(1000));
        assert_eq!(info.remaining, Some(500));
    }

    #[test]
    fn test_parse_openai_error() {
        let body = r#"{"error": {"type": "rate_limit_exceeded", "message": "Rate limit exceeded", "code": "rate_limit_exceeded"}}"#;
        let info = parse_openai_error(body);

        assert_eq!(info.error_type, Some("rate_limit_exceeded".to_string()));
        assert_eq!(info.message, Some("Rate limit exceeded".to_string()));
        assert_eq!(info.code, Some("rate_limit_exceeded".to_string()));
    }

    #[test]
    fn test_parse_anthropic_error() {
        // Anthropic error format has type and message at root level
        let body = r#"{"type": "rate_limit_error", "message": "Rate limit exceeded. Please retry after 30 seconds."}"#;
        let info = parse_anthropic_error(body);

        assert_eq!(info.error_type, Some("rate_limit_error".to_string()));
        assert!(info.message.as_ref().unwrap().contains("Rate limit"));
    }

    #[test]
    fn test_parse_generic_error() {
        let body = r#"{"error": "Internal server error"}"#;
        let info = parse_generic_error(body);

        assert_eq!(info.message, Some("Internal server error".to_string()));
    }

    #[test]
    fn test_parse_rate_limit_info_routing() {
        let mut headers = HeaderMap::new();
        headers.insert("x-ratelimit-limit-requests", "100".parse().unwrap());

        // Test OpenAI routing
        let info = parse_rate_limit_info(&headers, None, "openai");
        assert_eq!(info.request_limit, Some(100));

        // Test Anthropic routing
        let info = parse_rate_limit_info(&headers, None, "anthropic");
        assert_eq!(info.request_limit, None); // Different header name

        // Test unknown provider
        let info = parse_rate_limit_info(&headers, None, "unknown");
        assert_eq!(info.request_limit, None);
    }
}
