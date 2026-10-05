use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Represents the scope of a rate limit.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum RateLimitScope {
    Account,
    Model,
    #[default]
    Unknown,
}

/// Represents the type of failure encountered when communicating with an upstream.
#[derive(Debug, Clone)]
pub enum FailureType {
    AuthFailed,
    PaymentRequired,
    RateLimited {
        scope: RateLimitScope,
        retry_after: Option<u64>,
    },
    ModelNotFound,
    ServerError,
    Timeout,
    ConnectionError,
    EmptyResponse,
}

#[derive(Debug)]
struct UpstreamState {
    failure_count: AtomicU32,
    last_failure_time: Option<Instant>,
    failure_type: Option<FailureType>,
    rate_limit_until: Option<Instant>,
    /// Set only while the single Half-Open probe slot is leased.
    ///
    /// A timestamp rather than a bool gives the breaker a bounded recovery path if
    /// the caller that received the probe slot exits without calling either
    /// `record_success` or `record_failure_with_type`.
    half_open_probe_started_at: Option<Instant>,
}

impl Default for UpstreamState {
    fn default() -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            last_failure_time: None,
            failure_type: None,
            rate_limit_until: None,
            half_open_probe_started_at: None,
        }
    }
}

const DEFAULT_RATE_LIMIT_RETRY_SECS: u64 = 60;
/// Maximum time a caller may hold the Half-Open probe slot without reporting a result.
const HALF_OPEN_PROBE_TIMEOUT_SECS: u64 = 30;

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
    ///
    /// Once an Open circuit's cooldown expires, exactly one caller leases the
    /// Half-Open probe slot. Other callers remain rejected until that probe reports
    /// success/failure or the bounded probe timeout expires.
    pub fn allow_request(&self, upstream_id: &str) -> bool {
        // A mutable entry guard makes claiming the Half-Open probe slot atomic for
        // this upstream. Unknown/healthy upstreams still do not allocate state.
        let mut entry = match self.states.get_mut(upstream_id) {
            Some(entry) => entry,
            None => return true,
        };

        if let Some(rate_limit_until) = entry.rate_limit_until {
            if rate_limit_until > Instant::now() {
                return false;
            }
        }

        let current_failures = entry.failure_count.load(Ordering::Relaxed);
        if current_failures < self.failure_threshold {
            return true;
        }

        let Some(last_failure) = entry.last_failure_time else {
            return false;
        };
        if last_failure.elapsed() < self.cooldown_duration {
            return false;
        }

        let now = Instant::now();
        let probe_timeout = Duration::from_secs(HALF_OPEN_PROBE_TIMEOUT_SECS);
        match entry.half_open_probe_started_at {
            None => {
                entry.half_open_probe_started_at = Some(now);
                true
            }
            Some(started_at) if started_at.elapsed() >= probe_timeout => {
                // The previous probe never reported a result. Re-lease the slot so
                // one lost caller cannot permanently disconnect this upstream.
                entry.half_open_probe_started_at = Some(now);
                true
            }
            Some(_) => false,
        }
    }

    /// Records a successful request.
    pub fn record_success(&self, upstream_id: &str) {
        if let Some(mut entry) = self.states.get_mut(upstream_id) {
            entry.failure_count.store(0, Ordering::Relaxed);
            entry.last_failure_time = None;
            entry.failure_type = None;
            entry.rate_limit_until = None;
            entry.half_open_probe_started_at = None;
        }
    }

    /// Records a failed request with a specific failure type.
    pub fn record_failure_with_type(&self, upstream_id: &str, failure_type: FailureType) {
        let mut entry = self.states.entry(upstream_id.to_string()).or_default();

        entry.failure_type = Some(failure_type.clone());
        entry.last_failure_time = Some(Instant::now());
        // Whether this failure came from the Half-Open probe or a Closed-state
        // request, it ends any outstanding probe lease. A future probe must first
        // wait for the newly-started cooldown.
        entry.half_open_probe_started_at = None;

        match failure_type {
            FailureType::AuthFailed | FailureType::PaymentRequired => {
                entry
                    .failure_count
                    .store(self.failure_threshold * 10, Ordering::Relaxed);
                entry.last_failure_time = Some(Instant::now());
                entry.rate_limit_until = Some(Instant::now() + Duration::from_secs(1800));
                tracing::warn!(
                    "Circuit Breaker: Upstream {} auth/payment failure ({:?}) — circuit tripped for 30 min",
                    upstream_id,
                    failure_type
                );
            }
            FailureType::RateLimited {
                scope: _,
                retry_after,
            } => {
                let duration = retry_after
                    .map(Duration::from_secs)
                    .unwrap_or(Duration::from_secs(DEFAULT_RATE_LIMIT_RETRY_SECS));
                entry.rate_limit_until = Some(Instant::now() + duration);

                let new_count = entry.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if new_count >= self.failure_threshold {
                    tracing::warn!(
                        "Circuit Breaker: Upstream {} tripped due to rate limit (Failures: {})",
                        upstream_id,
                        new_count
                    );
                }
            }
            _ => {
                let new_count = entry.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if new_count >= self.failure_threshold {
                    tracing::warn!(
                        "Circuit Breaker: Upstream {} tripped! (Failures: {})",
                        upstream_id,
                        new_count
                    );
                }
            }
        }
    }

    /// Emergency trip: force all known upstreams into Open state.
    pub fn trip_all(&self) -> Vec<String> {
        let mut tripped = Vec::new();
        for mut entry in self.states.iter_mut() {
            entry
                .failure_count
                .store(self.failure_threshold, Ordering::Relaxed);
            entry.last_failure_time = Some(Instant::now());
            entry.failure_type = Some(FailureType::ServerError);
            entry.rate_limit_until = None;
            entry.half_open_probe_started_at = None;
            tripped.push(entry.key().clone());
        }
        tracing::warn!(
            "Circuit Breaker: Emergency trip-all triggered — {} upstream(s) forced to Open",
            tripped.len()
        );
        tripped
    }

    /// Get current health status map for monitoring.
    ///
    /// Existing strings are intentionally preserved because the health endpoint may
    /// already depend on them.
    pub fn get_status_map(&self) -> std::collections::HashMap<String, String> {
        let mut map = std::collections::HashMap::new();
        for r in self.states.iter() {
            let count = r.value().failure_count.load(Ordering::Relaxed);
            let status = if count >= self.failure_threshold {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use std::sync::{Arc, Barrier};

    fn server_error() -> FailureType {
        FailureType::ServerError
    }

    fn rate_limited(retry_after: Option<u64>) -> FailureType {
        FailureType::RateLimited {
            scope: RateLimitScope::Account,
            retry_after,
        }
    }

    #[test]
    fn an_unknown_upstream_is_allowed() {
        let b = CircuitBreaker::new(3, 60);
        assert!(b.allow_request("never-seen"));
    }

    #[test]
    fn failures_below_the_threshold_keep_the_circuit_closed() {
        let b = CircuitBreaker::new(3, 60);
        b.record_failure_with_type("up", server_error());
        b.record_failure_with_type("up", server_error());
        assert!(b.allow_request("up"));
    }

    #[test]
    fn the_threshold_failure_trips_the_circuit() {
        let b = CircuitBreaker::new(2, 3600);
        b.record_failure_with_type("up", server_error());
        assert!(b.allow_request("up"));
        b.record_failure_with_type("up", server_error());
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn one_upstream_tripping_does_not_affect_another() {
        let b = CircuitBreaker::new(1, 3600);
        b.record_failure_with_type("bad", server_error());
        assert!(!b.allow_request("bad"));
        assert!(b.allow_request("good"));
    }

    #[test]
    fn an_open_circuit_stays_open_for_the_cooldown() {
        let b = CircuitBreaker::new(1, 3600);
        b.record_failure_with_type("up", server_error());
        for _ in 0..5 {
            assert!(!b.allow_request("up"));
        }
    }

    #[test]
    fn auth_failure_trips_immediately_and_for_longer_than_the_cooldown_alone() {
        let b = CircuitBreaker::new(3, 0);
        b.record_failure_with_type("up", FailureType::AuthFailed);
        assert!(!b.allow_request("up"));

        let b = CircuitBreaker::new(3, 0);
        b.record_failure_with_type("up", FailureType::PaymentRequired);
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn a_rate_limit_refuses_until_retry_after_elapses() {
        let b = CircuitBreaker::new(10, 0);
        b.record_failure_with_type("up", rate_limited(Some(3600)));
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn a_rate_limit_without_retry_after_uses_the_default_window() {
        let b = CircuitBreaker::new(10, 0);
        b.record_failure_with_type("up", rate_limited(None));
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn a_rate_limit_also_counts_toward_the_threshold() {
        let b = CircuitBreaker::new(2, 3600);
        b.record_failure_with_type("up", rate_limited(Some(0)));
        b.record_failure_with_type("up", rate_limited(Some(0)));
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn a_success_resets_every_field_of_the_state() {
        let b = CircuitBreaker::new(1, 3600);
        b.record_failure_with_type("up", FailureType::AuthFailed);
        assert!(!b.allow_request("up"));
        b.record_success("up");
        assert!(b.allow_request("up"));
    }

    #[test]
    fn a_success_for_an_unknown_upstream_is_a_no_op() {
        let b = CircuitBreaker::new(1, 3600);
        b.record_success("never-seen");
        assert!(b.allow_request("never-seen"));
        assert!(!b.get_status_map().contains_key("never-seen"));
    }

    #[test]
    fn the_half_open_branch_admits_one_request_not_all() {
        let b = CircuitBreaker::new(1, 0);
        b.record_failure_with_type("up", server_error());

        let admitted = (0..10).filter(|_| b.allow_request("up")).count();
        println!("requests admitted after the cooldown elapsed: {admitted} of 10");
        assert_eq!(admitted, 1);
    }

    #[test]
    fn the_half_open_state_is_left_by_a_failure_and_can_probe_again() {
        let b = CircuitBreaker::new(1, 0);
        b.record_failure_with_type("up", server_error());
        assert!(b.allow_request("up"));
        assert!(!b.allow_request("up"));

        b.record_failure_with_type("up", server_error());
        // A zero-duration cooldown has immediately elapsed, so clearing the old
        // probe lease permits one new probe rather than permanently opening it.
        assert!(b.allow_request("up"));
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn a_failed_probe_re_opens_the_circuit_for_the_full_cooldown() {
        let b = CircuitBreaker::new(1, 3600);
        b.record_failure_with_type("up", server_error());

        // Move only the test fixture's failure time behind the cooldown so the
        // first Half-Open probe can be exercised without sleeping an hour.
        {
            let mut state = b.states.get_mut("up").expect("state exists");
            state.last_failure_time = Some(Instant::now() - Duration::from_secs(3601));
        }
        assert!(b.allow_request("up"));
        b.record_failure_with_type("up", server_error());
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn a_lost_probe_slot_expires_and_allows_exactly_one_replacement() {
        let b = CircuitBreaker::new(1, 0);
        b.record_failure_with_type("up", server_error());
        assert!(b.allow_request("up"));
        assert!(!b.allow_request("up"));

        {
            let mut state = b.states.get_mut("up").expect("state exists");
            state.half_open_probe_started_at = Some(
                Instant::now() - Duration::from_secs(HALF_OPEN_PROBE_TIMEOUT_SECS + 1),
            );
        }

        assert!(b.allow_request("up"), "expired lease permits a replacement probe");
        assert!(!b.allow_request("up"), "replacement probe owns the only slot");
    }

    #[test]
    fn concurrent_half_open_callers_admit_exactly_one_probe() {
        let breaker = Arc::new(CircuitBreaker::new(1, 0));
        breaker.record_failure_with_type("up", server_error());

        let callers = 10;
        let barrier = Arc::new(Barrier::new(callers));
        let mut handles = Vec::new();
        for _ in 0..callers {
            let breaker = Arc::clone(&breaker);
            let barrier = Arc::clone(&barrier);
            handles.push(std::thread::spawn(move || {
                barrier.wait();
                breaker.allow_request("up")
            }));
        }

        let admitted = handles
            .into_iter()
            .filter(|handle| handle.join().unwrap_or(false))
            .count();
        assert_eq!(admitted, 1, "Half-Open must expose exactly one probe slot");
    }

    #[test]
    fn trip_all_forces_every_known_upstream_open() {
        let b = CircuitBreaker::new(5, 3600);
        b.record_failure_with_type("known-a", server_error());
        b.record_failure_with_type("known-b", server_error());

        let tripped = b.trip_all();
        let mut sorted = tripped.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec!["known-a".to_string(), "known-b".to_string()]
        );
        assert!(!b.allow_request("known-a"));
        assert!(!b.allow_request("known-b"));
        assert!(b.allow_request("never-seen"));
    }

    #[test]
    fn trip_all_forces_a_lightly_failed_upstream_open() {
        let b = CircuitBreaker::new(5, 3600);
        b.record_failure_with_type("up", rate_limited(Some(3600)));
        assert!(!b.allow_request("up"));

        let tripped = b.trip_all();
        assert_eq!(tripped, vec!["up".to_string()]);
        let state = b.get_status_map().get("up").cloned().unwrap_or_default();
        assert!(state.contains("Open"));
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn the_status_map_reports_a_state_for_every_known_upstream() {
        let b = CircuitBreaker::new(2, 3600);
        b.record_failure_with_type("open-up", server_error());
        b.record_failure_with_type("open-up", server_error());
        b.record_failure_with_type("closed-up", server_error());
        b.record_failure_with_type("recovered-up", server_error());
        b.record_failure_with_type("recovered-up", server_error());
        b.record_success("recovered-up");

        let map = b.get_status_map();
        assert!(map.contains_key("open-up"));
        assert!(map.contains_key("closed-up"));
        assert!(map.contains_key("recovered-up"));
        assert!(!map.contains_key("never-seen"));
        assert_ne!(map.get("open-up"), map.get("recovered-up"));
    }

    #[test]
    fn the_half_open_state_admits_one_probe() {
        let b = CircuitBreaker::new(1, 0);
        b.record_failure_with_type("up", server_error());
        assert!(b.allow_request("up"));
        assert!(!b.allow_request("up"));
    }
}
