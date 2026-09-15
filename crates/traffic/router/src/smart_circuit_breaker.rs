//! Smart Circuit Breaker Module
//!
//! This module implements an intelligent circuit breaker that uses error rates
//! instead of absolute failure counts, making it more robust for varying traffic loads.
//!
//! Key features:
//! - Error rate based triggering (not absolute count)
//! - Sliding window statistics
//! - Health score calculation
//! - Multi-level circuit breaking (Model vs Channel)

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::response_quality::{ResponseQuality, ResponseQualityDetector, UpstreamErrorType};

/// Default error rate threshold for circuit breaking (10%)
pub const DEFAULT_ERROR_RATE_THRESHOLD: f64 = 0.1;

/// Minimum requests before calculating error rate
pub const DEFAULT_MIN_REQUESTS: u32 = 10;

/// Default sliding window size (60 seconds)
pub const DEFAULT_WINDOW_SIZE_SECS: u64 = 60;

/// Default cooldown duration (60 seconds)
pub const DEFAULT_COOLDOWN_SECS: u64 = 60;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CircuitState {
    /// Circuit is closed - requests flow normally
    Closed,
    /// Circuit is open - requests are blocked
    Open,
    /// Circuit is half-open - allowing probe requests
    HalfOpen,
}

/// Level of circuit breaking
#[derive(Debug, Clone, PartialEq)]
pub enum TripLevel {
    /// No circuit breaking
    None,
    /// Degraded weight - reduce traffic
    Degraded {
        /// Weight multiplier (0.0 ~ 1.0)
        weight: f64,
    },
    /// Model-level circuit break - only affects specific model
    Model {
        /// When the circuit will reset
        until: Instant,
        /// Reason for the trip
        reason: String,
    },
    /// Channel-level circuit break - affects all models in channel
    Channel {
        /// When the circuit will reset
        until: Instant,
        /// Reason for the trip
        reason: String,
    },
}

/// Statistics for a time window
#[derive(Debug, Clone, Default)]
pub struct WindowStats {
    pub total_requests: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub total_latency_ms: u64,
}

impl WindowStats {
    pub fn error_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.failure_count as f64 / self.total_requests as f64
        }
    }

    pub fn avg_latency_ms(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.total_latency_ms as f64 / self.total_requests as f64
        }
    }
}

/// Request record for sliding window
#[derive(Debug, Clone)]
struct RequestRecord {
    timestamp: Instant,
    is_success: bool,
    latency_ms: u64,
    health_score: f64,
}

/// Configuration for smart circuit breaker
#[derive(Debug, Clone)]
pub struct SmartCircuitBreakerConfig {
    /// Error rate threshold (0.0 ~ 1.0)
    pub error_rate_threshold: f64,
    /// Minimum requests before calculating error rate
    pub min_requests: u32,
    /// Sliding window size
    pub window_size: Duration,
    /// Cooldown duration when circuit opens
    pub cooldown_duration: Duration,
    /// Health score threshold for degradation
    pub health_degraded_threshold: f64,
    /// Health score threshold for circuit break
    pub health_break_threshold: f64,
}

impl Default for SmartCircuitBreakerConfig {
    fn default() -> Self {
        Self {
            error_rate_threshold: DEFAULT_ERROR_RATE_THRESHOLD,
            min_requests: DEFAULT_MIN_REQUESTS,
            window_size: Duration::from_secs(DEFAULT_WINDOW_SIZE_SECS),
            cooldown_duration: Duration::from_secs(DEFAULT_COOLDOWN_SECS),
            health_degraded_threshold: 0.7,
            health_break_threshold: 0.3,
        }
    }
}

/// Smart circuit breaker using error rates and sliding windows
pub struct SmartCircuitBreaker {
    /// Configuration
    config: SmartCircuitBreakerConfig,
    /// Request records in sliding window
    records: VecDeque<RequestRecord>,
    /// Current circuit state
    state: CircuitState,
    /// When the circuit will reset (if open)
    reset_at: Option<Instant>,
    /// Reason for circuit break
    trip_reason: Option<String>,
    /// Cumulative health score
    health_score: AtomicU64, // Stored as fixed-point (0-10000 = 0.0-1.0)
}

impl SmartCircuitBreaker {
    pub fn new(config: SmartCircuitBreakerConfig) -> Self {
        Self {
            config,
            records: VecDeque::new(),
            state: CircuitState::Closed,
            reset_at: None,
            trip_reason: None,
            health_score: AtomicU64::new(10000), // Start at 1.0
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(SmartCircuitBreakerConfig::default())
    }

    /// Record a response and update state
    pub fn record(&mut self, quality: &ResponseQuality, latency_ms: u64) {
        let now = Instant::now();
        let health_score = ResponseQualityDetector::quality_to_health_score(quality);
        let is_success = health_score > 0.5;

        // Add record
        self.records.push_back(RequestRecord {
            timestamp: now,
            is_success,
            latency_ms,
            health_score,
        });

        // Remove old records
        self.prune_old_records(now);

        // Update health score (exponential moving average)
        self.update_health_score(health_score);

        // Check for state transitions
        self.check_state_transition();
    }

    /// Get current circuit state
    pub fn state(&self) -> CircuitState {
        // Check if cooldown has passed
        if self.state == CircuitState::Open {
            if let Some(reset_at) = self.reset_at {
                if Instant::now() >= reset_at {
                    return CircuitState::HalfOpen;
                }
            }
        }
        self.state
    }

    /// Check if requests are allowed
    pub fn allow_request(&self) -> TripLevel {
        let now = Instant::now();

        match self.state {
            CircuitState::Closed => {
                // Check health score for degradation
                let health = self.get_health_score();
                if health < self.config.health_degraded_threshold {
                    TripLevel::Degraded { weight: health }
                } else {
                    TripLevel::None
                }
            }
            CircuitState::Open => {
                // Check if cooldown has passed
                if let Some(reset_at) = self.reset_at {
                    if now >= reset_at {
                        // Transition to HalfOpen - allow probe
                        TripLevel::Degraded { weight: 0.1 }
                    } else {
                        TripLevel::Channel {
                            until: reset_at,
                            reason: self.trip_reason.clone().unwrap_or_else(|| "Circuit open".to_string()),
                        }
                    }
                } else {
                    TripLevel::Channel {
                        until: now + self.config.cooldown_duration,
                        reason: "Circuit open".to_string(),
                    }
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests for probing
                TripLevel::Degraded { weight: 0.1 }
            }
        }
    }

    /// Get current health score (0.0 ~ 1.0)
    pub fn get_health_score(&self) -> f64 {
        let raw = self.health_score.load(Ordering::Relaxed);
        raw as f64 / 10000.0
    }

    /// Get window statistics
    pub fn get_stats(&self) -> WindowStats {
        let now = Instant::now();
        let cutoff = now - self.config.window_size;

        let mut stats = WindowStats::default();
        for record in &self.records {
            if record.timestamp >= cutoff {
                stats.total_requests += 1;
                if record.is_success {
                    stats.success_count += 1;
                } else {
                    stats.failure_count += 1;
                }
                stats.total_latency_ms += record.latency_ms;
            }
        }
        stats
    }

    /// Manual reset (for admin intervention)
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.reset_at = None;
        self.trip_reason = None;
        self.health_score.store(10000, Ordering::Relaxed);
    }

    /// Manual trip (for admin intervention)
    pub fn trip(&mut self, reason: &str, duration: Duration) {
        self.state = CircuitState::Open;
        self.reset_at = Some(Instant::now() + duration);
        self.trip_reason = Some(reason.to_string());
    }

    // --- Private methods ---

    fn prune_old_records(&mut self, now: Instant) {
        let cutoff = now - self.config.window_size;
        while let Some(front) = self.records.front() {
            if front.timestamp < cutoff {
                self.records.pop_front();
            } else {
                break;
            }
        }
    }

    fn update_health_score(&self, new_score: f64) {
        // Exponential moving average with alpha = 0.1
        const ALPHA: f64 = 0.1;
        let current = self.get_health_score();
        let updated = ALPHA * new_score + (1.0 - ALPHA) * current;
        let raw = (updated * 10000.0).clamp(0.0, 10000.0) as u64;
        self.health_score.store(raw, Ordering::Relaxed);
    }

    fn check_state_transition(&mut self) {
        let stats = self.get_stats();

        // Need minimum requests to make decision
        if stats.total_requests < self.config.min_requests as u64 {
            return;
        }

        let error_rate = stats.error_rate();
        let health = self.get_health_score();

        match self.state {
            CircuitState::Closed => {
                // Check if we should trip
                if error_rate >= self.config.error_rate_threshold {
                    self.trip_circuit(&format!(
                        "Error rate {:.1}% exceeded threshold {:.1}%",
                        error_rate * 100.0,
                        self.config.error_rate_threshold * 100.0
                    ));
                } else if health < self.config.health_break_threshold {
                    self.trip_circuit(&format!(
                        "Health score {:.2} below threshold {:.2}",
                        health, self.config.health_break_threshold
                    ));
                }
            }
            CircuitState::Open => {
                // Handled in allow_request()
            }
            CircuitState::HalfOpen => {
                // Check if we should close or re-open
                if health > self.config.health_degraded_threshold {
                    // Recovery successful
                    self.state = CircuitState::Closed;
                    self.reset_at = None;
                    self.trip_reason = None;
                    tracing::info!("Circuit recovered to Closed state");
                } else if health < self.config.health_break_threshold {
                    // Recovery failed, re-open
                    self.trip_circuit("Recovery probe failed");
                }
            }
        }
    }

    fn trip_circuit(&mut self, reason: &str) {
        self.state = CircuitState::Open;
        self.reset_at = Some(Instant::now() + self.config.cooldown_duration);
        self.trip_reason = Some(reason.to_string());
        tracing::warn!("Circuit tripped: {}", reason);
    }
}

impl Default for SmartCircuitBreaker {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Multi-level circuit breaker manager
pub struct MultiLevelCircuitBreaker {
    /// Channel-level breaker
    channel_breaker: SmartCircuitBreaker,
    /// Model-level breakers
    model_breakers: std::collections::HashMap<String, SmartCircuitBreaker>,
}

impl MultiLevelCircuitBreaker {
    pub fn new(config: SmartCircuitBreakerConfig) -> Self {
        Self {
            channel_breaker: SmartCircuitBreaker::new(config),
            model_breakers: std::collections::HashMap::new(),
        }
    }

    /// Record response for a model
    pub fn record(&mut self, model: &str, quality: &ResponseQuality, latency_ms: u64) {
        // Record at model level
        self.model_breakers
            .entry(model.to_string())
            .or_insert_with(SmartCircuitBreaker::with_defaults)
            .record(quality, latency_ms);

        // Also record at channel level (for detecting channel-wide issues)
        self.channel_breaker.record(quality, latency_ms);
    }

    /// Check if request is allowed for a model
    pub fn allow_request(&self, model: &str) -> TripLevel {
        let now = Instant::now();

        // Check channel-level first
        match self.channel_breaker.allow_request() {
            TripLevel::Channel { until, reason } => {
                return TripLevel::Channel { until, reason };
            }
            TripLevel::Degraded { weight } if weight < 0.3 => {
                // Channel is heavily degraded
                return TripLevel::Degraded { weight };
            }
            _ => {}
        }

        // Check model-level
        if let Some(model_breaker) = self.model_breakers.get(model) {
            match model_breaker.allow_request() {
                TripLevel::Channel { until, reason } => {
                    // Convert channel-level trip to model-level
                    return TripLevel::Model { until, reason };
                }
                TripLevel::Degraded { weight } => {
                    return TripLevel::Degraded { weight };
                }
                TripLevel::Model { until, reason } => {
                    return TripLevel::Model { until, reason };
                }
                TripLevel::None => {}
            }
        }

        TripLevel::None
    }

    /// Get health score for a model
    pub fn get_health_score(&self, model: Option<&str>) -> f64 {
        match model {
            Some(m) => {
                self.model_breakers
                    .get(m)
                    .map(|b| b.get_health_score())
                    .unwrap_or_else(|| self.channel_breaker.get_health_score())
            }
            None => self.channel_breaker.get_health_score(),
        }
    }

    /// Get all model health scores
    pub fn get_all_health_scores(&self) -> std::collections::HashMap<String, f64> {
        self.model_breakers
            .iter()
            .map(|(model, breaker)| (model.clone(), breaker.get_health_score()))
            .collect()
    }

    /// Check if channel-level trip is needed
    pub fn should_trip_channel(&self) -> Option<String> {
        let stats = self.channel_breaker.get_stats();

        // Check how many models are unhealthy
        let unhealthy_count = self
            .model_breakers
            .values()
            .filter(|b| b.get_health_score() < 0.3)
            .count();

        let total_models = self.model_breakers.len();

        if total_models > 0 && unhealthy_count * 2 > total_models {
            // More than half of models are unhealthy
            Some(format!(
                "{} of {} models are unhealthy",
                unhealthy_count, total_models
            ))
        } else if stats.error_rate() > 0.3 {
            // Channel-wide high error rate
            Some(format!(
                "Channel error rate {:.1}%",
                stats.error_rate() * 100.0
            ))
        } else {
            None
        }
    }

    /// Manual reset
    pub fn reset(&mut self) {
        self.channel_breaker.reset();
        for breaker in self.model_breakers.values_mut() {
            breaker.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_rate_calculation() {
        let mut breaker = SmartCircuitBreaker::with_defaults();

        // Record 10 successes and 2 failures
        for _ in 0..10 {
            breaker.record(
                &ResponseQuality::Healthy {
                    tokens: 100,
                    latency_ms: 100,
                    is_streaming: false,
                },
                100,
            );
        }
        for _ in 0..2 {
            breaker.record(
                &ResponseQuality::Empty {
                    http_status: 200,
                    raw_body: None,
                    content_type: None,
                },
                100,
            );
        }

        let stats = breaker.get_stats();
        assert_eq!(stats.total_requests, 12);
        assert_eq!(stats.success_count, 10);
        assert_eq!(stats.failure_count, 2);

        let error_rate = stats.error_rate();
        assert!((error_rate - 0.1667).abs() < 0.01);
    }

    #[test]
    fn test_circuit_trip_on_high_error_rate() {
        let config = SmartCircuitBreakerConfig {
            error_rate_threshold: 0.5,
            min_requests: 5,
            ..Default::default()
        };
        let mut breaker = SmartCircuitBreaker::new(config);

        // Record high error rate
        for _ in 0..5 {
            breaker.record(
                &ResponseQuality::Empty {
                    http_status: 200,
                    raw_body: None,
                    content_type: None,
                },
                100,
            );
        }
        for _ in 0..5 {
            breaker.record(
                &ResponseQuality::Healthy {
                    tokens: 100,
                    latency_ms: 100,
                    is_streaming: false,
                },
                100,
            );
        }

        // Should trip due to 50% error rate
        assert_eq!(breaker.state(), CircuitState::Open);
    }

    #[test]
    fn test_degraded_weight() {
        let mut breaker = SmartCircuitBreaker::with_defaults();

        // Record some failures to lower health score
        for _ in 0..15 {
            breaker.record(
                &ResponseQuality::Partial {
                    received_tokens: 1,
                    expected_tokens: None,
                    interruption_reason: None,
                },
                100,
            );
        }

        let level = breaker.allow_request();
        match level {
            TripLevel::Degraded { weight } => {
                assert!(weight < 1.0);
            }
            _ => {}
        }
    }

    #[test]

    #[test]
    fn test_multi_level_breaker() {
        let config = SmartCircuitBreakerConfig {
            min_requests: 5,
            error_rate_threshold: 0.3,
            ..Default::default()
        };
        let mut breaker = MultiLevelCircuitBreaker::new(config);

        // Record failures for one model only
        for _ in 0..15 {
            breaker.record(
                "model-a",
                &ResponseQuality::Empty {
                    http_status: 200,
                    raw_body: None,
                    content_type: None,
                },
                100,
            );
        }

        // Record successes for another model to keep channel healthy
        for _ in 0..30 {
            breaker.record(
                "model-b",
                &ResponseQuality::Healthy {
                    tokens: 100,
                    latency_ms: 100,
                    is_streaming: false,
                },
                100,
            );
        }

        // model-a should be unhealthy
        let score_a = breaker.get_health_score(Some("model-a"));
        assert!(score_a < 0.5);

        // model-b should be healthy
        let score_b = breaker.get_health_score(Some("model-b"));
        assert!(score_b > 0.5);
    }
}
