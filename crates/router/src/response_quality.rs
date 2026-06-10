//! Response Quality Detector Module
//!
//! This module provides functionality for detecting and classifying response quality
//! from upstream API responses. It goes beyond simple "empty vs non-empty" detection
//! to provide a nuanced quality score that feeds into the smart circuit breaker.
//!
//! Quality levels:
//! - Healthy: Complete response with tokens and normal latency
//! - Partial: Stream interrupted but received some tokens
//! - Empty: HTTP 200 but no tokens generated
//! - Malformed: Response could not be parsed
//! - UpstreamError: Explicit error from upstream

use axum::http::HeaderMap;
use serde::{Deserialize, Serialize};

/// Minimum valid token count threshold.
/// Responses with fewer tokens than this are considered "empty".
const MIN_VALID_TOKENS: u32 = 1;

/// Response quality level with detailed metrics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResponseQuality {
    /// Complete healthy response with tokens and latency info
    Healthy {
        /// Number of tokens generated (input + output for some providers)
        tokens: u32,
        /// Response latency in milliseconds
        latency_ms: u64,
        /// Whether this was a streaming response
        is_streaming: bool,
    },
    /// Partial response - stream interrupted but some tokens received
    Partial {
        /// Tokens received before interruption
        received_tokens: u32,
        /// Expected tokens (if known from response)
        expected_tokens: Option<u32>,
        /// Reason for partial response (if determinable)
        interruption_reason: Option<String>,
    },
    /// Empty response - HTTP success but no tokens generated
    Empty {
        /// HTTP status code (usually 200)
        http_status: u16,
        /// Raw response body for debugging (optional)
        raw_body: Option<String>,
        /// Content-Type header value
        content_type: Option<String>,
    },
    /// Malformed response - could not parse valid structure
    Malformed {
        /// Parsing error description
        error: String,
        /// Raw response body
        raw: String,
        /// HTTP status code
        http_status: u16,
    },
    /// Explicit upstream error response
    UpstreamError {
        /// HTTP status code
        code: u16,
        /// Error message from upstream
        message: String,
        /// Classified error type for circuit breaker
        error_type: UpstreamErrorType,
    },
}

/// Classification of upstream error types for differentiated handling.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UpstreamErrorType {
    /// Rate limited (429) - temporary, should retry later
    RateLimited {
        /// Scope of rate limit (account or model level)
        scope: RateLimitScope,
        /// Seconds until retry is allowed
        retry_after: Option<u64>,
    },
    /// Authentication failed (401) - permanent, needs key update
    AuthFailed,
    /// Payment required / quota exhausted (402) - needs billing action
    PaymentRequired,
    /// Model not found (404) - configuration issue
    ModelNotFound,
    /// Server error (500) - temporary upstream issue
    ServerError,
    /// Gateway error (502/503/504) - temporary infrastructure issue
    GatewayError,
    /// Request timeout - upstream took too long
    Timeout,
    /// Connection failed - network/DNS/TLS issue
    ConnectionError,
    /// Service overloaded (Anthropic specific)
    Overloaded {
        /// Estimated wait time if provided
        retry_after: Option<u64>,
    },
}

/// Scope of rate limit or error impact.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub enum RateLimitScope {
    /// Rate limit applies at account level (affects all models)
    Account,
    /// Rate limit applies at model level (specific model only)
    Model,
    /// Scope is unknown
    #[default]
    Unknown,
}

/// Configuration for response quality detection.
#[derive(Debug, Clone)]
pub struct QualityDetectorConfig {
    /// Minimum token count to consider response as healthy
    pub min_valid_tokens: u32,
    /// Whether to capture raw body for empty/malformed responses
    pub capture_raw_body: bool,
    /// Maximum raw body length to capture (to avoid memory bloat)
    pub max_raw_body_len: usize,
    /// Latency threshold for "slow" response (ms)
    pub slow_latency_threshold_ms: u64,
}

impl Default for QualityDetectorConfig {
    fn default() -> Self {
        Self {
            min_valid_tokens: MIN_VALID_TOKENS,
            capture_raw_body: true,
            max_raw_body_len: 1024, // 1KB max
            slow_latency_threshold_ms: 5000, // 5 seconds
        }
    }
}

/// Response quality detector that classifies upstream responses.
pub struct ResponseQualityDetector {
    config: QualityDetectorConfig,
}

impl ResponseQualityDetector {
    /// Create a new detector with default configuration.
    pub fn new() -> Self {
        Self {
            config: QualityDetectorConfig::default(),
        }
    }

    /// Create a detector with custom configuration.
    pub fn with_config(config: QualityDetectorConfig) -> Self {
        Self { config }
    }

    /// Detect response quality from HTTP response.
    ///
    /// # Arguments
    /// * `http_status` - HTTP status code
    /// * `headers` - Response headers
    /// * `body` - Response body as string
    /// * `latency_ms` - Response latency in milliseconds
    /// * `is_streaming` - Whether this was a streaming response
    /// * `channel_type` - Provider type (openai, anthropic, etc.)
    ///
    /// # Returns
    /// Classified ResponseQuality enum
    pub fn detect(
        &self,
        http_status: u16,
        headers: &HeaderMap,
        body: &str,
        latency_ms: u64,
        is_streaming: bool,
        channel_type: &str,
    ) -> ResponseQuality {
        // 1. Check HTTP status - non-success codes are upstream errors
        if http_status >= 400 {
            return self.classify_upstream_error(http_status, headers, body, channel_type);
        }

        // 2. Handle empty body
        if body.is_empty() {
            return ResponseQuality::Empty {
                http_status,
                raw_body: None,
                content_type: headers
                    .get("content-type")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string()),
            };
        }

        // 3. Try to parse tokens from response
        match self.parse_tokens(body, channel_type) {
            Ok(tokens) if tokens >= self.config.min_valid_tokens => {
                ResponseQuality::Healthy {
                    tokens,
                    latency_ms,
                    is_streaming,
                }
            }
            Ok(tokens) => {
                // Tokens below threshold - treat as empty
                ResponseQuality::Empty {
                    http_status,
                    raw_body: self.capture_raw_body(body),
                    content_type: headers
                        .get("content-type")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string()),
                }
            }
            Err(error) => {
                // Parse failed - malformed response
                ResponseQuality::Malformed {
                    error,
                    raw: self.truncate_raw_body(body),
                    http_status,
                }
            }
        }
    }

    /// Detect quality for streaming chunk (partial response detection).
    ///
    /// Streaming responses may be interrupted mid-stream. This method
    /// helps detect partial responses.
    pub fn detect_stream_chunk(
        &self,
        chunk_data: &str,
        total_received_tokens: u32,
        is_final_chunk: bool,
        channel_type: &str,
    ) -> Option<ResponseQuality> {
        if is_final_chunk {
            // Final chunk - check if we got enough tokens
            if total_received_tokens >= self.config.min_valid_tokens {
                return None; // Success, no quality issue
            } else {
                // Stream finished but no/minimal tokens - empty response
                return Some(ResponseQuality::Empty {
                    http_status: 200,
                    raw_body: self.capture_raw_body(chunk_data),
                    content_type: None,
                });
            }
        }

        // Non-final chunk - check for error indicators in chunk
        if chunk_data.contains("error") || chunk_data.contains("Error") {
            // Try to parse error from chunk
            if let Some(error_type) = self.parse_stream_error(chunk_data, channel_type) {
                return Some(ResponseQuality::UpstreamError {
                    code: 400, // Assume client error for stream errors
                    message: "Stream error detected".to_string(),
                    error_type,
                });
            }
        }

        None // No quality issue detected in this chunk
    }

    /// Classify upstream HTTP error response.
    fn classify_upstream_error(
        &self,
        http_status: u16,
        headers: &HeaderMap,
        body: &str,
        channel_type: &str,
    ) -> ResponseQuality {
        let error_type = match http_status {
            429 => {
                // Rate limited - extract retry_after and scope
                let retry_after = headers
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok());
                let scope = self.detect_rate_limit_scope(body, channel_type);
                UpstreamErrorType::RateLimited { scope, retry_after }
            }
            401 => UpstreamErrorType::AuthFailed,
            402 => UpstreamErrorType::PaymentRequired,
            404 => UpstreamErrorType::ModelNotFound,
            500 => UpstreamErrorType::ServerError,
            502 | 503 | 504 => UpstreamErrorType::GatewayError,
            code if code >= 500 => UpstreamErrorType::ServerError,
            _ => UpstreamErrorType::ServerError, // Default for unknown errors
        };

        // Parse error message from body
        let message = self.parse_error_message(body, channel_type);

        ResponseQuality::UpstreamError {
            code: http_status,
            message,
            error_type,
        }
    }

    /// Parse token count from response body based on provider format.
    fn parse_tokens(&self, body: &str, channel_type: &str) -> Result<u32, String> {
        match channel_type.to_lowercase().as_str() {
            "openai" | "azure" => self.parse_openai_tokens(body),
            "anthropic" | "claude" => self.parse_anthropic_tokens(body),
            "gemini" | "vertex" => self.parse_gemini_tokens(body),
            _ => self.parse_generic_tokens(body),
        }
    }

    /// Parse tokens from OpenAI format response.
    fn parse_openai_tokens(&self, body: &str) -> Result<u32, String> {
        let json: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("JSON parse error: {e}"))?;

        // Check for error response
        if json.get("error").is_some() {
            return Err("Response contains error".to_string());
        }

        // Try usage field first
        if let Some(usage) = json.get("usage") {
            let total = usage
                .get("total_tokens")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            if let Some(t) = total {
                return Ok(t);
            }
            // Fallback: sum prompt + completion
            let prompt = usage.get("prompt_tokens").and_then(|p| p.as_u64()).unwrap_or(0);
            let completion = usage.get("completion_tokens").and_then(|c| c.as_u64()).unwrap_or(0);
            return Ok((prompt + completion) as u32);
        }

        // Streaming format - count from choices delta
        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
            // For streaming, we may not have usage yet
            // Check if there's content in delta
            for choice in choices {
                if let Some(delta) = choice.get("delta") {
                    if delta.get("content").is_some() {
                        // Has content, assume at least 1 token
                        return Ok(1);
                    }
                }
                // Non-streaming: check message content
                if let Some(message) = choice.get("message") {
                    if message.get("content").is_some() {
                        return Ok(1); // Has content
                    }
                }
            }
        }

        Err("No tokens found in OpenAI response".to_string())
    }

    /// Parse tokens from Anthropic format response.
    fn parse_anthropic_tokens(&self, body: &str) -> Result<u32, String> {
        let json: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("JSON parse error: {e}"))?;

        // Check for error type
        if json.get("type").and_then(|t| t.as_str()) == Some("error") {
            return Err("Response contains error".to_string());
        }

        // Try usage field
        if let Some(usage) = json.get("usage") {
            let output = usage.get("output_tokens").and_then(|t| t.as_u64());
            let input = usage.get("input_tokens").and_then(|t| t.as_u64());
            if let (Some(o), Some(i)) = (output, input) {
                return Ok((o + i) as u32);
            }
            if let Some(o) = output {
                return Ok(o as u32);
            }
        }

        // Check for content blocks (streaming or non-streaming)
        if let Some(content) = json.get("content").and_then(|c| c.as_array()) {
            if !content.is_empty() {
                return Ok(1); // Has content
            }
        }

        // Streaming delta format
        if json.get("type").and_then(|t| t.as_str()) == Some("content_block_delta") {
            return Ok(1); // Streaming delta, has content
        }

        Err("No tokens found in Anthropic response".to_string())
    }

    /// Parse tokens from Gemini format response.
    fn parse_gemini_tokens(&self, body: &str) -> Result<u32, String> {
        let json: serde_json::Value = serde_json::from_str(body)
            .map_err(|e| format!("JSON parse error: {e}"))?;

        // Check for error
        if json.get("error").is_some() {
            return Err("Response contains error".to_string());
        }

        // Try usageMetadata
        if let Some(usage) = json.get("usageMetadata") {
            let total = usage
                .get("totalTokenCount")
                .and_then(|t| t.as_u64())
                .map(|t| t as u32);
            if let Some(t) = total {
                return Ok(t);
            }
        }

        // Check for candidates
        if let Some(candidates) = json.get("candidates").and_then(|c| c.as_array()) {
            if !candidates.is_empty() {
                return Ok(1); // Has candidates
            }
        }

        Err("No tokens found in Gemini response".to_string())
    }

    /// Generic token parsing fallback.
    fn parse_generic_tokens(&self, body: &str) -> Result<u32, String> {
        // Try JSON parsing
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            // Common patterns
            if json.get("error").is_some() {
                return Err("Response contains error".to_string());
            }
            if json.get("choices").is_some() || json.get("content").is_some() {
                return Ok(1);
            }
            if let Some(usage) = json.get("usage") {
                if usage.get("total_tokens").is_some() {
                    return usage.get("total_tokens").and_then(|t| t.as_u64()).map(|t| t as u32)
                        .ok_or_else(|| "Could not parse total_tokens".to_string());
                }
            }
        }

        Err("Could not parse tokens from response".to_string())
    }

    /// Parse error message from response body.
    fn parse_error_message(&self, body: &str, _channel_type: &str) -> String {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
            // Try common error message paths
            let msg = json
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .or_else(|| json.get("message").and_then(|m| m.as_str()))
                .or_else(|| json.get("error").and_then(|e| e.as_str()));

            if let Some(m) = msg {
                return m.to_string();
            }
        }
        // Fallback: truncated raw body
        self.truncate_raw_body(body)
    }

    /// Detect rate limit scope from error response.
    fn detect_rate_limit_scope(&self, body: &str, _channel_type: &str) -> RateLimitScope {
        let body_lower = body.to_lowercase();
        if body_lower.contains("account") || body_lower.contains("organization") {
            RateLimitScope::Account
        } else if body_lower.contains("model") {
            RateLimitScope::Model
        } else {
            RateLimitScope::Unknown
        }
    }

    /// Parse error from streaming chunk.
    fn parse_stream_error(&self, chunk: &str, channel_type: &str) -> Option<UpstreamErrorType> {
        // Try to parse JSON from SSE data line
        let json_str = if chunk.starts_with("data: ") {
            &chunk[6..]
        } else {
            chunk
        };

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
            // Check for error type
            let error_type = json.get("type").and_then(|t| t.as_str());
            
            match channel_type.to_lowercase().as_str() {
                "anthropic" | "claude" => {
                    match error_type {
                        Some("error") | Some("api_error") => {
                            // Check for overloaded
                            let msg = json.get("error").and_then(|e| e.as_str())
                                .or_else(|| json.get("message").and_then(|m| m.as_str()));
                            if let Some(m) = msg {
                                if m.to_lowercase().contains("overloaded") {
                                    return Some(UpstreamErrorType::Overloaded { retry_after: None });
                                }
                            }
                            return Some(UpstreamErrorType::ServerError);
                        }
                        Some("rate_limit_error") => {
                            return Some(UpstreamErrorType::RateLimited {
                                scope: RateLimitScope::Unknown,
                                retry_after: None,
                            });
                        }
                        _ => {}
                    }
                }
                "openai" | "azure" => {
                    if let Some(error) = json.get("error") {
                        let error_type_str = error.get("type").and_then(|t| t.as_str());
                        match error_type_str {
                            Some("rate_limit_exceeded") => {
                                return Some(UpstreamErrorType::RateLimited {
                                    scope: RateLimitScope::Unknown,
                                    retry_after: None,
                                });
                            }
                            Some(_) => {
                                return Some(UpstreamErrorType::ServerError);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        None
    }

    /// Capture raw body for debugging (respecting max length).
    fn capture_raw_body(&self, body: &str) -> Option<String> {
        if self.config.capture_raw_body {
            Some(self.truncate_raw_body(body))
        } else {
            None
        }
    }

    /// Truncate raw body to max length.
    fn truncate_raw_body(&self, body: &str) -> String {
        if body.len() > self.config.max_raw_body_len {
            format!("{}...", &body[..self.config.max_raw_body_len])
        } else {
            body.to_string()
        }
    }

    /// Convert ResponseQuality to a health score (0.0 ~ 1.0).
    ///
    /// This score feeds into the smart circuit breaker's health calculation.
    pub fn quality_to_health_score(quality: &ResponseQuality) -> f64 {
        match quality {
            ResponseQuality::Healthy { tokens, latency_ms, .. } => {
                // Base score is 1.0, penalize for slow latency
                let latency_penalty = if *latency_ms > 5000 {
                    0.7 // 30% penalty for slow responses
                } else if *latency_ms > 2000 {
                    0.9 // 10% penalty for moderately slow
                } else {
                    1.0
                };
                // Bonus for high token count
                let token_bonus: f64 = if *tokens > 1000 {
                    1.05 // 5% bonus
                } else {
                    1.0
                };
                latency_penalty * token_bonus.min(1.0)
            }
            ResponseQuality::Partial { received_tokens, .. } => {
                // Partial response - moderate penalty
                if *received_tokens > 0 {
                    0.5 // Got something, not complete failure
                } else {
                    0.1 // Nearly empty
                }
            }
            ResponseQuality::Empty { .. } => {
                0.0 // No tokens = zero health
            }
            ResponseQuality::Malformed { .. } => {
                0.1 // Parse failure = near-zero health
            }
            ResponseQuality::UpstreamError { error_type, .. } => {
                match error_type {
                    UpstreamErrorType::RateLimited { .. } => 0.3, // Temporary
                    UpstreamErrorType::Overloaded { .. } => 0.3, // Temporary
                    UpstreamErrorType::ServerError => 0.2,
                    UpstreamErrorType::GatewayError => 0.2,
                    UpstreamErrorType::Timeout => 0.2,
                    UpstreamErrorType::ConnectionError => 0.1,
                    UpstreamErrorType::AuthFailed => 0.0, // Permanent failure
                    UpstreamErrorType::PaymentRequired => 0.0,
                    UpstreamErrorType::ModelNotFound => 0.0,
                }
            }
        }
    }
}

impl Default for ResponseQualityDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn test_detect_healthy_openai_response() {
        let detector = ResponseQualityDetector::new();
        let mut headers = HeaderMap::new();
        headers.insert("content-type", HeaderValue::from_static("application/json"));
        
        let body = r#"{"choices":[{"message":{"content":"Hello"}}],"usage":{"total_tokens":10}}"#;
        
        let quality = detector.detect(200, &headers, body, 100, false, "openai");
        
        match quality {
            ResponseQuality::Healthy { tokens, .. } => {
                assert_eq!(tokens, 10);
            }
            _ => panic!("Expected Healthy quality"),
        }
    }

    #[test]
    fn test_detect_empty_response() {
        let detector = ResponseQualityDetector::new();
        let headers = HeaderMap::new();
        
        let quality = detector.detect(200, &headers, "", 100, false, "openai");
        
        match quality {
            ResponseQuality::Empty { http_status, .. } => {
                assert_eq!(http_status, 200);
            }
            _ => panic!("Expected Empty quality"),
        }
    }

    #[test]
    fn test_detect_rate_limit_error() {
        let detector = ResponseQualityDetector::new();
        let mut headers = HeaderMap::new();
        headers.insert("retry-after", HeaderValue::from_static("30"));
        
        let body = r#"{"error":{"message":"Rate limit exceeded","type":"rate_limit_error"}}"#;
        
        let quality = detector.detect(429, &headers, body, 100, false, "anthropic");
        
        match quality {
            ResponseQuality::UpstreamError { code, error_type, .. } => {
                assert_eq!(code, 429);
                match error_type {
                    UpstreamErrorType::RateLimited { retry_after, .. } => {
                        assert_eq!(retry_after.clone(), Some(30));
                    }
                    _ => panic!("Expected RateLimited error type"),
                }
            }
            _ => panic!("Expected UpstreamError quality"),
        }
    }

    #[test]
    fn test_health_score_calculation() {
        let healthy = ResponseQuality::Healthy {
            tokens: 100,
            latency_ms: 100,
            is_streaming: false,
        };
        assert_eq!(ResponseQualityDetector::quality_to_health_score(&healthy), 1.0);

        let slow_healthy = ResponseQuality::Healthy {
            tokens: 100,
            latency_ms: 6000,
            is_streaming: false,
        };
        assert_eq!(ResponseQualityDetector::quality_to_health_score(&slow_healthy), 0.7);

        let empty = ResponseQuality::Empty {
            http_status: 200,
            raw_body: None,
            content_type: None,
        };
        assert_eq!(ResponseQualityDetector::quality_to_health_score(&empty), 0.0);

        let rate_limited = ResponseQuality::UpstreamError {
            code: 429,
            message: "Rate limit".to_string(),
            error_type: UpstreamErrorType::RateLimited {
                scope: RateLimitScope::Unknown,
                retry_after: Some(30),
            },
        };
        assert_eq!(ResponseQualityDetector::quality_to_health_score(&rate_limited), 0.3);
    }
}
