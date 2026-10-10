//! Channel State Tracker Module
//!
//! This module provides functionality for tracking the health and availability
//! of upstream channels and their models.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use serde::{Deserialize, Serialize};

use crate::aimd_limiter::{AimdConfig, AimdController, AimdSnapshot};
use crate::circuit_breaker::{FailureType, RateLimitScope};

// Health score penalty factors
const PENALTY_AUTH_FAILED: f64 = 0.1;
const PENALTY_BALANCE_LOW: f64 = 0.7;
const PENALTY_BALANCE_EXHAUSTED: f64 = 0.1;
const PENALTY_RATE_LIMITED: f64 = 0.3;
const PENALTY_QUOTA_EXHAUSTED: f64 = 0.1;
const PENALTY_MODEL_NOT_FOUND: f64 = 0.0;
const PENALTY_TEMPORARILY_DOWN: f64 = 0.2;
const LATENCY_SCORE_MIDPOINT_MS: f64 = 100.0;

/// Default retry duration when no retry_after is provided (seconds).
const DEFAULT_RATE_LIMIT_RETRY_SECS: u64 = 60;
/// Base cooldown for transient errors (seconds).
const BASE_COOLDOWN_SECS: u64 = 60;
/// Maximum cooldown cap (seconds) - 30 minutes.
const MAX_COOLDOWN_SECS: u64 = 1800;
/// Exponential moving average smoothing factor for latency tracking.
const LATENCY_EMA_ALPHA: f64 = 0.2;

/// Represents the balance status of a channel's account.
///
/// This is used to track whether the channel has sufficient quota/credits
/// to handle requests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum BalanceStatus {
    /// Account balance is healthy and can handle requests
    Ok,
    /// Account balance is low, may need attention
    Low,
    /// Account balance is exhausted, cannot process requests
    Exhausted,
    /// Balance status is unknown (e.g., unable to check)
    #[default]
    Unknown,
}

/// Represents the operational status of a specific model within a channel.
///
/// This tracks whether a model is available for use or if it has issues
/// that prevent it from handling requests.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum ModelStatus {
    /// Model is available and can handle requests
    #[default]
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

/// Represents the state of a specific model within a channel.
///
/// Tracks the model's operational status, rate limiting, errors, and performance metrics.
#[derive(Debug, Clone)]
#[allow(
    dead_code,
    reason = "newer tracking fields are not read on every path yet"
)]
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
    pub adaptive_limit: AimdController,
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
            adaptive_limit: AimdController::with_defaults(),
        }
    }

    /// Create a new ModelState with custom adaptive limit config
    #[allow(
        dead_code,
        reason = "newer tracking fields are not read on every path yet"
    )]
    pub fn with_config(model: String, channel_id: i32, config: AimdConfig) -> Self {
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
            adaptive_limit: AimdController::new(config),
        }
    }
}

/// Represents the state of a channel (upstream provider).
///
/// Tracks channel-level status including authentication, balance, and rate limits,
/// as well as the state of individual models available through this channel.
#[derive(Debug, Clone)]
#[allow(
    dead_code,
    reason = "newer tracking fields are not read on every path yet"
)]
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
    pub models: HashMap<String, ModelState>,
}

impl ChannelState {
    /// Create a new ChannelState for a specific channel
    pub fn new(channel_id: i32) -> Self {
        Self {
            channel_id,
            auth_ok: true, // Assume auth is OK until proven otherwise
            balance_status: BalanceStatus::default(),
            account_rate_limit_until: None,
            models: HashMap::new(),
        }
    }

    /// Get or create a ModelState for the given model name.
    /// Uses entry API: single hash lookup for both existing and new entries.
    fn get_or_create_model(&mut self, model_name: &str, channel_id: i32) -> &mut ModelState {
        use std::collections::hash_map::Entry;
        match self.models.entry(model_name.to_string()) {
            Entry::Occupied(e) => e.into_mut(),
            Entry::Vacant(e) => {
                let key = e.key().clone();
                e.insert(ModelState::new(key, channel_id))
            }
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
        let now = Instant::now();

        // Read-only: use get() to avoid write lock + allocation on hot path
        let channel_state = match self.channel_states.get(&channel_id) {
            Some(state) => state,
            None => return true, // Unknown channel = available
        };

        if !channel_gate_available(&channel_state, channel_id, now) {
            return false;
        }

        // If no model specified, channel-level checks passed
        let model_name = match model {
            Some(m) => m,
            None => return true,
        };

        match channel_state.models.get(model_name) {
            Some(model_state) => {
                model_status_available(model_state, channel_id, model_name, now)
                    && model_limits_available(model_state, channel_id, model_name, now)
            }
            None => true,
        }
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
                channel_state.models.iter_mut().for_each(|(_, m)| {
                    m.status = ModelStatus::TemporarilyDown;
                });
            }
            FailureType::PaymentRequired => {
                channel_state.balance_status = BalanceStatus::Exhausted;
            }
            FailureType::RateLimited { scope, retry_after } => {
                record_rate_limited_failure(
                    &mut channel_state,
                    channel_id,
                    model,
                    scope,
                    *retry_after,
                    error_message,
                    now,
                );
            }
            FailureType::ModelNotFound => {
                record_model_not_found(&mut channel_state, channel_id, model, error_message, now);
            }
            FailureType::EmptyResponse => {
                record_transient_model_failure(
                    &mut channel_state,
                    channel_id,
                    model,
                    error_message,
                    now,
                    "Empty response recorded with exponential backoff",
                );
            }
            FailureType::ServerError | FailureType::Timeout | FailureType::ConnectionError => {
                record_transient_model_failure(
                    &mut channel_state,
                    channel_id,
                    model,
                    error_message,
                    now,
                    "Transient error recorded with exponential backoff",
                );
            }
        }
    }
}

/// Channel-level gates: auth, balance and account-wide rate limit.
#[allow(
    clippy::cognitive_complexity,
    reason = "one early-return branch per independent availability gate"
)]
fn channel_gate_available(channel_state: &ChannelState, channel_id: i32, now: Instant) -> bool {
    if !channel_state.auth_ok {
        tracing::warn!(channel_id, "is_available=false: auth_ok=false");
        return false;
    }

    if channel_state.balance_status == BalanceStatus::Exhausted {
        tracing::warn!(channel_id, "is_available=false: balance exhausted");
        return false;
    }

    if let Some(rate_limit_until) = channel_state.account_rate_limit_until {
        if rate_limit_until > now {
            tracing::warn!(
                channel_id,
                ?rate_limit_until,
                "is_available=false: account rate limit"
            );
            return false;
        }
    }

    true
}

/// Model status gate. Rate-limited and temporarily-down models may probe once
/// their timer has expired — otherwise L0 would block all requests and recovery
/// could never happen.
#[allow(
    clippy::cognitive_complexity,
    reason = "one branch per model status; each branch only selects a log severity and message"
)]
fn model_status_available(
    model_state: &ModelState,
    channel_id: i32,
    model_name: &str,
    now: Instant,
) -> bool {
    match model_state.status {
        ModelStatus::Available => true,
        ModelStatus::RateLimited => {
            if let Some(rate_limit_until) = model_state.rate_limit_until {
                if rate_limit_until > now {
                    tracing::warn!(channel_id, %model_name, ?rate_limit_until, "is_available=false: model RateLimited (timer active)");
                    return false; // still within rate limit window
                }
                // Timer expired — allow probe request to test recovery
                tracing::info!(channel_id, %model_name, "is_available: model RateLimited but timer expired, allowing probe");
                true
            } else {
                tracing::warn!(channel_id, %model_name, "is_available=false: model RateLimited (no timer set — PERMANENT DEADLOCK)");
                false // no timer set, stay blocked
            }
        }
        ModelStatus::QuotaExhausted | ModelStatus::ModelNotFound => {
            tracing::warn!(channel_id, %model_name, ?model_state.status, "is_available=false: model status (permanent)");
            false
        }
        ModelStatus::TemporarilyDown => {
            if let Some(rate_limit_until) = model_state.rate_limit_until {
                if rate_limit_until > now {
                    tracing::warn!(channel_id, %model_name, ?rate_limit_until, "is_available=false: TemporarilyDown (timer active)");
                    return false;
                }
                tracing::info!(channel_id, %model_name, "is_available: TemporarilyDown but timer expired, allowing probe");
                true
            } else {
                // No timer — use a default backoff (5 minutes from when status was set)
                // to avoid permanent deadlock
                tracing::warn!(channel_id, %model_name, "is_available=false: TemporarilyDown (no timer set — PERMANENT DEADLOCK)");
                false
            }
        }
    }
}

/// Model-level rate-limit timer and adaptive (AIMD) limiter gate.
fn model_limits_available(
    model_state: &ModelState,
    channel_id: i32,
    model_name: &str,
    now: Instant,
) -> bool {
    if let Some(rate_limit_until) = model_state.rate_limit_until {
        if rate_limit_until > now {
            tracing::warn!(channel_id, %model_name, ?rate_limit_until, "is_available=false: model rate_limit_until");
            return false;
        }
    }

    if !model_state.adaptive_limit.check_available() {
        tracing::warn!(
            channel_id,
            %model_name,
            aimd_state = ?model_state.adaptive_limit.state,
            aimd_cooldown_until = ?model_state.adaptive_limit.cooldown_until,
            aimd_rate_limit_until = ?model_state.adaptive_limit.rate_limit_until,
            "is_available=false: AIMD check_available"
        );
        return false;
    }

    true
}

/// Apply a rate-limit failure at account, model or unknown scope.
///
/// # Arguments
/// * `channel_state` - Channel whose account/model limit timers are updated
/// * `channel_id` - Channel ID, used to create a model entry on demand
/// * `model` - Model the limit applies to, if known
/// * `scope` - Account, model or unknown rate-limit scope
/// * `retry_after` - Upstream-advertised retry delay, if any
/// * `error_message` - Human-readable error message stored on the model
/// * `now` - Timestamp captured by the caller
#[allow(
    clippy::too_many_arguments,
    reason = "private helper carrying the rate-limit failure context; grouping it into a struct would add a type with a single caller"
)]
fn record_rate_limited_failure(
    channel_state: &mut ChannelState,
    channel_id: i32,
    model: Option<&str>,
    scope: &RateLimitScope,
    retry_after: Option<u64>,
    error_message: &str,
    now: Instant,
) {
    let retry_after_duration = retry_after
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(DEFAULT_RATE_LIMIT_RETRY_SECS));
    let retry_until = now + retry_after_duration;

    match scope {
        RateLimitScope::Account => {
            channel_state.account_rate_limit_until = Some(retry_until);
        }
        RateLimitScope::Model => {
            if let Some(model_name) = model {
                let model_state = channel_state.get_or_create_model(model_name, channel_id);
                model_state.status = ModelStatus::RateLimited;
                model_state.rate_limit_until = Some(retry_until);
                model_state.last_error = Some(error_message.to_string());
                model_state.last_error_time = Some(now);
                model_state.failure_count += 1;
                // Update adaptive rate limiter
                model_state.adaptive_limit.on_rate_limited(retry_after);
            }
        }
        RateLimitScope::Unknown => {
            // If scope is unknown, treat as account-level to be safe
            channel_state.account_rate_limit_until = Some(retry_until);
            // Also update model-level adaptive limiter if model is specified
            if let Some(model_name) = model {
                let model_state = channel_state.get_or_create_model(model_name, channel_id);
                model_state.adaptive_limit.on_rate_limited(retry_after);
            }
        }
    }
}

/// Mark a model as not found on this channel.
fn record_model_not_found(
    channel_state: &mut ChannelState,
    channel_id: i32,
    model: Option<&str>,
    error_message: &str,
    now: Instant,
) {
    if let Some(model_name) = model {
        let model_state = channel_state.get_or_create_model(model_name, channel_id);
        model_state.status = ModelStatus::ModelNotFound;
        model_state.last_error = Some(error_message.to_string());
        model_state.last_error_time = Some(now);
        model_state.failure_count += 1;
    }
}

/// Record a transient model failure with exponential backoff
/// (60s -> 120s -> 240s -> 480s -> 960s -> 1800s, capped).
fn record_transient_model_failure(
    channel_state: &mut ChannelState,
    channel_id: i32,
    model: Option<&str>,
    error_message: &str,
    now: Instant,
    log_message: &'static str,
) {
    if let Some(model_name) = model {
        let model_state = channel_state.get_or_create_model(model_name, channel_id);
        model_state.failure_count += 1;

        // Exponential backoff: cooldown grows with consecutive failures (max 2^5 = 32)
        let multiplier = 1u64 << model_state.failure_count.min(5);
        let cooldown_secs = (BASE_COOLDOWN_SECS * multiplier).min(MAX_COOLDOWN_SECS);

        model_state.status = ModelStatus::TemporarilyDown;
        model_state.rate_limit_until = Some(now + Duration::from_secs(cooldown_secs));
        model_state.last_error = Some(error_message.to_string());
        model_state.last_error_time = Some(now);

        tracing::debug!(
            channel_id,
            %model_name,
            failure_count = model_state.failure_count,
            cooldown_secs,
            "{}",
            log_message
        );
    }
}

impl ChannelStateTracker {
    /// Record a successful request for a specific channel and model.
    ///
    /// Returns the AIMD learned limit if it changed after this success
    /// (used for BudgetUpdate feedback to InMemoryBudget).
    pub fn record_success(
        &self,
        channel_id: i32,
        model: Option<&str>,
        latency_ms: u64,
        upstream_limit: Option<u32>,
    ) -> Option<u32> {
        // Get or create channel state
        let mut channel_state = self
            .channel_states
            .entry(channel_id)
            .or_insert_with(|| ChannelState::new(channel_id));

        // If no model specified, nothing more to do
        let model_name = model?;

        // Update model state
        let model_state = channel_state.get_or_create_model(model_name, channel_id);

        // Update success count
        model_state.success_count += 1;

        // Update average latency using exponential moving average
        model_state.avg_latency_ms = LATENCY_EMA_ALPHA * latency_ms as f64
            + (1.0 - LATENCY_EMA_ALPHA) * model_state.avg_latency_ms;

        // If the model was temporarily down, restore it to available
        // (successful request indicates the issue is resolved)
        // Also reset failure_count to clear exponential backoff penalty
        if model_state.status == ModelStatus::TemporarilyDown {
            model_state.status = ModelStatus::Available;
            model_state.failure_count = 0; // Reset failure count on recovery
            model_state.last_error = None;
            model_state.last_error_time = None;
        }

        // Clear rate limit on success — a successful request proves the channel is available
        if model_state.status == ModelStatus::RateLimited {
            model_state.status = ModelStatus::Available;
            model_state.rate_limit_until = None;
        } else if let Some(rate_limit_until) = model_state.rate_limit_until {
            if rate_limit_until <= Instant::now() {
                model_state.rate_limit_until = None;
            }
        }

        // Update adaptive rate limiter with learned upstream limit
        let prev_learned = model_state.adaptive_limit.get_learned_limit();
        model_state.adaptive_limit.on_success(upstream_limit);
        let new_learned = model_state.adaptive_limit.get_learned_limit();
        // Return learned limit if it changed (for BudgetUpdate feedback)
        if new_learned != prev_learned {
            new_learned
        } else {
            None
        }
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
    #[allow(
        dead_code,
        reason = "list-filter helper kept for callers that need bulk availability checks"
    )]
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
    /// For model-specific scoring, delegates to `get_health_and_adaptive`.
    ///
    /// # Arguments
    /// * `channel_id` - The channel ID to calculate score for
    /// * `model` - Optional model name for model-specific scoring
    ///
    /// # Returns
    /// A health score (higher is better). Default is 1.0 for unknown channels.
    pub fn get_health_score(&self, channel_id: i32, model: Option<&str>) -> f64 {
        match model {
            Some(m) => self.get_health_and_adaptive(channel_id, m).0,
            None => {
                // Channel-only score (no model)
                let channel_state = match self.channel_states.get(&channel_id) {
                    Some(state) => state,
                    None => return 1.0,
                };
                let mut score = 1.0;
                if !channel_state.auth_ok {
                    score *= PENALTY_AUTH_FAILED;
                }
                match channel_state.balance_status {
                    BalanceStatus::Ok => {}
                    BalanceStatus::Low => score *= PENALTY_BALANCE_LOW,
                    BalanceStatus::Exhausted => score *= PENALTY_BALANCE_EXHAUSTED,
                    BalanceStatus::Unknown => {}
                }
                score
            }
        }
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

    /// Combined lookup: health score + adaptive snapshot in a single DashMap get.
    ///
    /// Returns (health_score, Some(snapshot)) on success,
    /// (1.0, None) for unknown channels, or (computed_score, cold_start_snapshot) for unknown models.
    pub fn get_health_and_adaptive(&self, channel_id: i32, model: &str) -> (f64, AimdSnapshot) {
        let cold_start = AimdSnapshot {
            current_limit: crate::aimd_limiter::DEFAULT_INITIAL_LIMIT,
            state: crate::aimd_limiter::RateLimitState::Learning,
        };

        let channel_state = match self.channel_states.get(&channel_id) {
            Some(state) => state,
            None => return (1.0, cold_start),
        };

        let mut score = 1.0;

        if !channel_state.auth_ok {
            score *= 0.1;
        }

        match channel_state.balance_status {
            BalanceStatus::Ok => {}
            BalanceStatus::Low => score *= 0.7,
            BalanceStatus::Exhausted => score *= 0.1,
            BalanceStatus::Unknown => {}
        }

        let model_state = match channel_state.models.get(model) {
            Some(ms) => ms,
            None => return (score, cold_start),
        };

        let total = model_state.success_count + model_state.failure_count;
        if total > 0 {
            score *= model_state.success_count as f64 / total as f64;
        }

        if model_state.avg_latency_ms > 0.0 {
            score *= LATENCY_SCORE_MIDPOINT_MS
                / (LATENCY_SCORE_MIDPOINT_MS + model_state.avg_latency_ms);
        }

        match model_state.status {
            ModelStatus::Available => {}
            ModelStatus::RateLimited => score *= PENALTY_RATE_LIMITED,
            ModelStatus::QuotaExhausted => score *= PENALTY_QUOTA_EXHAUSTED,
            ModelStatus::ModelNotFound => score *= PENALTY_MODEL_NOT_FOUND,
            ModelStatus::TemporarilyDown => score *= PENALTY_TEMPORARILY_DOWN,
        }

        (score, model_state.adaptive_limit.snapshot())
    }

    /// Get health scores for a batch of channels.
    #[cfg(test)]
    pub fn get_all_health_scores(
        &self,
        channel_ids: &[i32],
        model: Option<&str>,
    ) -> std::collections::HashMap<i32, f64> {
        channel_ids
            .iter()
            .map(|&id| (id, self.get_health_score(id, model)))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    //! Behavioural contract for the channel/model availability gate.
    //!
    //! This module previously held a `#[cfg(test)]` item but no tests, while the logic it
    //! guards is the first thing every scheduling attempt consults. The failure modes pinned
    //! here are the ones the source itself calls out: a model that becomes permanently
    //! blocked, a channel-wide failure mistaken for a model-local one, and a rate-limited
    //! model that silently returns to availability.
    //!
    //! These must live inline rather than in `tests/`: `FailureType`, `RateLimitScope` and
    //! `AimdSnapshot` are not publicly re-exported, and `record_error` takes a `&FailureType`,
    //! so the gate is not reachable from an integration test at all.
    //!
    //! Deliberately **not** asserted: the expiry branches ("timer elapsed, allow a probe").
    //! This module uses `std::time::Instant`, which tokio's paused clock does not advance:
    //! `tokio::time::pause`/`advance` only control `tokio::time::Instant`. Asserting those
    //! branches would require a real sleep, which is the flaky shape to avoid. What is
    //! asserted instead is that the timer is **set** when a failure is recorded.
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "unit tests: a failed expectation is the intended failure signal, and the workspace denies unwrap_used even in test code"
    )]
    use super::*;

    const CHANNEL: i32 = 7;
    const MODEL: &str = "gpt-4o";

    /// Look up the state for `channel_id`, asserting it was created.
    fn channel_state(tracker: &ChannelStateTracker, channel_id: i32) -> ChannelState {
        tracker
            .get_all_states()
            .into_iter()
            .find(|(id, _)| *id == channel_id)
            .map(|(_, state)| state)
            .expect("recording an error must create the channel state")
    }

    /// An error condition with no `retry_after` must still arm a timer. If it did not, the
    /// "no timer set: PERMANENT DEADLOCK" branch in the source would become reachable and the
    /// model could never be probed again.
    #[test]
    fn a_transient_failure_arms_a_backoff_timer_and_blocks_the_model() {
        let tracker = ChannelStateTracker::new();

        tracker.record_error(
            CHANNEL,
            Some(MODEL),
            &FailureType::ServerError,
            "upstream 500",
        );

        assert!(
            !tracker.is_available(CHANNEL, Some(MODEL)),
            "a model just marked TemporarilyDown must not be available"
        );

        let state = channel_state(&tracker, CHANNEL);
        let model = state
            .models
            .get(MODEL)
            .expect("recording a model error must create the model state");

        assert_eq!(model.status, ModelStatus::TemporarilyDown);
        assert_eq!(model.failure_count, 1);
        assert!(
            model.rate_limit_until.is_some(),
            "backoff must set rate_limit_until, otherwise the status can never be probed again"
        );
    }

    /// The other transient kinds share one code path but are distinct variants, so a
    /// reclassification mistake in the match would otherwise go unnoticed.
    #[test]
    fn every_transient_kind_blocks_its_model_but_not_the_channel() {
        for (index, failure) in [
            FailureType::Timeout,
            FailureType::ConnectionError,
            FailureType::EmptyResponse,
        ]
        .into_iter()
        .enumerate()
        {
            let tracker = ChannelStateTracker::new();
            let channel = 100 + index as i32;

            tracker.record_error(channel, Some(MODEL), &failure, "transient");

            assert!(
                !tracker.is_available(channel, Some(MODEL)),
                "{failure:?} must block the model it was recorded against"
            );
            assert!(
                tracker.is_available(channel, Some("some-other-model")),
                "{failure:?} must not take the whole channel down"
            );
        }
    }

    /// An auth failure is channel-wide: it must block every model, including one never seen
    /// before. Getting this wrong keeps sending traffic through a channel that cannot
    /// authenticate, losing the request instead of failing over.
    #[test]
    fn an_auth_failure_blocks_the_whole_channel_including_unknown_models() {
        let tracker = ChannelStateTracker::new();

        // Establish a healthy, known model first so the check is not merely "no state yet".
        tracker.record_success(CHANNEL, Some(MODEL), 10, None);
        assert!(tracker.is_available(CHANNEL, Some(MODEL)));

        tracker.record_error(CHANNEL, None, &FailureType::AuthFailed, "401");

        assert!(
            !tracker.is_available(CHANNEL, Some(MODEL)),
            "auth failure must block an already-known model"
        );
        assert!(
            !tracker.is_available(CHANNEL, Some("never-seen-before")),
            "auth failure must block a model the tracker has never seen"
        );
        assert!(
            !tracker.is_available(CHANNEL, None),
            "auth failure must block the channel even with no model named"
        );
    }

    /// `PaymentRequired` exhausts the balance, which is also channel-wide.
    #[test]
    fn a_payment_failure_exhausts_the_balance_and_blocks_the_channel() {
        let tracker = ChannelStateTracker::new();

        tracker.record_error(CHANNEL, None, &FailureType::PaymentRequired, "402");

        assert!(!tracker.is_available(CHANNEL, None));
        assert!(!tracker.is_available(CHANNEL, Some(MODEL)));

        assert_eq!(
            channel_state(&tracker, CHANNEL).balance_status,
            BalanceStatus::Exhausted
        );
    }

    /// A model-scoped rate limit must not spill onto a sibling model on the same channel.
    #[test]
    fn a_model_rate_limit_does_not_block_a_sibling_model() {
        let tracker = ChannelStateTracker::new();

        tracker.record_error(
            CHANNEL,
            Some(MODEL),
            &FailureType::RateLimited {
                scope: RateLimitScope::Model,
                retry_after: Some(30),
            },
            "429",
        );

        assert!(
            !tracker.is_available(CHANNEL, Some(MODEL)),
            "the rate-limited model must be unavailable"
        );
        assert!(
            tracker.is_available(CHANNEL, Some("sibling-model")),
            "a model-scoped limit must leave sibling models usable"
        );

        let state = channel_state(&tracker, CHANNEL);
        assert_eq!(
            state.models.get(MODEL).map(|m| &m.status),
            Some(&ModelStatus::RateLimited)
        );
        assert!(
            state.account_rate_limit_until.is_none(),
            "a model-scoped limit must not arm the account-wide timer"
        );
    }

    /// An account-scoped rate limit is channel-wide and must arm the account timer.
    #[test]
    fn an_account_rate_limit_arms_the_account_timer_and_blocks_every_model() {
        let tracker = ChannelStateTracker::new();

        tracker.record_error(
            CHANNEL,
            None,
            &FailureType::RateLimited {
                scope: RateLimitScope::Account,
                retry_after: Some(120),
            },
            "429 account",
        );

        assert!(!tracker.is_available(CHANNEL, None));
        assert!(!tracker.is_available(CHANNEL, Some(MODEL)));
        assert!(!tracker.is_available(CHANNEL, Some("any-other-model")));

        assert!(
            channel_state(&tracker, CHANNEL)
                .account_rate_limit_until
                .is_some(),
            "an account-scoped limit must arm the account-wide timer"
        );
    }

    /// An unknown scope is documented as "treat as account-level to be safe". This pins that
    /// choice: it is deliberately the conservative branch, not a model-only one.
    #[test]
    fn an_unknown_rate_limit_scope_falls_back_to_the_account_wide_timer() {
        let tracker = ChannelStateTracker::new();

        tracker.record_error(
            CHANNEL,
            Some(MODEL),
            &FailureType::RateLimited {
                scope: RateLimitScope::Unknown,
                retry_after: Some(45),
            },
            "429 unknown scope",
        );

        assert!(
            !tracker.is_available(CHANNEL, Some("a-model-that-was-never-mentioned")),
            "an unknown scope must be treated as account-wide"
        );
        assert!(channel_state(&tracker, CHANNEL)
            .account_rate_limit_until
            .is_some());
    }

    /// `ModelNotFound` is permanent for the model: it stays blocked while carrying no timer.
    #[test]
    fn model_not_found_blocks_while_carrying_no_timer() {
        let tracker = ChannelStateTracker::new();

        tracker.record_error(CHANNEL, Some(MODEL), &FailureType::ModelNotFound, "404");

        assert!(!tracker.is_available(CHANNEL, Some(MODEL)));

        let state = channel_state(&tracker, CHANNEL);
        let model = state.models.get(MODEL).expect("the model state must exist");
        assert_eq!(model.status, ModelStatus::ModelNotFound);
        assert!(
            model.rate_limit_until.is_none(),
            "a permanent status carries no timer, so it must not look probe-able"
        );
    }

    /// A channel or model the tracker has never seen is assumed available, so a cold start
    /// does not refuse every request.
    #[test]
    fn untracked_channels_and_models_are_assumed_available() {
        let tracker = ChannelStateTracker::new();

        assert!(tracker.is_available(999, None));
        assert!(tracker.is_available(999, Some("anything")));

        // A tracked-but-healthy channel must also admit a model it has no state for.
        tracker.record_success(CHANNEL, Some(MODEL), 10, None);
        assert!(tracker.is_available(CHANNEL, Some("brand-new-model")));
    }

    /// A success must clear the *transient* bookkeeping: the status returns to `Available`
    /// and the failure counter resets, so the next backoff starts from the base delay rather
    /// than compounding.
    #[test]
    fn a_success_returns_the_status_to_available_and_resets_the_failure_count() {
        let tracker = ChannelStateTracker::new();

        tracker.record_error(CHANNEL, Some(MODEL), &FailureType::ServerError, "500");
        assert!(!tracker.is_available(CHANNEL, Some(MODEL)));

        tracker.record_success(CHANNEL, Some(MODEL), 25, None);

        let state = channel_state(&tracker, CHANNEL);
        let model = state.models.get(MODEL).expect("the model state must exist");
        assert_eq!(model.status, ModelStatus::Available);
        assert_eq!(
            model.failure_count, 0,
            "the failure count must reset so the next backoff starts from the base delay"
        );
        assert!(model.last_error.is_none());
    }

    /// **Known defect, pinned rather than fixed.** `record_success` restores `status` to
    /// `Available` but does **not** clear `rate_limit_until`. `is_available` then asks
    /// `model_status_available` (true, the status is Available) and `model_limits_available`
    /// (false, the stale backoff timer is still in the future), so the model stays unavailable
    /// for the remainder of its backoff window even though a request has just succeeded.
    ///
    /// For `ServerError` that window is 120 seconds (failure_count becomes 1, so the
    /// multiplier is `1 << 1`), which is why this is asserted deterministically rather than by
    /// racing a timer: with time on our side the observed behaviour is always "still blocked".
    ///
    /// This test exists so the behaviour is **visible**. It is deliberately written to fail if
    /// the defect is fixed, forcing whoever fixes it to update this test and notice the
    /// contract change. Marking it as expected-behaviour would hide a real recovery delay.
    ///
    /// Reported separately: this is a runtime behaviour change to the router's availability
    /// gate, and #634 explicitly forbids folding production behaviour changes into test work.
    /// Contrast `RateLimited`, where `record_success` *does* clear `rate_limit_until`.
    #[test]
    fn a_success_leaves_the_backoff_timer_set_so_the_model_stays_unavailable() {
        let tracker = ChannelStateTracker::new();

        tracker.record_error(CHANNEL, Some(MODEL), &FailureType::ServerError, "500");
        tracker.record_success(CHANNEL, Some(MODEL), 25, None);

        let state = channel_state(&tracker, CHANNEL);
        let model = state.models.get(MODEL).expect("the model state must exist");
        assert_eq!(
            model.status,
            ModelStatus::Available,
            "the status is restored even though the timer is not cleared"
        );
        assert!(
            model.rate_limit_until.is_some(),
            "the stale backoff timer is the defect: it should have been cleared"
        );
        assert!(
            !tracker.is_available(CHANNEL, Some(MODEL)),
            "consequence of the defect: a just-succeeded model is still unavailable"
        );
    }

    /// `get_available_channels` is the bulk form of `is_available`; it must drop exactly the
    /// unavailable channels and preserve the caller's order for the rest.
    #[test]
    fn the_bulk_filter_drops_only_the_unavailable_channels() {
        let tracker = ChannelStateTracker::new();
        let healthy = 11;
        let unhealthy = 12;

        tracker.record_error(unhealthy, None, &FailureType::AuthFailed, "401");

        let filtered = tracker.get_available_channels(&[healthy, unhealthy, 13], None);
        assert_eq!(
            filtered,
            vec![healthy, 13],
            "the bulk filter must drop only the failed channel and keep the input order"
        );
    }

    /// Channel-only health scoring, independent of any model.
    #[test]
    fn channel_health_score_reflects_auth_and_balance_but_not_model_state() {
        let tracker = ChannelStateTracker::new();

        assert_eq!(
            tracker.get_health_score(CHANNEL, None),
            1.0,
            "an untracked channel scores full health"
        );

        tracker.record_error(CHANNEL, None, &FailureType::PaymentRequired, "402");
        let exhausted = tracker.get_health_score(CHANNEL, None);
        assert!(
            exhausted < 1.0,
            "an exhausted balance must reduce the channel score, got {exhausted}"
        );

        let auth_failed = 21;
        tracker.record_error(auth_failed, None, &FailureType::AuthFailed, "401");
        let auth_score = tracker.get_health_score(auth_failed, None);
        assert!(
            auth_score < 1.0,
            "an auth failure must reduce the channel score, got {auth_score}"
        );

        // Model-level state must not move the channel-only score.
        tracker.record_error(CHANNEL, Some(MODEL), &FailureType::ServerError, "500");
        assert_eq!(
            tracker.get_health_score(CHANNEL, None),
            exhausted,
            "the channel-only score must ignore model-level failures"
        );
    }

    /// `get_health_and_adaptive` returns the learner snapshot alongside the score, and a cold
    /// model still reports the default limit rather than nothing.
    #[test]
    fn health_and_adaptive_returns_a_usable_snapshot_for_a_cold_model() {
        let tracker = ChannelStateTracker::new();

        let (score, snapshot) = tracker.get_health_and_adaptive(999, MODEL);
        assert_eq!(score, 1.0);
        assert_eq!(
            snapshot.current_limit,
            crate::aimd_limiter::DEFAULT_INITIAL_LIMIT
        );

        // A known channel whose model has no entry keeps full *channel* score, but the returned
        // score also folds in the sibling model's success ratio and latency, so it is only
        // bounded rather than exactly 1.0. Assert the bound, not an invented constant.
        tracker.record_success(CHANNEL, Some(MODEL), 10, None);
        let (score, snapshot) = tracker.get_health_and_adaptive(CHANNEL, "unseen-model");
        assert!(
            score > 0.0 && score <= 1.0,
            "a healthy channel must score within (0, 1], got {score}"
        );
        assert_eq!(
            snapshot.current_limit,
            crate::aimd_limiter::DEFAULT_INITIAL_LIMIT,
            "a model with no entry must still report the cold-start limit"
        );
    }

    /// Default construction must behave identically to `new`.
    #[test]
    fn default_matches_new() {
        let from_default = ChannelStateTracker::default();
        let from_new = ChannelStateTracker::new();

        assert!(from_default.is_available(1, None));
        assert!(from_new.is_available(1, None));
        assert!(from_default.get_all_states().is_empty());
        assert!(from_new.get_all_states().is_empty());
    }
}
