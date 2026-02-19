//! Adaptive Rate Limit Module
//!
//! This module provides functionality for dynamically learning and adapting to
//! upstream API rate limits. It implements a state machine that:
//! - Learns the actual rate limits from response headers
//! - Adjusts request rates based on success/failure patterns
//! - Manages cooldown periods when rate limited

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// Represents the state of the adaptive rate limit state machine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RateLimitState {
    /// Initial state - learning the actual rate limits
    Learning,
    /// Stable state - operating within known limits
    Stable,
    /// Cooldown state - recovering from rate limit errors
    Cooldown,
}

impl Default for RateLimitState {
    fn default() -> Self {
        Self::Learning
    }
}

/// Configuration for the adaptive rate limiter.
#[derive(Debug, Clone)]
pub struct AdaptiveLimitConfig {
    /// Duration in requests before transitioning from Learning to Stable
    pub learning_duration: u32,
    /// Initial request limit (conservative starting point)
    pub initial_limit: u32,
    /// Step size for adjusting limits up or down
    pub adjustment_step: u32,
    /// Number of consecutive successes before increasing limit
    pub success_threshold: u32,
    /// Number of consecutive failures before entering cooldown
    pub failure_threshold: u32,
    /// Duration to stay in cooldown state (seconds)
    pub cooldown_duration: Duration,
    /// Ratio to reduce limit when entering cooldown (e.g., 0.5 = 50%)
    pub recovery_ratio: f64,
    /// Maximum allowed request limit
    pub max_limit: u32,
}

impl Default for AdaptiveLimitConfig {
    fn default() -> Self {
        Self {
            learning_duration: 10, // Learn over 10 requests
            initial_limit: 10,     // Start conservatively
            adjustment_step: 5,    // Adjust by 5 requests at a time
            success_threshold: 5,  // 5 successes to increase
            failure_threshold: 2,  // 2 failures to cooldown
            cooldown_duration: Duration::from_secs(30),
            recovery_ratio: 0.5, // Reduce to 50% after cooldown
            max_limit: 1000,     // Cap at 1000 requests
        }
    }
}

/// Adaptive rate limiter that learns and adjusts to upstream limits.
///
/// This struct maintains state for a single upstream endpoint or model,
/// tracking learned limits and adjusting based on success/failure patterns.
#[derive(Debug)]
pub struct AdaptiveRateLimit {
    /// The rate limit learned from upstream response headers
    pub learned_limit: Option<u32>,
    /// The current effective limit being used
    pub current_limit: u32,
    /// Current state in the state machine
    pub state: RateLimitState,
    /// Number of consecutive successful requests
    pub success_streak: u32,
    /// Number of consecutive failed requests (rate limit errors)
    pub failure_streak: u32,
    /// When the cooldown period will end (if in Cooldown state)
    pub cooldown_until: Option<Instant>,
    /// When the rate limit will expire (from 429 response)
    pub rate_limit_until: Option<Instant>,
    /// When the limit was last adjusted
    pub last_adjusted_at: Option<Instant>,
    /// Configuration
    config: AdaptiveLimitConfig,
    /// Number of requests processed (used for learning duration)
    request_count: u32,
}

impl AdaptiveRateLimit {
    /// Create a new AdaptiveRateLimit with the given configuration.
    pub fn new(config: AdaptiveLimitConfig) -> Self {
        Self {
            learned_limit: None,
            current_limit: config.initial_limit,
            state: RateLimitState::Learning,
            success_streak: 0,
            failure_streak: 0,
            cooldown_until: None,
            rate_limit_until: None,
            last_adjusted_at: None,
            config,
            request_count: 0,
        }
    }

    /// Create a new AdaptiveRateLimit with default configuration.
    pub fn with_defaults() -> Self {
        Self::new(AdaptiveLimitConfig::default())
    }

    /// Called when a request succeeds.
    ///
    /// This method:
    /// - Learns rate limits from response headers
    /// - Updates success streak
    /// - Transitions from Learning to Stable
    /// - Attempts to increase limit in Learning state
    ///
    /// # Arguments
    /// * `upstream_limit` - Optional rate limit learned from response headers
    pub fn on_success(&mut self, upstream_limit: Option<u32>) {
        let now = Instant::now();

        // Learn the upstream limit if provided
        if let Some(limit) = upstream_limit {
            if limit > 0 && limit <= self.config.max_limit {
                self.learned_limit = Some(limit);
                // If we learned a limit and it's higher than current, use it
                if limit > self.current_limit {
                    self.current_limit = limit.min(self.config.max_limit);
                    self.last_adjusted_at = Some(now);
                }
            }
        }

        // Increment request count and success streak
        self.request_count += 1;
        self.success_streak += 1;
        self.failure_streak = 0;

        // Handle state transitions and adjustments
        match self.state {
            RateLimitState::Learning => {
                // Check if we should transition to Stable
                if self.request_count >= self.config.learning_duration {
                    self.state = RateLimitState::Stable;
                    println!(
                        "AdaptiveLimit: Transitioned to Stable after {} requests",
                        self.request_count
                    );
                }

                // In learning state, try to increase limit after success threshold
                if self.success_streak >= self.config.success_threshold {
                    self.try_increase_limit(now);
                }
            }
            RateLimitState::Stable => {
                // In stable state, increase limit if we have success streak
                if self.success_streak >= self.config.success_threshold {
                    self.try_increase_limit(now);
                }
            }
            RateLimitState::Cooldown => {
                // Check if cooldown period is over
                if let Some(cooldown_until) = self.cooldown_until {
                    if now >= cooldown_until {
                        self.recover_from_cooldown();
                    }
                }
            }
        }

        // Clear rate limit expiry if it has passed
        if let Some(rate_limit_until) = self.rate_limit_until {
            if now >= rate_limit_until {
                self.rate_limit_until = None;
            }
        }
    }

    /// Called when a rate limit error (429) is encountered.
    ///
    /// This method:
    /// - Updates failure streak
    /// - Reduces current limit to 80%
    /// - Checks if we should enter Cooldown state
    /// - Records when the rate limit will expire
    ///
    /// # Arguments
    /// * `retry_after` - Optional seconds until retry is allowed
    pub fn on_rate_limited(&mut self, retry_after: Option<u64>) {
        let now = Instant::now();

        // Update failure streak and reset success streak
        self.failure_streak += 1;
        self.success_streak = 0;

        // Record rate limit expiry time
        if let Some(seconds) = retry_after {
            self.rate_limit_until = Some(now + Duration::from_secs(seconds));
        }

        // Reduce current limit by 20% (keep 80%)
        let new_limit = (self.current_limit as f64 * 0.8).ceil() as u32;
        self.current_limit = new_limit.max(1); // Ensure at least 1
        self.last_adjusted_at = Some(now);

        println!(
            "AdaptiveLimit: Reduced limit to {} after rate limit error",
            self.current_limit
        );

        // Check if we should enter cooldown
        if self.failure_streak >= self.config.failure_threshold {
            self.enter_cooldown(now);
        }
    }

    /// Check if requests are currently allowed.
    ///
    /// Returns false if:
    /// - In Cooldown state and cooldown period hasn't expired
    /// - Rate limit expiry time hasn't passed
    pub fn check_available(&self) -> bool {
        let now = Instant::now();

        // Check if in cooldown
        if self.state == RateLimitState::Cooldown {
            if let Some(cooldown_until) = self.cooldown_until {
                if now < cooldown_until {
                    return false;
                }
            }
        }

        // Check if rate limited
        if let Some(rate_limit_until) = self.rate_limit_until {
            if now < rate_limit_until {
                return false;
            }
        }

        true
    }

    /// Get the current effective limit.
    pub fn get_current_limit(&self) -> u32 {
        self.current_limit
    }

    /// Get the learned upstream limit (if known).
    pub fn get_learned_limit(&self) -> Option<u32> {
        self.learned_limit
    }

    /// Get the current state.
    pub fn get_state(&self) -> &RateLimitState {
        &self.state
    }

    /// Enter cooldown state.
    fn enter_cooldown(&mut self, now: Instant) {
        self.state = RateLimitState::Cooldown;
        self.cooldown_until = Some(now + self.config.cooldown_duration);
        println!(
            "AdaptiveLimit: Entering Cooldown for {}s",
            self.config.cooldown_duration.as_secs()
        );
    }

    /// Recover from cooldown state.
    ///
    /// Sets state to Learning and reduces limit by recovery_ratio.
    fn recover_from_cooldown(&mut self) {
        let now = Instant::now();

        // Reduce limit by recovery ratio
        let new_limit = (self.current_limit as f64 * self.config.recovery_ratio).ceil() as u32;
        self.current_limit = new_limit.max(1);

        // Reset state
        self.state = RateLimitState::Learning;
        self.cooldown_until = None;
        self.failure_streak = 0;
        self.success_streak = 0;
        self.last_adjusted_at = Some(now);

        println!(
            "AdaptiveLimit: Recovered from Cooldown, new limit: {}",
            self.current_limit
        );
    }

    /// Try to increase the current limit.
    fn try_increase_limit(&mut self, now: Instant) {
        // Don't exceed learned limit if known
        let max_allowed = self.learned_limit.unwrap_or(self.config.max_limit);

        // Don't exceed configured max
        let max_allowed = max_allowed.min(self.config.max_limit);

        if self.current_limit < max_allowed {
            let new_limit = (self.current_limit + self.config.adjustment_step).min(max_allowed);
            self.current_limit = new_limit;
            self.last_adjusted_at = Some(now);
            self.success_streak = 0; // Reset streak after adjustment

            println!("AdaptiveLimit: Increased limit to {}", self.current_limit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let limiter = AdaptiveRateLimit::with_defaults();
        assert_eq!(limiter.state, RateLimitState::Learning);
        assert_eq!(limiter.current_limit, 10); // default initial_limit
        assert!(limiter.check_available());
    }

    #[test]
    fn test_learning_to_stable_transition() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            learning_duration: 5,
            ..Default::default()
        });

        // Process 5 requests
        for _ in 0..5 {
            limiter.on_success(None);
        }

        assert_eq!(limiter.state, RateLimitState::Stable);
    }

    #[test]
    fn test_rate_limited_reduces_limit() {
        let mut limiter = AdaptiveRateLimit::with_defaults();
        let initial_limit = limiter.current_limit;

        limiter.on_rate_limited(None);

        assert!(limiter.current_limit < initial_limit);
        assert_eq!(
            limiter.current_limit,
            (initial_limit as f64 * 0.8).ceil() as u32
        );
    }

    #[test]
    fn test_cooldown_state() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_secs(1),
            ..Default::default()
        });

        // Trigger cooldown
        limiter.on_rate_limited(None);

        assert_eq!(limiter.state, RateLimitState::Cooldown);
        assert!(!limiter.check_available());
    }

    #[test]
    fn test_recovery_from_cooldown() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            failure_threshold: 1,
            cooldown_duration: Duration::from_millis(10),
            ..Default::default()
        });

        let initial_limit = limiter.current_limit;

        // Trigger cooldown
        limiter.on_rate_limited(None);
        assert_eq!(limiter.state, RateLimitState::Cooldown);

        // Wait for cooldown
        std::thread::sleep(Duration::from_millis(20));

        // Check availability should trigger recovery
        limiter.check_available();
        // Actually trigger recovery via on_success
        limiter.on_success(None);

        assert_eq!(limiter.state, RateLimitState::Learning);
        // Limit should be reduced by recovery_ratio (50%)
        assert!(limiter.current_limit <= (initial_limit as f64 * 0.8 * 0.5).ceil() as u32);
    }

    #[test]
    fn test_learn_upstream_limit() {
        let mut limiter = AdaptiveRateLimit::with_defaults();

        limiter.on_success(Some(100));

        assert_eq!(limiter.learned_limit, Some(100));
        assert_eq!(limiter.current_limit, 100);
    }

    #[test]
    fn test_success_streak_increases_limit() {
        let mut limiter = AdaptiveRateLimit::new(AdaptiveLimitConfig {
            success_threshold: 3,
            adjustment_step: 5,
            learning_duration: 100, // Stay in Learning
            ..Default::default()
        });

        let initial_limit = limiter.current_limit;

        // 3 successes should increase limit
        for _ in 0..3 {
            limiter.on_success(None);
        }

        assert_eq!(limiter.current_limit, initial_limit + 5);
    }

    #[test]
    fn test_rate_limit_expiry() {
        let mut limiter = AdaptiveRateLimit::with_defaults();

        // Set rate limit for 10ms
        limiter.on_rate_limited(Some(0)); // 0 seconds

        // Should be available immediately
        assert!(limiter.check_available());
    }
}
