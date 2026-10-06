//! Request-class-scoped retry budget for the failover loop (#680).
//!
//! # The defect this exists to close
//!
//! The failover loop attempts each candidate once, and the candidate list was
//! capped by a literal `take(5)` in the router while every attempt ran under one
//! shared HTTP client timeout of `HTTP_REQUEST_TIMEOUT_SECS = 36000` — ten
//! hours, which is deliberate for long-running media generation. Nothing bounded
//! the **total**, and nothing compared elapsed time against a deadline
//! (`elapsed()` was only ever read to record a latency).
//!
//! So one client request could occupy a connection for up to `5 × 10 h = 50 h`,
//! and for that whole time its [`crate::rate_budget::BudgetGuard`] held the
//! channel's TPM reservation — a hung upstream could pin a channel's budget for
//! two days.
//!
//! # Why it is per class, not one number
//!
//! The ten-hour timeout is *load-bearing* for video, audio and music generation:
//! those are legitimately long tasks, and replacing it with an interactive
//! deadline would kill them — a worse outcome than the defect. So the policy is
//! chosen per request class:
//!
//! | class | attempts | per attempt | total deadline |
//! | --- | --- | --- | --- |
//! | [`RequestClass::Interactive`] | 3 | 120 s | 300 s |
//! | [`RequestClass::LongTask`] | 2 | 10 h (unchanged) | 20 h |
//!
//! The long-task deadline is therefore **still bounded** — it exists and is
//! configurable, which is what the issue asks for — while its per-attempt
//! timeout is untouched.
//!
//! # Reading the budget
//!
//! Everything the loop needs is named: `max_attempts`, `attempt_timeout`,
//! `total_deadline`, and [`RetryBudget::candidate_limit`]. A reader looking for
//! "retry budget" finds this module instead of inferring a budget from a
//! `take(5)` in the candidate list.

use std::time::Duration;

/// Which retry policy a request gets.
///
/// The split is the whole point: one global deadline cannot serve both a chat
/// completion (seconds) and a video generation (hours).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestClass {
    /// Chat completions, embeddings, rerank — a user is waiting, so both the
    /// per-attempt timeout and the total are short.
    Interactive,
    /// Video / audio / music / speech generation — a long task by nature. The
    /// per-attempt timeout stays at the ten-hour value the code documents as
    /// deliberate; only the *total* is newly bounded.
    LongTask,
}

impl RequestClass {
    /// Classify a request from its path.
    ///
    /// Matched on path prefixes rather than on the model name because the path
    /// is what the upstream protocol defines as a long-running operation, and it
    /// is available before routing.
    ///
    /// A `GET /v1/videos/{task_id}` poll is also a long task: it waits on work
    /// already started, so it must not be cut off by the interactive deadline.
    pub fn for_path(path: &str) -> Self {
        const LONG_TASK_PREFIXES: [&str; 7] = [
            "/v1/video",
            "/v1/audio",
            "/v1/music",
            "/v1/speech",
            "/v1/images/generations",
            "/v1/realtime",
            "/v1/batches",
        ];
        if LONG_TASK_PREFIXES
            .iter()
            .any(|prefix| path.starts_with(prefix))
        {
            Self::LongTask
        } else {
            Self::Interactive
        }
    }

    /// Static label for logs and `router_logs.layer_decision` context.
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::LongTask => "long_task",
        }
    }
}

/// One class's policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Hard cap on attempts for one client request.
    ///
    /// This is the retry budget proper: how many upstreams may be tried before
    /// the request fails. It is deliberately **smaller** than the candidate list
    /// the router produces, so a request has ranked fallbacks in reserve rather
    /// than spending its whole budget on the worst-ranked channels.
    pub max_attempts: usize,
    /// Timeout applied to each individual upstream attempt.
    pub attempt_timeout: Duration,
    /// Total time one client request may spend in the failover loop, across all
    /// attempts. `None` would mean unbounded, which is the state this module
    /// exists to remove, so no built-in policy uses it.
    pub total_deadline: Duration,
}

impl RetryPolicy {
    /// The per-attempt timeout is never larger than the total: a single attempt
    /// that cannot finish inside the deadline would otherwise hold the request
    /// (and its TPM reservation) past the budget.
    ///
    /// The total is *not* divided evenly across attempts, because an attempt
    /// that succeeds on the first try should be allowed the full remaining
    /// budget — only the deadline itself bounds it.
    pub fn attempt_timeout_within(&self, remaining: Duration) -> Duration {
        self.attempt_timeout.min(remaining)
    }
}

/// The retry budget for every request class.
#[derive(Debug, Clone, Copy)]
pub struct RetryBudget {
    interactive: RetryPolicy,
    long_task: RetryPolicy,
    /// How many ranked candidates the router should produce.
    ///
    /// Deliberately larger than any class's `max_attempts`: the candidate list is
    /// the *reserve* of ranked fallbacks, while the attempt cap is what the
    /// request may spend. Keeping them separate means the router's existing
    /// "top-5 ranked fallbacks" contract survives (L3 affinity and the scorer
    /// both assume it) while the number of attempts becomes a real constraint.
    candidate_limit: usize,
}

/// The number of ranked fallback candidates the router produces per request.
///
/// Unchanged from the literal `take(5)` this replaced — now named, so a reader
/// finds the candidate contract instead of inferring it.
pub const DEFAULT_CANDIDATE_LIMIT: usize = 5;

impl Default for RetryBudget {
    fn default() -> Self {
        Self {
            // A user is waiting on a chat completion. Three attempts inside five
            // minutes is generous for an interactive path; the previous effective
            // policy was five attempts inside fifty hours.
            interactive: RetryPolicy {
                max_attempts: 3,
                attempt_timeout: Duration::from_secs(120),
                total_deadline: Duration::from_secs(300),
            },
            // Video/audio/music generation, where a single upstream call may
            // legitimately run for the ten hours `HTTP_REQUEST_TIMEOUT_SECS`
            // documents. Two attempts, so the total is bounded at twenty hours
            // rather than fifty, and the interactive deadline can never reach it.
            long_task: RetryPolicy {
                max_attempts: 2,
                attempt_timeout: Duration::from_secs(36_000),
                total_deadline: Duration::from_secs(72_000),
            },
            candidate_limit: DEFAULT_CANDIDATE_LIMIT,
        }
    }
}

impl RetryBudget {
    /// Build a budget from explicit policies, keeping the default candidate
    /// limit. Used by tests and by any caller that wants to configure the bounds.
    pub fn new(interactive: RetryPolicy, long_task: RetryPolicy) -> Self {
        Self {
            interactive,
            long_task,
            candidate_limit: DEFAULT_CANDIDATE_LIMIT,
        }
    }

    /// Override how many ranked candidates the router produces.
    pub fn with_candidate_limit(mut self, candidate_limit: usize) -> Self {
        self.candidate_limit = candidate_limit.max(1);
        self
    }

    /// The policy for a class.
    pub fn policy(&self, class: RequestClass) -> RetryPolicy {
        match class {
            RequestClass::Interactive => self.interactive,
            RequestClass::LongTask => self.long_task,
        }
    }

    /// The policy for a request path — the entry point the failover loop uses.
    pub fn policy_for_path(&self, path: &str) -> RetryPolicy {
        self.policy(RequestClass::for_path(path))
    }

    /// How many candidates the router should produce.
    ///
    /// At least the largest attempt cap, so the budget can always be spent even
    /// if a caller sets a generous `max_attempts`.
    pub fn candidate_limit(&self) -> usize {
        self.candidate_limit
            .max(self.interactive.max_attempts)
            .max(self.long_task.max_attempts)
    }
}

/// Why the failover loop stopped trying candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetExhaustion {
    /// The attempt cap was reached (`max_attempts` candidates were tried).
    AttemptsExhausted {
        attempts: usize,
        max_attempts: usize,
    },
    /// The total deadline passed. `elapsed_ms` is what makes the message useful
    /// in a report; `over_by_ms` says how far past the deadline the check ran.
    DeadlineExceeded {
        elapsed_ms: u128,
        deadline_ms: u128,
        over_by_ms: u128,
    },
}

impl BudgetExhaustion {
    /// One line for the client-visible `last_error` (#662). Not a log line: a
    /// budget exhaustion must be distinguishable from an upstream failure in the
    /// 502 body, because the operator response is completely different.
    pub fn reason(&self) -> String {
        match self {
            Self::AttemptsExhausted {
                attempts,
                max_attempts,
            } => format!("retry budget exhausted: {attempts} of {max_attempts} attempts used"),
            Self::DeadlineExceeded {
                elapsed_ms,
                deadline_ms,
                over_by_ms,
            } => format!(
                "retry budget deadline exceeded: {elapsed_ms} ms elapsed against a {deadline_ms} ms deadline \
                 ({over_by_ms} ms over)"
            ),
        }
    }
}

/// Evaluates a request's retry budget as the failover loop consumes it.
///
/// Constructed once per client request, before the loop, from the request path.
#[derive(Debug, Clone)]
pub struct RetryBudgetGuard {
    class: RequestClass,
    policy: RetryPolicy,
    attempts: usize,
    started: std::time::Instant,
}

impl RetryBudgetGuard {
    /// Start tracking a request of the given class.
    pub fn start(class: RequestClass, policy: RetryPolicy) -> Self {
        Self {
            class,
            policy,
            attempts: 0,
            started: std::time::Instant::now(),
        }
    }

    /// Start tracking a request from its path.
    pub fn for_path(path: &str, budget: &RetryBudget) -> Self {
        let class = RequestClass::for_path(path);
        Self::start(class, budget.policy(class))
    }

    pub fn class(&self) -> RequestClass {
        self.class
    }

    pub fn policy(&self) -> RetryPolicy {
        self.policy
    }

    pub fn attempts(&self) -> usize {
        self.attempts
    }

    /// Time left before the deadline; zero once it has passed.
    pub fn remaining(&self) -> Duration {
        self.policy
            .total_deadline
            .saturating_sub(self.started.elapsed())
    }

    /// The timeout to apply to the attempt about to be sent.
    pub fn per_attempt_timeout(&self) -> Duration {
        self.policy.attempt_timeout_within(self.remaining())
    }

    /// Consume one attempt slot, or report why the budget is spent.
    ///
    /// Called immediately before each candidate is tried. Checking the deadline
    /// first means a request that has no time left does not start an attempt it
    /// would have to abandon, so the connection and the TPM reservation are
    /// released at the loop's top rather than hours later.
    pub fn try_consume_attempt(&mut self) -> Result<(), BudgetExhaustion> {
        let elapsed = self.started.elapsed();
        if elapsed >= self.policy.total_deadline {
            return Err(BudgetExhaustion::DeadlineExceeded {
                elapsed_ms: elapsed.as_millis(),
                deadline_ms: self.policy.total_deadline.as_millis(),
                over_by_ms: elapsed
                    .saturating_sub(self.policy.total_deadline)
                    .as_millis(),
            });
        }
        if self.attempts >= self.policy.max_attempts {
            return Err(BudgetExhaustion::AttemptsExhausted {
                attempts: self.attempts,
                max_attempts: self.policy.max_attempts,
            });
        }
        self.attempts += 1;
        Ok(())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn chat_and_embeddings_are_interactive() {
        for path in [
            "/v1/chat/completions",
            "/v1/embeddings",
            "/v1/rerank",
            "/v1/messages",
        ] {
            assert_eq!(
                RequestClass::for_path(path),
                RequestClass::Interactive,
                "{path} is a request a user is waiting on"
            );
        }
    }

    #[test]
    fn media_generation_paths_are_long_tasks() {
        for path in [
            "/v1/video/generations",
            "/v1/videos/task-123",
            "/v1/audio/transcriptions",
            "/v1/music/generations",
            "/v1/speech",
            "/v1/images/generations",
        ] {
            assert_eq!(
                RequestClass::for_path(path),
                RequestClass::LongTask,
                "{path} is a long-running operation and must keep its long timeout"
            );
        }
    }

    #[test]
    fn the_interactive_deadline_is_seconds_and_the_long_task_one_is_hours() {
        // The two policies must not be the same number: collapsing them would
        // either kill video generation or leave chat requests unbounded.
        let interactive = RetryBudget::default().policy(RequestClass::Interactive);
        let long_task = RetryBudget::default().policy(RequestClass::LongTask);

        assert!(
            interactive.total_deadline <= Duration::from_secs(600),
            "an interactive request must have a bounded deadline in the minutes, not hours"
        );
        assert!(
            long_task.total_deadline >= Duration::from_secs(36_000),
            "a long task must still be allowed the ten-hour per-attempt window"
        );
        assert!(
            long_task.total_deadline > interactive.total_deadline * 100,
            "the classes must be orders of magnitude apart, not merely different"
        );
    }

    #[test]
    fn the_long_task_attempt_timeout_is_unchanged_from_the_shared_client() {
        // The ten-hour value is deliberate; this pins that the long-task class
        // still gets it. If this fails, video generation was shortened.
        let policy = RetryBudget::default().policy(RequestClass::LongTask);
        assert_eq!(
            policy.attempt_timeout,
            Duration::from_secs(36_000),
            "the long-task per-attempt timeout must remain the value HTTP_REQUEST_TIMEOUT_SECS documents"
        );
    }

    #[test]
    fn attempts_are_capped_and_the_cap_is_reported() {
        let mut guard = RetryBudgetGuard::start(
            RequestClass::Interactive,
            RetryPolicy {
                max_attempts: 2,
                attempt_timeout: Duration::from_secs(1),
                total_deadline: Duration::from_secs(3600),
            },
        );

        assert!(guard.try_consume_attempt().is_ok());
        assert!(guard.try_consume_attempt().is_ok());
        let exhausted = guard
            .try_consume_attempt()
            .expect_err("a third attempt must be refused");
        assert_eq!(
            exhausted,
            BudgetExhaustion::AttemptsExhausted {
                attempts: 2,
                max_attempts: 2
            }
        );
        assert!(
            exhausted.reason().contains("2 of 2 attempts"),
            "the reason must name the cap: {}",
            exhausted.reason()
        );
    }

    #[test]
    fn an_expired_deadline_stops_the_request_even_with_attempts_left() {
        // The property the defect was about: attempts remaining is not enough if
        // the total time is gone.
        let mut guard = RetryBudgetGuard::start(
            RequestClass::Interactive,
            RetryPolicy {
                max_attempts: 5,
                attempt_timeout: Duration::from_secs(1),
                total_deadline: Duration::ZERO,
            },
        );

        let exhausted = guard
            .try_consume_attempt()
            .expect_err("a zero deadline must refuse the first attempt");
        match exhausted {
            BudgetExhaustion::DeadlineExceeded {
                deadline_ms,
                over_by_ms,
                ..
            } => {
                assert_eq!(deadline_ms, 0);
                assert!(over_by_ms > 0 || deadline_ms == 0);
            }
            other => panic!("expected a deadline exhaustion, got {other:?}"),
        }
        assert_eq!(
            guard.attempts(),
            0,
            "a refused attempt must not be counted as used"
        );
    }

    #[test]
    fn the_per_attempt_timeout_shrinks_to_fit_the_remaining_budget() {
        let guard = RetryBudgetGuard::start(
            RequestClass::Interactive,
            RetryPolicy {
                max_attempts: 3,
                attempt_timeout: Duration::from_secs(3600),
                total_deadline: Duration::from_secs(10),
            },
        );

        let timeout = guard.per_attempt_timeout();
        assert!(
            timeout <= Duration::from_secs(10),
            "one attempt must not be allowed to outlive the total deadline, got {timeout:?}"
        );
    }

    #[test]
    fn the_candidate_limit_covers_every_attempt_cap() {
        // The router's candidate list is the reserve of ranked fallbacks; it must
        // never be shorter than what the budget is allowed to spend, or a request
        // would run out of candidates before it ran out of budget.
        let budget = RetryBudget::default();
        for class in [RequestClass::Interactive, RequestClass::LongTask] {
            assert!(
                budget.candidate_limit() >= budget.policy(class).max_attempts,
                "{} allows {} attempts but only {} candidates exist",
                class.as_label(),
                budget.policy(class).max_attempts,
                budget.candidate_limit()
            );
        }

        // A caller that asks for a very generous attempt cap still gets enough
        // candidates, rather than a silent truncation.
        let generous = RetryBudget::new(
            RetryPolicy {
                max_attempts: 9,
                attempt_timeout: Duration::from_secs(1),
                total_deadline: Duration::from_secs(10),
            },
            RetryBudget::default().policy(RequestClass::LongTask),
        );
        assert_eq!(generous.candidate_limit(), 9);
    }

    #[test]
    fn the_default_candidate_limit_keeps_the_ranked_fallback_contract() {
        // The literal `take(5)` was a routing detail, not a budget — but it is a
        // contract other layers assume, so it is preserved by name.
        assert_eq!(RetryBudget::default().candidate_limit(), 5);
        assert_eq!(DEFAULT_CANDIDATE_LIMIT, 5);
    }

    #[test]
    fn the_old_worst_case_is_no_longer_reachable() {
        // The measured baseline in `retry_budget.rs` was 5 × 36_000 s = 50 h with
        // no overall deadline. Both classes are now bounded well inside that.
        const OLD_WORST_CASE_SECS: u64 = 36_000 * 5;
        let budget = RetryBudget::default();
        for class in [RequestClass::Interactive, RequestClass::LongTask] {
            let policy = budget.policy(class);
            let worst_case = policy.attempt_timeout.as_secs() * policy.max_attempts as u64;
            assert!(
                policy.total_deadline.as_secs() <= OLD_WORST_CASE_SECS,
                "{} must be bounded at or below the old worst case",
                class.as_label()
            );
            println!(
                "{}: {} attempts x {} s = {} s worst case, deadline {} s",
                class.as_label(),
                policy.max_attempts,
                policy.attempt_timeout.as_secs(),
                worst_case,
                policy.total_deadline.as_secs()
            );
        }

        let interactive = budget.policy(RequestClass::Interactive);
        let worst = interactive.attempt_timeout.as_secs() * interactive.max_attempts as u64;
        assert_eq!(
            worst, 360,
            "the interactive worst case is six minutes, not fifty hours"
        );
    }
}
