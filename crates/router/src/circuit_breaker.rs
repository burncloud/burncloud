use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Represents the scope of a rate limit.
///
/// Used to distinguish between account-level and model-level rate limits.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RateLimitScope {
    /// Rate limit applies at the account level (affects all models)
    Account,
    /// Rate limit applies at the model level (specific model only)
    Model,
    /// Rate limit scope is unknown
    Unknown,
}

impl Default for RateLimitScope {
    fn default() -> Self {
        Self::Unknown
    }
}

/// Represents the type of failure encountered when communicating with an upstream.
///
/// Different failure types may trigger different behaviors in the circuit breaker
/// and channel state tracker.
#[derive(Debug, Clone)]
pub enum FailureType {
    /// Authentication failed (401 Unauthorized)
    AuthFailed,
    /// Payment required / quota exhausted (402 Payment Required)
    PaymentRequired,
    /// Rate limited by upstream (429 Too Many Requests)
    RateLimited {
        /// The scope of the rate limit
        scope: RateLimitScope,
        /// When the rate limit will reset (seconds from now)
        retry_after: Option<u64>,
    },
    /// Model not found on upstream (404 Not Found)
    ModelNotFound,
    /// Server error from upstream (5xx)
    ServerError,
    /// Request timed out
    Timeout,
}

#[derive(Debug)]
struct UpstreamState {
    failure_count: AtomicU32,
    last_failure_time: Option<Instant>,
    /// The type of the last failure encountered
    failure_type: Option<FailureType>,
    /// When the rate limit will expire (if rate limited)
    rate_limit_until: Option<Instant>,
}

impl Default for UpstreamState {
    fn default() -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            last_failure_time: None,
            failure_type: None,
            rate_limit_until: None,
        }
    }
}

pub struct CircuitBreaker {
    states: DashMap<String, UpstreamState>,
    failure_threshold: u32,
    cooldown_duration: Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, cooldown_seconds: u64) -> Self {
        Self {
            states: DashMap::new(),
            failure_threshold,
            cooldown_duration: Duration::from_secs(cooldown_seconds),
        }
    }

    /// Checks if a request is allowed to proceed to the given upstream.
    pub fn allow_request(&self, upstream_id: &str) -> bool {
        let entry = self.states.entry(upstream_id.to_string()).or_default();

        // Check if currently rate limited
        if let Some(rate_limit_until) = entry.rate_limit_until {
            if rate_limit_until > Instant::now() {
                return false; // Still rate limited
            }
        }

        let current_failures = entry.failure_count.load(Ordering::Relaxed);

        if current_failures < self.failure_threshold {
            return true; // Closed state
        }

        // Circuit is Open, check if cooldown has passed
        if let Some(last_failure) = entry.last_failure_time {
            if last_failure.elapsed() >= self.cooldown_duration {
                // Transition to Half-Open (allow one probe)
                // We don't change state explicitly here, but we allow *this* request.
                // In a real strict implementation, we might want to ensure only ONE request passes,
                // but for simplicity in this stateless router, allowing traffic after cooldown is acceptable.
                // If it fails again, the time updates and we wait again.
                return true;
            }
        }

        false // Still Open
    }

    /// Records a successful request.
    pub fn record_success(&self, upstream_id: &str) {
        if let Some(mut entry) = self.states.get_mut(upstream_id) {
            // Reset failure count on success
            entry.failure_count.store(0, Ordering::Relaxed);
            entry.last_failure_time = None;
            entry.failure_type = None;
            entry.rate_limit_until = None;
        }
    }

    /// Records a failed request with a specific failure type.
    ///
    /// Different failure types may have different impacts:
    /// - AuthFailed/PaymentRequired: Immediate circuit trip
    /// - RateLimited: Records retry_after time
    /// - Other failures: Increment failure count
    pub fn record_failure_with_type(&self, upstream_id: &str, failure_type: FailureType) {
        let mut entry = self.states.entry(upstream_id.to_string()).or_default();

        // Store the failure type
        entry.failure_type = Some(failure_type.clone());
        entry.last_failure_time = Some(Instant::now());

        match failure_type {
            FailureType::AuthFailed | FailureType::PaymentRequired => {
                // Auth and payment failures immediately trip the circuit
                entry
                    .failure_count
                    .store(self.failure_threshold, Ordering::Relaxed);
                println!(
                    "Circuit Breaker: Upstream {} immediately tripped due to {:?}",
                    upstream_id, failure_type
                );
            }
            FailureType::RateLimited {
                scope: _,
                retry_after,
            } => {
                // Set rate limit expiry time
                let duration = retry_after
                    .map(|s| Duration::from_secs(s))
                    .unwrap_or(Duration::from_secs(60));
                entry.rate_limit_until = Some(Instant::now() + duration);

                // Also increment failure count for rate limits
                let new_count = entry.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if new_count >= self.failure_threshold {
                    println!(
                        "Circuit Breaker: Upstream {} tripped due to rate limit (Failures: {})",
                        upstream_id, new_count
                    );
                }
            }
            _ => {
                // Other failures just increment the count
                let new_count = entry.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if new_count >= self.failure_threshold {
                    println!(
                        "Circuit Breaker: Upstream {} tripped! (Failures: {})",
                        upstream_id, new_count
                    );
                }
            }
        }
    }

    /// Records a failed request (legacy method for backward compatibility).
    pub fn record_failure(&self, upstream_id: &str) {
        let mut entry = self.states.entry(upstream_id.to_string()).or_default();
        let new_count = entry.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
        entry.last_failure_time = Some(Instant::now());

        if new_count >= self.failure_threshold {
            println!(
                "Circuit Breaker: Upstream {} tripped! (Failures: {})",
                upstream_id, new_count
            );
        }
    }

    /// Get the last failure type for an upstream.
    pub fn get_failure_type(&self, upstream_id: &str) -> Option<FailureType> {
        self.states
            .get(upstream_id)
            .and_then(|entry| entry.failure_type.clone())
    }

    /// Get current health status map for monitoring
    pub fn get_status_map(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        for r in self.states.iter() {
            let count = r.value().failure_count.load(Ordering::Relaxed);
            let status = if count >= self.failure_threshold {
                // Check if in cooldown
                if let Some(last) = r.value().last_failure_time {
                    if last.elapsed() < self.cooldown_duration {
                        format!(
                            "Open (Tripped, {}s left)",
                            (self.cooldown_duration - last.elapsed()).as_secs()
                        )
                    } else {
                        "Half-Open (Probing)".to_string()
                    }
                } else {
                    "Open".to_string()
                }
            } else {
                "Closed (Healthy)".to_string()
            };
            map.insert(r.key().clone(), status);
        }
        map
    }
}
