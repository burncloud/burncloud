#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Retry-budget contract for #680.
//!
//! Before the fix, one request could spend five candidates at the shared
//! ten-hour timeout with no total deadline. The contract now distinguishes
//! interactive requests from long tasks, makes the limits operator-configurable,
//! and counts only candidates that actually reach an upstream HTTP request.

use burncloud_router::retry::{
    RequestClass, RetryBudget, RetryBudgetGuard, RetryPolicy, DEFAULT_CANDIDATE_LIMIT,
};
use std::time::Duration;

const SHARED_CLIENT_TIMEOUT_SECS: u64 = 36_000;

fn source(path: &str) -> String {
    let full_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    std::fs::read_to_string(&full_path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", full_path.display()))
}

#[test]
fn candidate_limit_is_named_and_not_take_five() {
    let src = source("src/model_router.rs");
    let line = src
        .lines()
        .find(|line| line.contains(".take(candidate_limit)"))
        .expect("candidate limit must come from the retry budget");
    assert!(!line.contains("take(5)"));

    let budget = RetryBudget::default();
    assert!(budget.candidate_limit() >= DEFAULT_CANDIDATE_LIMIT);
    for class in [RequestClass::Interactive, RequestClass::LongTask] {
        assert!(budget.candidate_limit() >= budget.policy(class).max_attempts);
    }
}

#[test]
fn production_budget_has_real_configuration_inputs() {
    let src = source("src/retry.rs");
    for name in [
        "BURNCLOUD_RETRY_INTERACTIVE_MAX_ATTEMPTS",
        "BURNCLOUD_RETRY_INTERACTIVE_ATTEMPT_TIMEOUT_SECS",
        "BURNCLOUD_RETRY_INTERACTIVE_TOTAL_DEADLINE_SECS",
        "BURNCLOUD_RETRY_LONG_TASK_MAX_ATTEMPTS",
        "BURNCLOUD_RETRY_LONG_TASK_ATTEMPT_TIMEOUT_SECS",
        "BURNCLOUD_RETRY_LONG_TASK_TOTAL_DEADLINE_SECS",
        "BURNCLOUD_RETRY_CANDIDATE_LIMIT",
    ] {
        assert!(
            src.contains(name),
            "{name} must be a production configuration input, not only a test constructor"
        );
    }
    assert!(
        src.contains("std::env::var"),
        "RetryBudget::default must be wired to operator configuration"
    );
}

#[test]
fn explicit_policy_is_enforced() {
    let custom = RetryBudget::new(
        RetryPolicy {
            max_attempts: 2,
            attempt_timeout: Duration::from_secs(5),
            total_deadline: Duration::from_secs(35),
        },
        RetryPolicy {
            max_attempts: 1,
            attempt_timeout: Duration::from_secs(99),
            total_deadline: Duration::from_secs(99),
        },
    );
    let policy = custom.policy(RequestClass::Interactive);
    let mut guard = RetryBudgetGuard::start(RequestClass::Interactive, policy);

    for expected in 1..=2 {
        guard
            .try_consume_attempt()
            .unwrap_or_else(|e| panic!("attempt {expected} must be admitted: {e:?}"));
        let timeout = guard.per_attempt_timeout();
        assert!(timeout <= Duration::from_secs(5));
        assert_eq!(guard.attempts(), expected);
    }
    let exhausted = guard.try_consume_attempt().expect_err("third is refused");
    assert!(exhausted.reason().contains("2 of 2 attempts"));
}

#[test]
fn local_skips_do_not_consume_upstream_attempts() {
    let mut guard = RetryBudgetGuard::start(
        RequestClass::Interactive,
        RetryPolicy {
            max_attempts: 1,
            attempt_timeout: Duration::from_secs(5),
            total_deadline: Duration::from_secs(60),
        },
    );

    // These model Shaper/CB skips: the router calls the top-of-loop admission
    // check but never asks for an HTTP timeout because no HTTP request is built.
    for _ in 0..4 {
        guard
            .try_consume_attempt()
            .expect("a locally skipped candidate must not spend the one real attempt");
        assert_eq!(guard.attempts(), 0);
    }

    guard
        .try_consume_attempt()
        .expect("the healthy candidate still gets the real attempt");
    let _ = guard.per_attempt_timeout();
    assert_eq!(guard.attempts(), 1);
    assert!(guard.try_consume_attempt().is_err());
}

#[test]
fn long_tasks_keep_the_ten_hour_attempt_window_by_default() {
    let long = RetryBudget::default().policy(RequestClass::LongTask);
    assert_eq!(
        long.attempt_timeout,
        Duration::from_secs(SHARED_CLIENT_TIMEOUT_SECS),
        "video/audio/music must keep the deliberate ten-hour per-attempt default"
    );
    assert!(long.total_deadline >= long.attempt_timeout);
}

#[test]
fn interactive_requests_are_bounded_in_minutes_by_default() {
    let interactive = RetryBudget::default().policy(RequestClass::Interactive);
    assert!(interactive.attempt_timeout <= Duration::from_secs(300));
    assert!(interactive.total_deadline <= Duration::from_secs(600));
}

#[test]
fn path_classification_keeps_media_long_running() {
    for path in [
        "/v1/chat/completions",
        "/v1/completions",
        "/v1/embeddings",
        "/v1/messages",
    ] {
        assert_eq!(RequestClass::for_path(path), RequestClass::Interactive);
    }
    for path in [
        "/v1/video/generations",
        "/v1/videos/task-abc",
        "/v1/audio/transcriptions",
        "/v1/music/generations",
        "/v1/speech",
    ] {
        assert_eq!(RequestClass::for_path(path), RequestClass::LongTask);
    }
}

#[test]
fn deadline_refuses_work_without_spending_an_attempt() {
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
        .expect_err("a spent deadline must refuse the candidate");
    assert!(exhausted.reason().contains("deadline"));
    assert_eq!(guard.attempts(), 0);
}

#[test]
fn one_real_attempt_cannot_outlive_the_remaining_deadline() {
    let mut guard = RetryBudgetGuard::start(
        RequestClass::Interactive,
        RetryPolicy {
            max_attempts: 3,
            attempt_timeout: Duration::from_secs(SHARED_CLIENT_TIMEOUT_SECS),
            total_deadline: Duration::from_secs(30),
        },
    );
    guard
        .try_consume_attempt()
        .expect("candidate is admitted before HTTP");
    let timeout = guard.per_attempt_timeout();
    assert!(timeout <= Duration::from_secs(30));
    assert_eq!(guard.attempts(), 1);
}

#[test]
fn shared_client_timeout_remains_ten_hours() {
    let src = source("src/lib.rs");
    let line = src
        .lines()
        .find(|line| line.contains("const HTTP_REQUEST_TIMEOUT_SECS:"))
        .expect("HTTP_REQUEST_TIMEOUT_SECS must still exist");
    let value: u64 = line
        .split('=')
        .nth(1)
        .and_then(|value| value.trim().trim_end_matches(';').parse().ok())
        .unwrap_or_else(|| panic!("could not parse shared timeout from {line}"));
    assert_eq!(value, SHARED_CLIENT_TIMEOUT_SECS);
}

#[test]
fn default_worst_cases_are_bounded_below_the_old_fifty_hours() {
    const OLD_WORST_CASE_SECS: u64 = SHARED_CLIENT_TIMEOUT_SECS * 5;
    let budget = RetryBudget::default();
    for class in [RequestClass::Interactive, RequestClass::LongTask] {
        let policy = budget.policy(class);
        let bound = (policy.attempt_timeout.as_secs() * policy.max_attempts as u64)
            .min(policy.total_deadline.as_secs());
        assert!(bound < OLD_WORST_CASE_SECS);
    }
}
