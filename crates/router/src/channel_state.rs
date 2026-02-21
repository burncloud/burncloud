//! Channel State Tracker Module
//!
//! This module provides functionality for tracking the health and availability
//! of upstream channels and their models.

use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

// Import FailureType from circuit_breaker module
use crate::circuit_breaker::{FailureType, RateLimitScope};
use crate::adaptive_limit::{AdaptiveRateLimit, AdaptiveLimitConfig};

/// Represents the balance status of a channel's account.
///
/// This is used to track whether the channel has sufficient quota/credits
/// to handle requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BalanceStatus {
    /// Account balance is healthy and can handle requests
    Ok,
    /// Account balance is low, may need attention
    Low,
    /// Account balance is exhausted, cannot process requests
    Exhausted,
    /// Balance status is unknown (e.g., unable to check)
    Unknown,
}

impl Default for BalanceStatus {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Represents the operational status of a specific model within a channel.
///
/// This tracks whether a model is available for use or if it has issues
/// that prevent it from handling requests.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModelStatus {
    /// Model is available and can handle requests
    Available,
    /// Model is temporarily rate limited
    RateLimited,
    /// Model quota is exhausted for this channel
    QuotaExhausted,
    /// Model is not found on this channel
    ModelNotFound,
    /// Model is temporarily down (e.g., upstream issues)
    TemporarilyDown,
}

impl Default for ModelStatus {
    fn default() -> Self {
        Self::Available
    }
}

/// Represents the state of a specific model within a channel.
///
/// Tracks the model's operational status, rate limiting, errors, and performance metrics.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ModelState {
    /// The model name/identifier
    pub model: String,
    /// The channel ID this model belongs to
    pub channel_id: i32,
    /// Current operational status of the model
    pub status: ModelStatus,
    /// When the rate limit will expire (if rate limited)
    pub rate_limit_until: Option<Instant>,
    /// Last error message encountered
    pub last_error: Option<String>,
    /// When the last error occurred
    pub last_error_time: Option<Instant>,
    /// Number of successful requests
    pub success_count: u64,
    /// Number of failed requests
    pub failure_count: u64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Adaptive rate limiter (learns and adjusts to upstream limits)
    pub adaptive_limit: AdaptiveRateLimit,
}

impl ModelState {
    /// Create a new ModelState for a specific model and channel
    pub fn new(model: String, channel_id: i32) -> Self {
        Self {
            model,
            channel_id,
            status: ModelStatus::default(),
            rate_limit_until: None,
            last_error: None,
            last_error_time: None,
            success_count: 0,
            failure_count: 0,
            avg_latency_ms: 0.0,
            adaptive_limit: AdaptiveRateLimit::with_defaults(),
        }
    }

    /// Create a new ModelState with custom adaptive limit config
    #[allow(dead_code)]
    pub fn with_config(model: String, channel_id: i32, config: AdaptiveLimitConfig) -> Self {
        Self {
            model,
            channel_id,
            status: ModelStatus::default(),
            rate_limit_until: None,
            last_error: None,
            last_error_time: None,
            success_count: 0,
            failure_count: 0,
            avg_latency_ms: 0.0,
            adaptive_limit: AdaptiveRateLimit::new(config),
        }
    }
}

/// Represents the state of a channel (upstream provider).
///
/// Tracks channel-level status including authentication, balance, and rate limits,
/// as well as the state of individual models available through this channel.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ChannelState {
    /// The channel ID
    pub channel_id: i32,
    /// Whether authentication is valid for this channel
    pub auth_ok: bool,
    /// Balance status of the channel's account
    pub balance_status: BalanceStatus,
    /// When the account-level rate limit will expire
    pub account_rate_limit_until: Option<Instant>,
    /// State of individual models within this channel
    pub models: DashMap<String, ModelState>,
}

impl ChannelState {
    /// Create a new ChannelState for a specific channel
    pub fn new(channel_id: i32) -> Self {
        Self {
            channel_id,
            auth_ok: true, // Assume auth is OK until proven otherwise
            balance_status: BalanceStatus::default(),
            account_rate_limit_until: None,
            models: DashMap::new(),
        }
    }
}

/// Global tracker for all channel states.
///
/// This is the main entry point for querying and updating channel health.
/// Uses DashMap for lock-free concurrent access.
pub struct ChannelStateTracker {
    /// Map of channel_id to ChannelState
    channel_states: DashMap<i32, ChannelState>,
}

impl ChannelStateTracker {
    /// Create a new empty ChannelStateTracker
    pub fn new() -> Self {
        Self {
            channel_states: DashMap::new(),
        }
    }
}

impl Default for ChannelStateTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl ChannelStateTracker {
    /// Check if a specific channel and model is available for requests.
    ///
    /// This checks both channel-level status (auth, balance, account rate limits)
    /// and model-level status (model availability, model rate limits).
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to check
    /// * `model` - Optional model name to check (if None, only channel-level checks are performed)
    ///
    /// # Returns
    /// * `true` if the channel (and model if specified) is available
    /// * `false` if any condition prevents the channel/model from being used
    pub fn is_available(&self, channel_id: i32, model: Option<&str>) -> bool {
        // Get or create channel state
        let channel_state = self
            .channel_states
            .entry(channel_id)
            .or_insert_with(|| ChannelState::new(channel_id));

        // Check channel-level conditions
        if !channel_state.auth_ok {
            return false;
        }

        if channel_state.balance_status == BalanceStatus::Exhausted {
            return false;
        }

        // Check account-level rate limit
        if let Some(rate_limit_until) = channel_state.account_rate_limit_until {
            if rate_limit_until > Instant::now() {
                return false;
            }
        }

        // If no model specified, channel-level checks passed
        let model_name = match model {
            Some(m) => m,
            None => return true,
        };

        // Check model-level status
        if let Some(model_state) = channel_state.models.get(model_name) {
            // Check if model is available
            match model_state.status {
                ModelStatus::Available => {}
                ModelStatus::RateLimited
                | ModelStatus::QuotaExhausted
                | ModelStatus::ModelNotFound
                | ModelStatus::TemporarilyDown => return false,
            }

            // Check model-level rate limit
            if let Some(rate_limit_until) = model_state.rate_limit_until {
                if rate_limit_until > Instant::now() {
                    return false;
                }
            }

            // Check adaptive rate limiter availability
            if !model_state.adaptive_limit.check_available() {
                return false;
            }
        }

        true
    }

    /// Record an error for a specific channel and optionally a specific model.
    ///
    /// This updates the channel or model state based on the type of failure encountered.
    /// Channel-level failures (auth, payment) affect the entire channel,
    /// while model-level failures (rate limits, model not found) affect only specific models.
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID where the error occurred
    /// * `model` - Optional model name if the error is model-specific
    /// * `failure_type` - The type of failure that occurred
    /// * `error_message` - Human-readable error message
    pub fn record_error(
        &self,
        channel_id: i32,
        model: Option<&str>,
        failure_type: &FailureType,
        error_message: &str,
    ) {
        // Get or create channel state
        let mut channel_state = self
            .channel_states
            .entry(channel_id)
            .or_insert_with(|| ChannelState::new(channel_id));

        let now = Instant::now();

        // Handle channel-level failures
        match failure_type {
            FailureType::AuthFailed => {
                channel_state.auth_ok = false;
                channel_state.models.iter_mut().for_each(|mut m| {
                    m.status = ModelStatus::TemporarilyDown;
                });
            }
            FailureType::PaymentRequired => {
                channel_state.balance_status = BalanceStatus::Exhausted;
            }
            FailureType::RateLimited { scope, retry_after } => {
                let retry_after_duration = retry_after
                    .map(|s| Duration::from_secs(s))
                    .unwrap_or(Duration::from_secs(60)); // Default 60s if not specified

                let retry_until = now + retry_after_duration;

                match scope {
                    RateLimitScope::Account => {
                        channel_state.account_rate_limit_until = Some(retry_until);
                    }
                    RateLimitScope::Model => {
                        if let Some(model_name) = model {
                            let mut model_state = channel_state
                                .models
                                .entry(model_name.to_string())
                                .or_insert_with(|| {
                                    ModelState::new(model_name.to_string(), channel_id)
                                });
                            model_state.status = ModelStatus::RateLimited;
                            model_state.rate_limit_until = Some(retry_until);
                            model_state.last_error = Some(error_message.to_string());
                            model_state.last_error_time = Some(now);
                            model_state.failure_count += 1;
                            // Update adaptive rate limiter
                            model_state.adaptive_limit.on_rate_limited(*retry_after);
                        }
                    }
                    RateLimitScope::Unknown => {
                        // If scope is unknown, treat as account-level to be safe
                        channel_state.account_rate_limit_until = Some(retry_until);
                        // Also update model-level adaptive limiter if model is specified
                        if let Some(model_name) = model {
                            let mut model_state = channel_state
                                .models
                                .entry(model_name.to_string())
                                .or_insert_with(|| {
                                    ModelState::new(model_name.to_string(), channel_id)
                                });
                            model_state.adaptive_limit.on_rate_limited(*retry_after);
                        }
                    }
                }
            }
            FailureType::ModelNotFound => {
                if let Some(model_name) = model {
                    let mut model_state = channel_state
                        .models
                        .entry(model_name.to_string())
                        .or_insert_with(|| ModelState::new(model_name.to_string(), channel_id));
                    model_state.status = ModelStatus::ModelNotFound;
                    model_state.last_error = Some(error_message.to_string());
                    model_state.last_error_time = Some(now);
                    model_state.failure_count += 1;
                }
            }
            FailureType::ServerError | FailureType::Timeout => {
                // These are transient errors, just update the model state if available
                if let Some(model_name) = model {
                    let mut model_state = channel_state
                        .models
                        .entry(model_name.to_string())
                        .or_insert_with(|| ModelState::new(model_name.to_string(), channel_id));
                    model_state.status = ModelStatus::TemporarilyDown;
                    model_state.last_error = Some(error_message.to_string());
                    model_state.last_error_time = Some(now);
                    model_state.failure_count += 1;
                }
            }
        }
    }

    /// Record a successful request for a specific channel and model.
    ///
    /// This updates success counts and average latency metrics.
    /// If the model was previously in a degraded state, this may restore it to available.
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID where the success occurred
    /// * `model` - Optional model name if the success is model-specific
    /// * `latency_ms` - The latency of the successful request in milliseconds
    /// * `upstream_limit` - Optional rate limit learned from upstream response headers
    pub fn record_success(&self, channel_id: i32, model: Option<&str>, latency_ms: u64, upstream_limit: Option<u32>) {
        // Get or create channel state
        let channel_state = self
            .channel_states
            .entry(channel_id)
            .or_insert_with(|| ChannelState::new(channel_id));

        // If no model specified, nothing more to do
        let model_name = match model {
            Some(m) => m,
            None => return,
        };

        // Update model state
        let mut model_state = channel_state
            .models
            .entry(model_name.to_string())
            .or_insert_with(|| ModelState::new(model_name.to_string(), channel_id));

        // Update success count
        model_state.success_count += 1;

        // Update average latency using exponential moving average
        let alpha = 0.2; // Weight for new value
        model_state.avg_latency_ms =
            alpha * latency_ms as f64 + (1.0 - alpha) * model_state.avg_latency_ms;

        // If the model was temporarily down, restore it to available
        // (successful request indicates the issue is resolved)
        if model_state.status == ModelStatus::TemporarilyDown {
            model_state.status = ModelStatus::Available;
            model_state.last_error = None;
            model_state.last_error_time = None;
        }

        // Clear rate limit if it has passed
        if let Some(rate_limit_until) = model_state.rate_limit_until {
            if rate_limit_until <= Instant::now() {
                model_state.rate_limit_until = None;
                if model_state.status == ModelStatus::RateLimited {
                    model_state.status = ModelStatus::Available;
                }
            }
        }

        // Update adaptive rate limiter with learned upstream limit
        model_state.adaptive_limit.on_success(upstream_limit);
    }

    /// Filter a list of candidate channels to return only available ones.
    ///
    /// This method takes a list of candidate channel IDs and filters out any
    /// that are currently unavailable based on their state (auth, balance, rate limits).
    /// Optionally filters by model availability as well.
    ///
    /// # Arguments
    /// * `candidates` - List of candidate channel IDs to filter
    /// * `model` - Optional model name to check availability for
    ///
    /// # Returns
    /// A vector of channel IDs that are available for the given model (if specified)
    #[allow(dead_code)]
    pub fn get_available_channels(&self, candidates: &[i32], model: Option<&str>) -> Vec<i32> {
        candidates
            .iter()
            .filter(|&&channel_id| self.is_available(channel_id, model))
            .copied()
            .collect()
    }

    /// Calculate a health score for a channel.
    ///
    /// Higher scores indicate healthier channels.
    /// Considers success rate, average latency, and current status.
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to calculate score for
    /// * `model` - Optional model name for model-specific scoring
    ///
    /// # Returns
    /// A health score (higher is better). Default is 1.0 for unknown channels.
    pub fn get_health_score(&self, channel_id: i32, model: Option<&str>) -> f64 {
        // Get channel state
        let channel_state = match self.channel_states.get(&channel_id) {
            Some(state) => state,
            None => return 1.0, // Unknown channel, neutral score
        };

        let mut score = 1.0;

        // Penalize if auth is not OK
        if !channel_state.auth_ok {
            score *= 0.1;
        }

        // Penalize based on balance status
        match channel_state.balance_status {
            BalanceStatus::Ok => {}
            BalanceStatus::Low => score *= 0.7,
            BalanceStatus::Exhausted => score *= 0.1,
            BalanceStatus::Unknown => {}
        }

        // If model is specified, factor in model-level stats
        if let Some(model_name) = model {
            if let Some(model_state) = channel_state.models.get(model_name) {
                // Calculate success rate
                let total = model_state.success_count + model_state.failure_count;
                if total > 0 {
                    let success_rate = model_state.success_count as f64 / total as f64;
                    score *= success_rate;
                }

                // Factor in average latency (lower is better)
                if model_state.avg_latency_ms > 0.0 {
                    // Normalize: 100ms = 1.0, 1000ms = 0.5, 5000ms = 0.1
                    let latency_factor = 100.0 / (100.0 + model_state.avg_latency_ms);
                    score *= latency_factor;
                }

                // Penalize based on model status
                match model_state.status {
                    ModelStatus::Available => {}
                    ModelStatus::RateLimited => score *= 0.3,
                    ModelStatus::QuotaExhausted => score *= 0.1,
                    ModelStatus::ModelNotFound => score *= 0.0,
                    ModelStatus::TemporarilyDown => score *= 0.2,
                }
            }
        }

        score
    }

    /// Get all channel states for monitoring/health reporting.
    ///
    /// Returns a vector of (channel_id, ChannelState) pairs.
    pub fn get_all_states(&self) -> Vec<(i32, ChannelState)> {
        self.channel_states
            .iter()
            .map(|entry| (*entry.key(), entry.value().clone()))
            .collect()
    }
}
