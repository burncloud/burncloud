//! Request-class-scoped retry budget for the failover loop (#680).
//!
//! The old effective policy was five ranked candidates multiplied by one shared
//! ten-hour HTTP timeout, with no overall deadline. This module makes the policy
//! explicit, keeps long-running media requests viable, and lets operators tune
//! the bounds without recompiling the router.
//!
//! Production configuration is read from these environment variables:
//!
//! - `BURNCLOUD_RETRY_INTERACTIVE_MAX_ATTEMPTS`
//! - `BURNCLOUD_RETRY_INTERACTIVE_ATTEMPT_TIMEOUT_SECS`
//! - `BURNCLOUD_RETRY_INTERACTIVE_TOTAL_DEADLINE_SECS`
//! - `BURNCLOUD_RETRY_LONG_TASK_MAX_ATTEMPTS`
//! - `BURNCLOUD_RETRY_LONG_TASK_ATTEMPT_TIMEOUT_SECS`
//! - `BURNCLOUD_RETRY_LONG_TASK_TOTAL_DEADLINE_SECS`
//! - `BURNCLOUD_RETRY_CANDIDATE_LIMIT`
//!
//! Defaults remain 3 × 120 s with a 300 s interactive deadline, and 2 × 10 h
//! with a 20 h long-task deadline.

use std::time::Duration;

const DEFAULT_INTERACTIVE_MAX_ATTEMPTS: usize = 3;
const DEFAULT_INTERACTIVE_ATTEMPT_TIMEOUT_SECS: u64 = 120;
const DEFAULT_INTERACTIVE_TOTAL_DEADLINE_SECS: u64 = 300;
const DEFAULT_LONG_TASK_MAX_ATTEMPTS: usize = 2;
const DEFAULT_LONG_TASK_ATTEMPT_TIMEOUT_SECS: u64 = 36_000;
const DEFAULT_LONG_TASK_TOTAL_DEADLINE_SECS: u64 = 72_000;
pub const DEFAULT_CANDIDATE_LIMIT: usize = 5;

const ENV_INTERACTIVE_MAX_ATTEMPTS: &str = "BURNCLOUD_RETRY_INTERACTIVE_MAX_ATTEMPTS";
const ENV_INTERACTIVE_ATTEMPT_TIMEOUT_SECS: &str =
    "BURNCLOUD_RETRY_INTERACTIVE_ATTEMPT_TIMEOUT_SECS";
const ENV_INTERACTIVE_TOTAL_DEADLINE_SECS: &str = "BURNCLOUD_RETRY_INTERACTIVE_TOTAL_DEADLINE_SECS";
const ENV_LONG_TASK_MAX_ATTEMPTS: &str = "BURNCLOUD_RETRY_LONG_TASK_MAX_ATTEMPTS";
const ENV_LONG_TASK_ATTEMPT_TIMEOUT_SECS: &str = "BURNCLOUD_RETRY_LONG_TASK_ATTEMPT_TIMEOUT_SECS";
const ENV_LONG_TASK_TOTAL_DEADLINE_SECS: &str = "BURNCLOUD_RETRY_LONG_TASK_TOTAL_DEADLINE_SECS";
const ENV_CANDIDATE_LIMIT: &str = "BURNCLOUD_RETRY_CANDIDATE_LIMIT";

/// Which retry policy a request gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestClass {
    /// Chat completions, embeddings, rerank and similar interactive requests.
    Interactive,
    /// Video/audio/music/speech generation and other deliberately long tasks.
    LongTask,
}

impl RequestClass {
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

    pub fn as_label(&self) -> &'static str {
        match self {
            Self::Interactive => "interactive",
            Self::LongTask => "long_task",
        }
    }
}

/// One request class's retry policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Maximum number of real upstream HTTP attempts.
    pub max_attempts: usize,
    /// Timeout applied to one upstream HTTP attempt.
    pub attempt_timeout: Duration,
    /// Total time allowed for the whole failover loop.
    pub total_deadline: Duration,
}

impl RetryPolicy {
    pub fn attempt_timeout_within(&self, remaining: Duration) -> Duration {
        self.attempt_timeout.min(remaining)
    }
}

/// Retry policy for all request classes.
#[derive(Debug, Clone, Copy)]
pub struct RetryBudget {
    interactive: RetryPolicy,
    long_task: RetryPolicy,
    candidate_limit: usize,
}

impl Default for RetryBudget {
    fn default() -> Self {
        Self::from_env()
    }
}

impl RetryBudget {
    pub fn new(interactive: RetryPolicy, long_task: RetryPolicy) -> Self {
        Self {
            interactive,
            long_task,
            candidate_limit: DEFAULT_CANDIDATE_LIMIT,
        }
    }

    /// Read the production retry policy from environment variables, with safe
    /// built-in defaults when a value is missing or malformed.
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    fn from_lookup<F>(mut lookup: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let interactive = RetryPolicy {
            max_attempts: lookup_usize(
                &mut lookup,
                ENV_INTERACTIVE_MAX_ATTEMPTS,
                DEFAULT_INTERACTIVE_MAX_ATTEMPTS,
            ),
            attempt_timeout: Duration::from_secs(lookup_u64(
                &mut lookup,
                ENV_INTERACTIVE_ATTEMPT_TIMEOUT_SECS,
                DEFAULT_INTERACTIVE_ATTEMPT_TIMEOUT_SECS,
            )),
            total_deadline: Duration::from_secs(lookup_u64(
                &mut lookup,
                ENV_INTERACTIVE_TOTAL_DEADLINE_SECS,
                DEFAULT_INTERACTIVE_TOTAL_DEADLINE_SECS,
            )),
        };
        let long_task = RetryPolicy {
            max_attempts: lookup_usize(
                &mut lookup,
                ENV_LONG_TASK_MAX_ATTEMPTS,
                DEFAULT_LONG_TASK_MAX_ATTEMPTS,
            ),
            attempt_timeout: Duration::from_secs(lookup_u64(
                &mut lookup,
                ENV_LONG_TASK_ATTEMPT_TIMEOUT_SECS,
                DEFAULT_LONG_TASK_ATTEMPT_TIMEOUT_SECS,
            )),
            total_deadline: Duration::from_secs(lookup_u64(
                &mut lookup,
                ENV_LONG_TASK_TOTAL_DEADLINE_SECS,
                DEFAULT_LONG_TASK_TOTAL_DEADLINE_SECS,
            )),
        };
        let candidate_limit =
            lookup_usize(&mut lookup, ENV_CANDIDATE_LIMIT, DEFAULT_CANDIDATE_LIMIT);
        Self {
            interactive,
            long_task,
            candidate_limit,
        }
    }

    pub fn with_candidate_limit(mut self, candidate_limit: usize) -> Self {
        self.candidate_limit = candidate_limit.max(1);
        self
    }

    pub fn policy(&self, class: RequestClass) -> RetryPolicy {
        match class {
            RequestClass::Interactive => self.interactive,
            RequestClass::LongTask => self.long_task,
        }
    }

    pub fn policy_for_path(&self, path: &str) -> RetryPolicy {
        self.policy(RequestClass::for_path(path))
    }

    pub fn candidate_limit(&self) -> usize {
        self.candidate_limit
            .max(self.interactive.max_attempts)
            .max(self.long_task.max_attempts)
    }
}

fn lookup_usize<F>(lookup: &mut F, name: &str, default: usize) -> usize
where
    F: FnMut(&str) -> Option<String>,
{
    lookup(name)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

fn lookup_u64<F>(lookup: &mut F, name: &str, default: u64) -> u64
where
    F: FnMut(&str) -> Option<String>,
{
    lookup(name)
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default)
}

/// Why the failover loop stopped trying candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BudgetExhaustion {
    AttemptsExhausted {
        attempts: usize,
        max_attempts: usize,
    },
    DeadlineExceeded {
        elapsed_ms: u128,
        deadline_ms: u128,
        over_by_ms: u128,
    },
}

impl BudgetExhaustion {
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

/// Tracks the retry budget for one client request.
///
/// `try_consume_attempt` is intentionally called at the top of each candidate
/// loop by the router, before local Shaper/circuit-breaker checks. It therefore
/// performs only an admission check. The attempt counter is incremented later by
/// `per_attempt_timeout`, exactly when the router is preparing the upstream HTTP
/// request. Local-only skips do not spend an attempt.
#[derive(Debug, Clone)]
pub struct RetryBudgetGuard {
    class: RequestClass,
    policy: RetryPolicy,
    attempts: usize,
    candidate_permitted: bool,
    started: std::time::Instant,
}

impl RetryBudgetGuard {
    pub fn start(class: RequestClass, policy: RetryPolicy) -> Self {
        Self {
            class,
            policy,
            attempts: 0,
            candidate_permitted: false,
            started: std::time::Instant::now(),
        }
    }

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

    pub fn remaining(&self) -> Duration {
        self.policy
            .total_deadline
            .saturating_sub(self.started.elapsed())
    }

    /// Return the timeout for the real upstream request and spend one attempt.
    /// This is called only after local candidate checks have admitted the
    /// candidate, so Shaper/CB skips never decrement the retry budget.
    pub fn per_attempt_timeout(&mut self) -> Duration {
        if self.candidate_permitted && self.attempts < self.policy.max_attempts {
            self.attempts += 1;
        }
        self.candidate_permitted = false;
        self.policy.attempt_timeout_within(self.remaining())
    }

    /// Check whether another candidate may be considered.
    ///
    /// This does not increment the attempt counter. A candidate becomes a real
    /// attempt only when `per_attempt_timeout` is requested immediately before
    /// the HTTP request is built.
    pub fn try_consume_attempt(&mut self) -> Result<(), BudgetExhaustion> {
        // If the previous candidate was skipped locally, discard its pending
        // permission without spending an upstream attempt.
        self.candidate_permitted = false;

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

        self.candidate_permitted = true;
        Ok(())
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    reason = "Test-only module: the assertions are the test."
)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn chat_and_embeddings_are_interactive() {
        for path in [
            "/v1/chat/completions",
            "/v1/embeddings",
            "/v1/rerank",
            "/v1/messages",
        ] {
            assert_eq!(RequestClass::for_path(path), RequestClass::Interactive);
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
            assert_eq!(RequestClass::for_path(path), RequestClass::LongTask);
        }
    }

    #[test]
    fn defaults_keep_interactive_short_and_long_tasks_long() {
        let budget = RetryBudget::from_lookup(|_| None);
        let interactive = budget.policy(RequestClass::Interactive);
        let long_task = budget.policy(RequestClass::LongTask);
        assert_eq!(interactive.max_attempts, 3);
        assert_eq!(interactive.attempt_timeout, Duration::from_secs(120));
        assert_eq!(interactive.total_deadline, Duration::from_secs(300));
        assert_eq!(long_task.max_attempts, 2);
        assert_eq!(long_task.attempt_timeout, Duration::from_secs(36_000));
        assert_eq!(long_task.total_deadline, Duration::from_secs(72_000));
    }

    #[test]
    fn environment_lookup_overrides_the_production_defaults() {
        let values = HashMap::from([
            (ENV_INTERACTIVE_MAX_ATTEMPTS, "7"),
            (ENV_INTERACTIVE_ATTEMPT_TIMEOUT_SECS, "11"),
            (ENV_INTERACTIVE_TOTAL_DEADLINE_SECS, "44"),
            (ENV_LONG_TASK_MAX_ATTEMPTS, "4"),
            (ENV_LONG_TASK_ATTEMPT_TIMEOUT_SECS, "40000"),
            (ENV_LONG_TASK_TOTAL_DEADLINE_SECS, "90000"),
            (ENV_CANDIDATE_LIMIT, "12"),
        ]);
        let budget = RetryBudget::from_lookup(|name| values.get(name).map(|v| (*v).to_string()));
        let interactive = budget.policy(RequestClass::Interactive);
        let long_task = budget.policy(RequestClass::LongTask);
        assert_eq!(interactive.max_attempts, 7);
        assert_eq!(interactive.attempt_timeout, Duration::from_secs(11));
        assert_eq!(interactive.total_deadline, Duration::from_secs(44));
        assert_eq!(long_task.max_attempts, 4);
        assert_eq!(long_task.attempt_timeout, Duration::from_secs(40_000));
        assert_eq!(long_task.total_deadline, Duration::from_secs(90_000));
        assert_eq!(budget.candidate_limit(), 12);
    }

    #[test]
    fn malformed_or_zero_environment_values_fall_back_safely() {
        let values = HashMap::from([
            (ENV_INTERACTIVE_MAX_ATTEMPTS, "0"),
            (ENV_INTERACTIVE_ATTEMPT_TIMEOUT_SECS, "not-a-number"),
        ]);
        let budget = RetryBudget::from_lookup(|name| values.get(name).map(|v| (*v).to_string()));
        let interactive = budget.policy(RequestClass::Interactive);
        assert_eq!(interactive.max_attempts, DEFAULT_INTERACTIVE_MAX_ATTEMPTS);
        assert_eq!(
            interactive.attempt_timeout,
            Duration::from_secs(DEFAULT_INTERACTIVE_ATTEMPT_TIMEOUT_SECS)
        );
    }

    #[test]
    fn locally_skipped_candidates_do_not_spend_attempt_budget() {
        let mut guard = RetryBudgetGuard::start(
            RequestClass::Interactive,
            RetryPolicy {
                max_attempts: 2,
                attempt_timeout: Duration::from_secs(1),
                total_deadline: Duration::from_secs(3600),
            },
        );

        // Candidate 1: Shaper/CB skip — top-of-loop check only.
        assert!(guard.try_consume_attempt().is_ok());
        assert_eq!(guard.attempts(), 0);
        // Candidate 2: another local skip.
        assert!(guard.try_consume_attempt().is_ok());
        assert_eq!(guard.attempts(), 0);
        // Candidate 3 actually reaches HTTP.
        assert!(guard.try_consume_attempt().is_ok());
        let _ = guard.per_attempt_timeout();
        assert_eq!(guard.attempts(), 1);
        // One real attempt remains.
        assert!(guard.try_consume_attempt().is_ok());
        let _ = guard.per_attempt_timeout();
        assert_eq!(guard.attempts(), 2);
        assert!(guard.try_consume_attempt().is_err());
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
        for _ in 0..2 {
            assert!(guard.try_consume_attempt().is_ok());
            let _ = guard.per_attempt_timeout();
        }
        let exhausted = guard.try_consume_attempt().expect_err("third is refused");
        assert_eq!(
            exhausted,
            BudgetExhaustion::AttemptsExhausted {
                attempts: 2,
                max_attempts: 2
            }
        );
        assert!(exhausted.reason().contains("2 of 2 attempts"));
    }

    #[test]
    fn an_expired_deadline_stops_the_request_even_with_attempts_left() {
        let mut guard = RetryBudgetGuard::start(
            RequestClass::Interactive,
            RetryPolicy {
                max_attempts: 5,
                attempt_timeout: Duration::from_secs(1),
                total_deadline: Duration::ZERO,
            },
        );
        assert!(matches!(
            guard.try_consume_attempt(),
            Err(BudgetExhaustion::DeadlineExceeded { .. })
        ));
        assert_eq!(guard.attempts(), 0);
    }

    #[test]
    fn the_per_attempt_timeout_shrinks_to_fit_the_remaining_budget() {
        let mut guard = RetryBudgetGuard::start(
            RequestClass::Interactive,
            RetryPolicy {
                max_attempts: 3,
                attempt_timeout: Duration::from_secs(3600),
                total_deadline: Duration::from_secs(10),
            },
        );
        guard.try_consume_attempt().expect("candidate is permitted");
        let timeout = guard.per_attempt_timeout();
        assert!(timeout <= Duration::from_secs(10));
        assert_eq!(guard.attempts(), 1);
    }

    #[test]
    fn the_candidate_limit_covers_every_attempt_cap() {
        let budget = RetryBudget::from_lookup(|_| None);
        for class in [RequestClass::Interactive, RequestClass::LongTask] {
            assert!(budget.candidate_limit() >= budget.policy(class).max_attempts);
        }
        let generous = RetryBudget::new(
            RetryPolicy {
                max_attempts: 9,
                attempt_timeout: Duration::from_secs(1),
                total_deadline: Duration::from_secs(10),
            },
            budget.policy(RequestClass::LongTask),
        );
        assert_eq!(generous.candidate_limit(), 9);
    }

    #[test]
    fn the_old_worst_case_is_no_longer_reachable_by_default() {
        const OLD_WORST_CASE_SECS: u64 = 36_000 * 5;
        let budget = RetryBudget::from_lookup(|_| None);
        for class in [RequestClass::Interactive, RequestClass::LongTask] {
            let policy = budget.policy(class);
            assert!(policy.total_deadline.as_secs() < OLD_WORST_CASE_SECS);
        }
    }
}
