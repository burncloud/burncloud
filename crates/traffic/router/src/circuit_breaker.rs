use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

/// Represents the scope of a rate limit.
///
/// Used to distinguish between account-level and model-level rate limits.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub enum RateLimitScope {
    /// Rate limit applies at the account level (affects all models)
    Account,
    /// Rate limit applies at the model level (specific model only)
    Model,
    /// Rate limit scope is unknown
    #[default]
    Unknown,
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
    /// Connection failed (DNS, TCP refused, TLS handshake)
    ConnectionError,
    /// Empty response (HTTP 200 but zero tokens)
    EmptyResponse,
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

/// Default retry duration when no retry_after header is provided.
const DEFAULT_RATE_LIMIT_RETRY_SECS: u64 = 60;

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
        // Read-only: use get() to avoid creating entries for healthy upstreams
        let entry = match self.states.get(upstream_id) {
            Some(e) => e,
            None => return true, // No state = never failed = allowed
        };

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
                // Auth/payment failures are permanent (bad key, exhausted quota).
                // Set high failure count + long cooldown (30 min) to avoid retrying.
                entry
                    .failure_count
                    .store(self.failure_threshold * 10, Ordering::Relaxed);
                entry.last_failure_time = Some(Instant::now());
                entry.rate_limit_until = Some(Instant::now() + Duration::from_secs(1800)); // 30 minutes
                tracing::warn!(
                    "Circuit Breaker: Upstream {} auth/payment failure ({:?}) — circuit tripped for 30 min",
                    upstream_id, failure_type
                );
            }
            FailureType::RateLimited {
                scope: _,
                retry_after,
            } => {
                // Set rate limit expiry time
                let duration = retry_after
                    .map(Duration::from_secs)
                    .unwrap_or(Duration::from_secs(DEFAULT_RATE_LIMIT_RETRY_SECS));
                entry.rate_limit_until = Some(Instant::now() + duration);

                // Also increment failure count for rate limits
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
                // Other failures just increment the count
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
    ///
    /// Sets each upstream's failure count to the threshold and records the
    /// current time as `last_failure_time`, so the circuit stays Open for
    /// the full cooldown duration. Returns the list of upstream IDs that
    /// were tripped.
    pub fn trip_all(&self) -> Vec<String> {
        let mut tripped = Vec::new();
        for mut entry in self.states.iter_mut() {
            entry
                .failure_count
                .store(self.failure_threshold, Ordering::Relaxed);
            entry.last_failure_time = Some(Instant::now());
            entry.failure_type = Some(FailureType::ServerError);
            entry.rate_limit_until = None;
            tripped.push(entry.key().clone());
        }
        tracing::warn!(
            "Circuit Breaker: Emergency trip-all triggered — {} upstream(s) forced to Open",
            tripped.len()
        );
        tripped
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    //! The circuit breaker's state machine (#633, plan section 5 item 12).
    //!
    //! `crates/traffic/router/src/circuit_breaker.rs` is 242 lines with **zero tests**, and it is the decision
    //! point of the failover loop: `lib.rs:2489` asks `allow_request` before every attempt, `record_success` at
    //! `:361`, `record_failure_with_type` at `:2698`/`:2729`, `trip_all` at `:1066` and `get_status_map` at
    //! `:1298`. Seven other files in the crate reference it. A wrong answer here does not produce a wrong number;
    //! it sends traffic to an upstream that is known to be failing, or keeps it away from one that has recovered.
    //!
    //! ## The state machine, read from the source
    //!
    //! ```text
    //! allow_request(id):
    //!   no state                          -> true                     (never failed)
    //!   rate_limit_until > now            -> false                    (still limited)
    //!   failure_count < threshold         -> true                     (closed)
    //!   last_failure.elapsed() >= cooldown-> true                     ("Half-Open: allow one probe")
    //!   otherwise                         -> false                    (open)
    //!
    //! record_failure_with_type(id, t):
    //!   AuthFailed | PaymentRequired      -> count = threshold * 10, rate_limit_until = now + 30min
    //!   RateLimited { retry_after }       -> rate_limit_until = now + retry_after (default 60s), count += 1
    //!   anything else                     -> count += 1
    //!
    //! record_success(id): if an entry exists -> count = 0, last_failure = None, rate_limit_until = None
    //! trip_all(): every known id -> count = threshold, last_failure = now, rate_limit_until = None
    //! ```
    //!
    //! ## The finding this file measures
    //!
    //! The comment at `:112` says "Transition to Half-Open (allow one probe)", but nothing is recorded to mark
    //! that a probe is in flight: the branch only reads `last_failure_time` and the failure count stays at the
    //! threshold. So once the cooldown elapses, **every** request is allowed, not one -- the upstream receives the
    //! full retry storm the open state existed to prevent, and it keeps receiving it until one of the requests
    //! succeeds and calls `record_success`.
    //!
    //! Measured rather than inferred: see `the_half_open_branch_admits_every_request_not_one`.
    //!
    //! ## Note on the cooldown tests
    //!
    //! Testing elapsed cooldowns needs real time to pass, so the tests below use a **0-second** cooldown, where
    //! `elapsed() >= 0` is immediately true. That exercises the same branch as a 30-second wait without sleeping,
    //! and it is a legitimate configuration (`CircuitBreaker::new(threshold, 0)`). The tests that need the open
    //! state to persist use a long cooldown, so nothing here depends on timing races.

    use super::*;

    fn server_error() -> FailureType {
        FailureType::ServerError
    }

    fn rate_limited(retry_after: Option<u64>) -> FailureType {
        FailureType::RateLimited {
            scope: RateLimitScope::Account,
            retry_after,
        }
    }

    /// Trip the breaker for one upstream by recording exactly `threshold` failures.
    fn trip(b: &CircuitBreaker, id: &str, threshold: u32) {
        for _ in 0..threshold {
            b.record_failure_with_type(id, server_error());
        }
    }

    // -------------------------------------------------------------------------------------------
    // the closed state
    // -------------------------------------------------------------------------------------------

    #[test]
    fn an_unknown_upstream_is_allowed() {
        // No state means never failed. `allow_request` uses `get` rather than `entry` so that a healthy upstream
        // never allocates, which is why the unknown case has to be handled explicitly.
        let b = CircuitBreaker::new(3, 60);
        assert!(b.allow_request("never-seen"));
    }

    #[test]
    fn failures_below_the_threshold_keep_the_circuit_closed() {
        let b = CircuitBreaker::new(3, 60);
        b.record_failure_with_type("up", server_error());
        b.record_failure_with_type("up", server_error());
        assert!(
            b.allow_request("up"),
            "two failures against a threshold of three must not trip"
        );
    }

    #[test]
    fn the_threshold_failure_trips_the_circuit() {
        // The boundary: the count reaching the threshold is enough, without needing one more.
        let b = CircuitBreaker::new(2, 3600);
        b.record_failure_with_type("up", server_error());
        assert!(
            b.allow_request("up"),
            "one failure, threshold two: still closed"
        );
        b.record_failure_with_type("up", server_error());
        assert!(
            !b.allow_request("up"),
            "the second failure reaches the threshold"
        );
    }

    #[test]
    fn one_upstream_tripping_does_not_affect_another() {
        // The breaker is keyed by upstream id, and the failover loop depends on that: a tripped channel must be
        // skipped while the next candidate stays usable.
        let b = CircuitBreaker::new(1, 3600);
        b.record_failure_with_type("bad", server_error());
        assert!(!b.allow_request("bad"));
        assert!(
            b.allow_request("good"),
            "a different upstream is unaffected"
        );
    }

    // -------------------------------------------------------------------------------------------
    // the open state
    // -------------------------------------------------------------------------------------------

    #[test]
    fn an_open_circuit_stays_open_for_the_cooldown() {
        let b = CircuitBreaker::new(1, 3600);
        b.record_failure_with_type("up", server_error());
        for _ in 0..5 {
            assert!(
                !b.allow_request("up"),
                "with an hour of cooldown left, every check must refuse"
            );
        }
    }

    #[test]
    fn auth_failure_trips_immediately_and_for_longer_than_the_cooldown_alone() {
        // Auth and payment failures are treated as permanent: the count is set to `threshold * 10` and a 30-minute
        // rate limit is applied. The rate limit is what refuses the request, independently of the cooldown --
        // here the cooldown is zero and the circuit still refuses.
        let b = CircuitBreaker::new(3, 0);
        b.record_failure_with_type("up", FailureType::AuthFailed);

        assert!(
        !b.allow_request("up"),
        "a single auth failure must refuse even with a zero cooldown, because of the 30-minute rate limit"
    );

        // The same for payment.
        let b = CircuitBreaker::new(3, 0);
        b.record_failure_with_type("up", FailureType::PaymentRequired);
        assert!(!b.allow_request("up"));
    }

    #[test]
    fn a_rate_limit_refuses_until_retry_after_elapses() {
        // `retry_after` is taken as seconds from now, and the failure also counts toward the threshold.
        let b = CircuitBreaker::new(10, 0);
        b.record_failure_with_type("up", rate_limited(Some(3600)));
        assert!(
        !b.allow_request("up"),
        "an hour of retry_after must refuse, even though the failure count is far below the threshold"
    );
    }

    #[test]
    fn a_rate_limit_without_retry_after_uses_the_default_window() {
        // The `None` case falls back to `DEFAULT_RATE_LIMIT_RETRY_SECS`, which is a minute.
        let b = CircuitBreaker::new(10, 0);
        b.record_failure_with_type("up", rate_limited(None));
        assert!(!b.allow_request("up"), "the default window is in force");
    }

    #[test]
    fn a_rate_limit_also_counts_toward_the_threshold() {
        // The `RateLimited` arm increments the count as well as setting the window, so enough rate limits trip the
        // circuit even after the last window expires.
        let b = CircuitBreaker::new(2, 3600);
        b.record_failure_with_type("up", rate_limited(Some(0)));
        b.record_failure_with_type("up", rate_limited(Some(0)));
        assert!(
        !b.allow_request("up"),
        "two rate limits reach a threshold of two, and the cooldown then holds the circuit open"
    );
    }

    // -------------------------------------------------------------------------------------------
    // recovery
    // -------------------------------------------------------------------------------------------

    #[test]
    fn a_success_resets_every_field_of_the_state() {
        // `record_success` clears the count, the last-failure time and the rate limit, so the upstream is treated
        // as never having failed.
        let b = CircuitBreaker::new(1, 3600);
        b.record_failure_with_type("up", FailureType::AuthFailed);
        assert!(!b.allow_request("up"));

        b.record_success("up");

        assert!(
        b.allow_request("up"),
        "after a success the circuit must be closed again, including the 30-minute auth rate limit"
    );
    }

    #[test]
    fn a_success_for_an_unknown_upstream_is_a_no_op() {
        // `record_success` uses `get_mut`, so it creates nothing. The consequence is that a success cannot
        // pre-register an upstream -- and also that it cannot clear state that was never there.
        let b = CircuitBreaker::new(1, 3600);
        b.record_success("never-seen");
        assert!(b.allow_request("never-seen"));

        // And the status map stays empty, which is how the health endpoint distinguishes "known and healthy" from
        // "never seen".
        assert!(
            !b.get_status_map().contains_key("never-seen"),
            "recording a success must not create an entry"
        );
    }

    // -------------------------------------------------------------------------------------------
    // the half-open branch, which is where the finding is
    // -------------------------------------------------------------------------------------------

    #[test]
    fn the_half_open_branch_admits_every_request_not_one() {
        // **The finding.** The source comment at `:112` says "Transition to Half-Open (allow one probe)". Nothing
        // is recorded to mark a probe in flight, so once the cooldown has elapsed the check returns true for every
        // caller, however many there are.
        //
        // A zero cooldown reaches that branch immediately: `last_failure.elapsed() >= 0` is true. The same branch
        // is reached with a 30-second cooldown after thirty seconds of real time.
        let b = CircuitBreaker::new(1, 0);
        b.record_failure_with_type("up", server_error());

        let admitted = (0..10).filter(|_| b.allow_request("up")).count();
        println!("requests admitted after the cooldown elapsed: {admitted} of 10");

        assert_eq!(
        admitted, 10,
        "every request is admitted, not one. If this is 1 the half-open gate has been implemented, which is \
         what the alternative in the issue asks for"
    );
    }

    #[test]
    fn the_half_open_state_is_not_left_by_a_failure() {
        // The other half of the same problem: a probe that fails does not re-open the circuit either. The count is
        // already at or above the threshold and the failure pushes it higher, but `last_failure_time` is refreshed
        // so the circuit does return to Open -- for the length of the cooldown. With a zero cooldown that is no
        // time at all, so it keeps admitting.
        let b = CircuitBreaker::new(1, 0);
        b.record_failure_with_type("up", server_error());

        b.allow_request("up"); // the "probe"
        b.record_failure_with_type("up", server_error()); // and it failed

        assert!(
        b.allow_request("up"),
        "with a zero cooldown a failed probe still admits, so the breaker never holds traffic back"
    );
    }

    #[test]
    fn a_failed_probe_re_opens_the_circuit_for_the_full_cooldown() {
        // The useful version of the above, with a real cooldown: the probe is admitted once the cooldown elapses,
        // and its failure restarts the clock.
        let b = CircuitBreaker::new(1, 0);
        b.record_failure_with_type("up", server_error());
        assert!(
            b.allow_request("up"),
            "the zero cooldown has already elapsed"
        );

        // A fresh breaker with a real cooldown, to show the clock restarting.
        let b = CircuitBreaker::new(1, 3600);
        b.record_failure_with_type("up", server_error());
        assert!(!b.allow_request("up"), "open");
        b.record_failure_with_type("up", server_error());
        assert!(
            !b.allow_request("up"),
            "and a further failure keeps it open rather than letting it through"
        );
    }

    // -------------------------------------------------------------------------------------------
    // trip_all and the status map
    // -------------------------------------------------------------------------------------------

    #[test]
    fn trip_all_forces_every_known_upstream_open() {
        // The emergency path used at `lib.rs:1066`. It only touches upstreams that already have state, because it
        // iterates the map -- so an upstream that has never failed is not affected, which is worth stating since
        // "trip all" reads as "all".
        let b = CircuitBreaker::new(5, 3600);
        b.record_failure_with_type("known-a", server_error());
        b.record_failure_with_type("known-b", server_error());

        let tripped = b.trip_all();

        let mut sorted = tripped.clone();
        sorted.sort();
        assert_eq!(
            sorted,
            vec!["known-a".to_string(), "known-b".to_string()],
            "both known upstreams are reported as tripped"
        );
        assert!(!b.allow_request("known-a"));
        assert!(!b.allow_request("known-b"));
        assert!(
            b.allow_request("never-seen"),
            "an upstream with no state is not tripped, despite the method's name"
        );
    }

    #[test]
    fn trip_all_forces_a_lightly_failed_upstream_open() {
        // `trip_all` raises every known upstream's count to the threshold. This one had a single rate-limit
        // failure against a threshold of five, so without the trip it would be admitted again once its window
        // expired.
        //
        // **A note on what cannot be observed.** `trip_all` also sets `rate_limit_until = None`, and the
        // obvious way to test that -- arrange a window that outlasts the cooldown and show the request is
        // admitted afterwards -- does not work: `trip_all` pushes the count to the threshold, so `allow_request`
        // takes the open branch and checks `last_failure_time.elapsed() >= cooldown`, which refuses regardless
        // of what the window says. The two mechanisms are not separable through `allow_request`.
        //
        // An earlier version of this test tried, with a long `retry_after` and a long cooldown, and it passed
        // for the wrong reason: a mutation that left the window in place **was not caught**. What follows
        // asserts the part that is observable, and reads the status rather than inferring it.
        let b = CircuitBreaker::new(5, 3600);
        b.record_failure_with_type("up", rate_limited(Some(3600)));
        assert!(
            !b.allow_request("up"),
            "an hour of retry_after refuses while the count is still one"
        );

        let tripped = b.trip_all();
        assert_eq!(
            tripped,
            vec!["up".to_string()],
            "the upstream is reported as tripped"
        );

        let state = b.get_status_map().get("up").cloned().unwrap_or_default();
        println!("status after trip_all on a lightly-failed upstream: {state:?}");

        assert!(
            state.contains("Open"),
            "a single rate-limit failure becomes Open after trip_all; got {state:?}"
        );
        assert!(
            !b.allow_request("up"),
            "and it stays refused, because the count now equals the threshold"
        );
    }

    #[test]
    fn the_status_map_reports_a_state_for_every_known_upstream() {
        // Used by the health endpoint at `lib.rs:1298`, so the shape matters as much as the content.
        let b = CircuitBreaker::new(2, 3600);
        b.record_failure_with_type("open-up", server_error());
        b.record_failure_with_type("open-up", server_error());
        b.record_failure_with_type("closed-up", server_error());
        b.record_success("recovered-up");
        b.record_failure_with_type("recovered-up", server_error());
        b.record_failure_with_type("recovered-up", server_error());
        b.record_success("recovered-up");

        let map = b.get_status_map();
        println!("status map: {map:?}");

        assert!(
            map.contains_key("open-up"),
            "a tripped upstream is reported"
        );
        assert!(
            map.contains_key("closed-up"),
            "one below the threshold is reported"
        );
        assert!(
            map.contains_key("recovered-up"),
            "and a recovered one keeps its entry, at a closed status"
        );
        assert!(
            !map.contains_key("never-seen"),
            "an upstream that was never seen has no status"
        );

        assert_ne!(
        map.get("open-up"),
        map.get("recovered-up"),
        "a tripped upstream and a recovered one must not report the same status, or the endpoint cannot be \
         used to tell them apart"
    );
    }

    // -------------------------------------------------------------------------------------------
    // the decision
    // -------------------------------------------------------------------------------------------

    /// The property the source comment promises: "Transition to Half-Open (allow one probe)".
    ///
    /// **Ignored, not deleted**, and failing today. It does not prescribe the mechanism -- a timestamp, a counter,
    /// a flag, or handing the probe slot to the failover loop -- only that after the cooldown the breaker admits a
    /// bounded number of requests rather than all of them. Two is the smallest bound that distinguishes "one
    /// probe" from "unbounded", so the assertion is `<= 1` on the second call.
    #[ignore = "the contract: the half-open state must admit one probe, not every request"]
    #[test]
    fn the_half_open_state_admits_one_probe() {
        let b = CircuitBreaker::new(1, 0);
        b.record_failure_with_type("up", server_error());

        assert!(
            b.allow_request("up"),
            "the first check after the cooldown is the probe"
        );
        assert!(
        !b.allow_request("up"),
        "the second must be refused while the probe is in flight; today every check is admitted, which is the \
         retry storm the open state exists to prevent"
    );
    }
}
