//! Integration module for Smart Circuit Breaker with existing ChannelStateTracker
//!
//! This module provides the glue code to integrate the new smart circuit breaker
//! with the existing channel state tracking system.

use std::collections::HashMap;

use dashmap::DashMap;

use crate::response_quality::{ResponseQuality, ResponseQualityDetector};
use crate::smart_circuit_breaker::{MultiLevelCircuitBreaker, SmartCircuitBreakerConfig, TripLevel};

/// Integrated channel health manager combining:
/// - Response quality detection
/// - Multi-level circuit breaking
/// - Existing channel state tracking
pub struct ChannelHealthManager {
    /// Per-channel multi-level circuit breakers
    breakers: DashMap<i32, MultiLevelCircuitBreaker>,
    /// Configuration
    config: SmartCircuitBreakerConfig,
    /// Response quality detector
    detector: ResponseQualityDetector,
}

impl ChannelHealthManager {
    /// Create a new health manager with default configuration
    pub fn new() -> Self {
        Self {
            breakers: DashMap::new(),
            config: SmartCircuitBreakerConfig::default(),
            detector: ResponseQualityDetector::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: SmartCircuitBreakerConfig) -> Self {
        Self {
            breakers: DashMap::new(),
            config,
            detector: ResponseQualityDetector::new(),
        }
    }

    /// Process a response and update health state
    #[allow(clippy::too_many_arguments)]
    pub fn process_response(
        &self,
        channel_id: i32,
        model: &str,
        http_status: u16,
        headers: &axum::http::HeaderMap,
        body: &str,
        latency_ms: u64,
        is_streaming: bool,
        channel_type: &str,
    ) {
        // 1. Detect response quality
        let quality = self.detector.detect(
            http_status,
            headers,
            body,
            latency_ms,
            is_streaming,
            channel_type,
        );

        // 2. Record to circuit breaker
        self.breakers
            .entry(channel_id)
            .or_insert_with(|| MultiLevelCircuitBreaker::new(self.config.clone()))
            .record(model, &quality, latency_ms);

        // 3. Log significant events
        match &quality {
            ResponseQuality::Empty { .. } => {
                tracing::warn!(
                    channel_id,
                    model,
                    "Empty response detected for channel/model"
                );
            }
            ResponseQuality::UpstreamError { code, message, error_type } => {
                tracing::warn!(
                    channel_id,
                    model,
                    code,
                    ?error_type,
                    "Upstream error: {}",
                    message
                );
            }
            ResponseQuality::Malformed { error, .. } => {
                tracing::warn!(
                    channel_id,
                    model,
                    "Malformed response: {}",
                    error
                );
            }
            _ => {}
        }
    }

    /// Check if a request should be allowed for a channel/model
    pub fn check_availability(&self, channel_id: i32, model: &str) -> TripLevel {
        if let Some(breaker) = self.breakers.get(&channel_id) {
            breaker.allow_request(model)
        } else {
            TripLevel::None
        }
    }

    /// Get health score for a channel/model
    pub fn get_health_score(&self, channel_id: i32, model: Option<&str>) -> f64 {
        if let Some(breaker) = self.breakers.get(&channel_id) {
            breaker.get_health_score(model)
        } else {
            1.0 // Unknown channel = healthy
        }
    }

    /// Get all health scores for a channel
    pub fn get_channel_health(&self, channel_id: i32) -> Option<HashMap<String, f64>> {
        self.breakers.get(&channel_id).map(|b| b.get_all_health_scores())
    }

    /// Manual reset for a channel
    pub fn reset_channel(&self, channel_id: i32) {
        if let Some(mut breaker) = self.breakers.get_mut(&channel_id) {
            breaker.reset();
        }
    }

    /// Manual reset for all channels
    pub fn reset_all(&self) {
        for mut breaker in self.breakers.iter_mut() {
            breaker.reset();
        }
    }

    /// Get circuit breaker status for monitoring
    pub fn get_status(&self, channel_id: i32) -> Option<ChannelHealthStatus> {
        self.breakers.get(&channel_id).map(|breaker| {
            let model_scores = breaker.get_all_health_scores();
            let channel_score = breaker.get_health_score(None);
            
            ChannelHealthStatus {
                channel_id,
                channel_health_score: channel_score,
                model_health_scores: model_scores,
            }
        })
    }

    /// Get status for all channels
    pub fn get_all_status(&self) -> Vec<ChannelHealthStatus> {
        self.breakers
            .iter()
            .map(|entry| {
                let breaker = entry.value();
                let channel_id = *entry.key();
                let model_scores = breaker.get_all_health_scores();
                let channel_score = breaker.get_health_score(None);
                
                ChannelHealthStatus {
                    channel_id,
                    channel_health_score: channel_score,
                    model_health_scores: model_scores,
                }
            })
            .collect()
    }
}

impl Default for ChannelHealthManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Health status for a channel
#[derive(Debug, Clone)]
pub struct ChannelHealthStatus {
    pub channel_id: i32,
    pub channel_health_score: f64,
    pub model_health_scores: HashMap<String, f64>,
}

impl ChannelHealthStatus {
    /// Check if channel is healthy (all scores >= threshold)
    pub fn is_healthy(&self, threshold: f64) -> bool {
        if self.channel_health_score < threshold {
            return false;
        }
        for score in self.model_health_scores.values() {
            if *score < threshold {
                return false;
            }
        }
        true
    }

    /// Get unhealthy models
    pub fn get_unhealthy_models(&self, threshold: f64) -> Vec<&str> {
        self.model_health_scores
            .iter()
            .filter(|(_, &score)| score < threshold)
            .map(|(model, _)| model.as_str())
            .collect()
    }
}

/// Helper function to convert TripLevel to routing decision
pub fn trip_level_to_weight(level: &TripLevel) -> f64 {
    match level {
        TripLevel::None => 1.0,
        TripLevel::Degraded { weight } => *weight,
        TripLevel::Model { .. } => 0.0,
        TripLevel::Channel { .. } => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    #[test]
    fn test_channel_health_manager_basic() {
        let manager = ChannelHealthManager::new();
        let headers = HeaderMap::new();
        
        // Process healthy response
        manager.process_response(
            1,
            "gpt-4",
            200,
            &headers,
            r#"{"choices":[{"message":{"content":"Hello"}}],"usage":{"total_tokens":10}}"#,
            100,
            false,
            "openai",
        );

        // Should allow request
        let level = manager.check_availability(1, "gpt-4");
        assert!(matches!(level, TripLevel::None | TripLevel::Degraded { .. }));

        // Health score should be high
        let score = manager.get_health_score(1, Some("gpt-4"));
        assert!(score > 0.5);
    }

    #[test]
    fn test_empty_response_tracking() {
        let manager = ChannelHealthManager::new();
        let headers = HeaderMap::new();

        // Process multiple empty responses
        for _ in 0..15 {
            manager.process_response(
                1,
                "gpt-4",
                200,
                &headers,
                "",
                100,
                false,
                "openai",
            );
        }

        // Health score should be low
        let score = manager.get_health_score(1, Some("gpt-4"));
        assert!(score < 0.5);
    }

    #[test]
    fn test_model_level_isolation() {
        let manager = ChannelHealthManager::new();
        let headers = HeaderMap::new();

        // Process failures for model-a
        for _ in 0..15 {
            manager.process_response(
                1,
                "model-a",
                200,
                &headers,
                "",
                100,
                false,
                "openai",
            );
        }

        // Process successes for model-b
        for _ in 0..15 {
            manager.process_response(
                1,
                "model-b",
                200,
                &headers,
                r#"{"choices":[{"message":{"content":"Hello"}}],"usage":{"total_tokens":10}}"#,
                100,
                false,
                "openai",
            );
        }

        // model-a should have low score
        let score_a = manager.get_health_score(1, Some("model-a"));
        assert!(score_a < 0.5);

        // model-b should have high score
        let score_b = manager.get_health_score(1, Some("model-b"));
        assert!(score_b > 0.5);
    }

    #[test]
    fn test_health_status() {
        let manager = ChannelHealthManager::new();
        let headers = HeaderMap::new();

        manager.process_response(
            1,
            "gpt-4",
            200,
            &headers,
            r#"{"choices":[{"message":{"content":"Hello"}}],"usage":{"total_tokens":10}}"#,
            100,
            false,
            "openai",
        );

        let status = manager.get_status(1);
        assert!(status.is_some());
        
        let status = status.unwrap();
        assert_eq!(status.channel_id, 1);
        assert!(status.channel_health_score > 0.5);
    }
}
