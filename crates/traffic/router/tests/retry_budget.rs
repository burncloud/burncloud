#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! The retry budget of the failover loop (#633 plan section 5 item 12, #680).
//!
//! ## What this file used to measure
//!
//! Before #680 there was no retry budget in the code at all. The effective policy was emergent from two
//! unrelated facts:
//!
//! ```text
//! model_router.rs      ranked.into_iter().take(5)                       -> at most 5 candidates, a literal
//! lib.rs               for (attempt, upstream) in candidates.iter()     -> one attempt per candidate
//! lib.rs               HTTP_REQUEST_TIMEOUT_SECS = 36000                -> 10 h per attempt, on a shared client
//! ```
//!
//! Fifty hours for one client request, with nothing comparing elapsed time to a deadline — and for that whole
//! time the request's `BudgetGuard` held the channel's TPM reservation.
//!
//! ## What it asserts now
//!
//! The budget is a named, per-class policy in `src/retry.rs`:
//!
//! | class | attempts | per attempt | total deadline |
//! | --- | --- | --- | --- |
//! | interactive (chat, embeddings) | 3 | 120 s | 300 s |
//! | long task (video/audio/music) | 2 | 10 h | 20 h |
//!
//! The long-task per-attempt timeout is **unchanged** from the shared client's ten hours, because that value is
//! load-bearing for media generation; only the total is newly bounded. Both halves are asserted below, along
//! with the fact that the candidate cap now follows the attempt cap instead of being a separate literal.

use burncloud_router::retry::{
    RequestClass, RetryBudget, RetryBudgetGuard, RetryPolicy, DEFAULT_CANDIDATE_LIMIT,
};
use std::time::Duration;

/// The old per-attempt timeout, from `lib.rs::HTTP_REQUEST_TIMEOUT_SECS`.
const SHARED_CLIENT_TIMEOUT_SECS: u64 = 36_000;

fn candidate_cap_line() -> String {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/model_router.rs"))
        .expect("model_router.rs is part of this crate");
    src.lines()
        .find(|l| l.contains(".take(candidate_limit)"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| {
            panic!(
                "the candidate cap expression was not found; if it moved, this test needs updating"
            )
        })
}

// -------------------------------------------------------------------------------------------
// the budget is a named, configurable policy
// -------------------------------------------------------------------------------------------

#[test]
fn the_candidate_cap_is_no_longer_a_hard_coded_literal() {
    // The first half of the old budget. The number is now named and reachable, and the router's candidate list
    // can never be shorter than the budget is allowed to spend.
    let cap = candidate_cap_line();
    println!("candidate cap expression: {cap}");
    assert!(
        cap.contains("candidate_limit"),
        "the cap must come from the retry budget, not a literal; the line reads: {cap}"
    );
    assert!(
        !cap.contains("take(5)"),
        "the literal `take(5)` is what made the budget unreadable and unconfigurable"
    );

    let budget = RetryBudget::default();
    assert_eq!(
        budget.candidate_limit(),
        DEFAULT_CANDIDATE_LIMIT,
        "the router's ranked-fallback contract is five candidates"
    );
    for class in [RequestClass::Interactive, RequestClass::LongTask] {
        assert!(
            budget.candidate_limit() >= budget.policy(class).max_attempts,
            "{} allows {} attempts, so at least that many candidates must exist",
            class.as_label(),
            budget.policy(class).max_attempts
        );
    }

    // And it is a setting, not a constant: a caller can widen it.
    assert_eq!(
        RetryBudget::default()
            .with_candidate_limit(12)
            .candidate_limit(),
        12
    );
}

#[test]
fn the_bounds_are_settings_rather_than_literals() {
    // Acceptance criterion 1: the upper bound is configurable. A caller can build a budget with any policy and
    // the guard obeys it, which is what "configurable" has to mean.
    let custom = RetryBudget::new(
        RetryPolicy {
            max_attempts: 7,
            attempt_timeout: Duration::from_secs(5),
            total_deadline: Duration::from_secs(35),
        },
        RetryPolicy {
            max_attempts: 1,
            attempt_timeout: Duration::from_secs(99),
            total_deadline: Duration::from_secs(99),
        },
    );

    let interactive = custom.policy(RequestClass::Interactive);
    assert_eq!(interactive.max_attempts, 7);
    assert_eq!(interactive.attempt_timeout, Duration::from_secs(5));
    assert_eq!(interactive.total_deadline, Duration::from_secs(35));

    let long = custom.policy(RequestClass::LongTask);
    assert_eq!(long.max_attempts, 1);

    // The custom policy is what the guard actually enforces, not just what it reports.
    let mut guard = RetryBudgetGuard::start(RequestClass::Interactive, interactive);
    for expected in 1..=7 {
        assert!(guard.try_consume_attempt().is_ok(), "attempt {expected}");
    }
    assert!(
        guard.try_consume_attempt().is_err(),
        "the eighth attempt must be refused by the configured cap of seven"
    );
}

#[test]
fn the_budget_is_reachable_by_the_name_a_reader_would_search_for() {
    // Acceptance criterion 5: "retry budget" must be findable. Before #680 a search for the phrase found
    // nothing and the number lived in a `take(5)`.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is part of this crate");
    assert!(
        src.contains("retry::RetryBudgetGuard") && src.contains("retry_guard"),
        "the failover loop must hold a retry guard, so the budget is enforced rather than documented"
    );

    let budget_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/retry.rs"))
        .expect("retry.rs is part of this crate");
    for name in ["max_attempts", "attempt_timeout", "total_deadline"] {
        assert!(
            budget_src.contains(name),
            "{name} must be a named part of the budget, not an implicit consequence of two other files"
        );
    }
    // And `config.rs` / `state.rs` now mention it, which the old baseline test asserted they did not.
    let state_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/state.rs"))
        .expect("state.rs is part of this crate");
    assert!(
        state_src.contains("retry_budget"),
        "the budget must live on AppState so it can be configured at boot"
    );
}

// -------------------------------------------------------------------------------------------
// the two classes, and why they differ
// -------------------------------------------------------------------------------------------

#[test]
fn a_long_task_keeps_the_timeout_that_the_shared_client_documents() {
    // Acceptance criterion 2: video generation must still work. The ten-hour value is deliberate, and this is
    // the assertion that would fail if someone "fixed" the fifty-hour worst case by shortening it.
    let long = RetryBudget::default().policy(RequestClass::LongTask);
    assert_eq!(
        long.attempt_timeout,
        Duration::from_secs(SHARED_CLIENT_TIMEOUT_SECS),
        "the long-task per-attempt timeout must remain the ten hours HTTP_REQUEST_TIMEOUT_SECS documents"
    );

    // Concretely: an attempt budget generous enough for the video task polling timeout.
    let video_task_poll_timeout = Duration::from_secs(600);
    assert!(
        long.attempt_timeout > video_task_poll_timeout * 10,
        "a video task's own polling timeout is 600 s; the attempt timeout must stay far above it so a \
         legitimate video request is never cut off"
    );
}

#[test]
fn an_interactive_request_is_bounded_in_seconds_not_hours() {
    let interactive = RetryBudget::default().policy(RequestClass::Interactive);
    assert!(
        interactive.attempt_timeout <= Duration::from_secs(300),
        "a chat completion must not be able to occupy a connection for hours"
    );
    assert!(
        interactive.total_deadline <= Duration::from_secs(600),
        "and the whole interactive request must finish within minutes"
    );

    // The two classes are not variations of one number.
    let long = RetryBudget::default().policy(RequestClass::LongTask);
    assert!(
        long.total_deadline > interactive.total_deadline * 50,
        "the classes must be separated by orders of magnitude, or the split is cosmetic"
    );
}

#[test]
fn the_path_decides_the_class() {
    for path in [
        "/v1/chat/completions",
        "/v1/completions",
        "/v1/embeddings",
        "/v1/messages",
    ] {
        assert_eq!(
            RequestClass::for_path(path),
            RequestClass::Interactive,
            "{path} is a request a user is waiting on"
        );
    }
    for path in [
        "/v1/video/generations",
        "/v1/videos/task-abc",
        "/v1/audio/transcriptions",
        "/v1/music/generations",
        "/v1/speech",
    ] {
        assert_eq!(
            RequestClass::for_path(path),
            RequestClass::LongTask,
            "{path} is a long-running operation"
        );
    }
}

// -------------------------------------------------------------------------------------------
// the deadline, which is the part that did not exist
// -------------------------------------------------------------------------------------------

#[test]
fn the_deadline_ends_the_request_even_with_attempts_left() {
    // Acceptance criterion 3: when the deadline passes the request fails clearly, rather than continuing to the
    // next candidate. This is the property the old code had no way to express — it had no deadline at all.
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
        .expect_err("a spent deadline must refuse the attempt");
    let reason = exhausted.reason();
    println!("deadline exhaustion reason: {reason}");
    assert!(
        reason.contains("deadline"),
        "the reason must name the deadline so the 502 can distinguish it from an upstream failure: {reason}"
    );
    assert_eq!(
        guard.attempts(),
        0,
        "an attempt that was never made must not be counted, or the attempt budget would leak"
    );
}

#[test]
fn the_attempt_cap_is_reported_with_both_numbers() {
    let mut guard = RetryBudgetGuard::start(
        RequestClass::LongTask,
        RetryPolicy {
            max_attempts: 2,
            attempt_timeout: Duration::from_secs(10),
            total_deadline: Duration::from_secs(600),
        },
    );
    guard.try_consume_attempt().expect("first");
    guard.try_consume_attempt().expect("second");
    let exhausted = guard.try_consume_attempt().expect_err("third is refused");

    let reason = exhausted.reason();
    println!("attempt exhaustion reason: {reason}");
    assert!(
        reason.contains("2 of 2"),
        "the reason must say how many attempts were used against the cap: {reason}"
    );
}

#[test]
fn one_attempt_can_never_outlive_the_total_deadline() {
    // The mechanism that makes the deadline load-bearing rather than decorative: the per-attempt timeout is
    // clamped to what is left, so an attempt cannot start with a ten-hour budget and a one-second deadline.
    let guard = RetryBudgetGuard::start(
        RequestClass::Interactive,
        RetryPolicy {
            max_attempts: 3,
            attempt_timeout: Duration::from_secs(SHARED_CLIENT_TIMEOUT_SECS),
            total_deadline: Duration::from_secs(30),
        },
    );

    let timeout = guard.per_attempt_timeout();
    println!("clamped attempt timeout: {timeout:?}");
    assert!(
        timeout <= Duration::from_secs(30),
        "the attempt timeout must shrink to the remaining deadline, got {timeout:?}"
    );
}

// -------------------------------------------------------------------------------------------
// the numbers, fixed rather than derived
// -------------------------------------------------------------------------------------------

#[test]
fn the_worst_cases_are_the_fixed_ones() {
    // Acceptance criterion 7 wants the before/after figures recorded. They are printed here so a run shows
    // them, and asserted so a silent change is caught.
    let budget = RetryBudget::default();

    let interactive = budget.policy(RequestClass::Interactive);
    let long = budget.policy(RequestClass::LongTask);

    let old_worst_case = SHARED_CLIENT_TIMEOUT_SECS * 5;
    println!(
        "BEFORE: 5 attempts x {SHARED_CLIENT_TIMEOUT_SECS} s = {old_worst_case} s ({} h), no overall deadline",
        old_worst_case / 3600
    );

    for class in [RequestClass::Interactive, RequestClass::LongTask] {
        let policy = budget.policy(class);
        let worst_case_secs = policy.attempt_timeout.as_secs() * policy.max_attempts as u64;
        let bounded_secs = worst_case_secs.min(policy.total_deadline.as_secs());
        println!(
            "AFTER ({}) : {} attempts x {} s = {} s worst case, deadline {} s -> bound {} s ({} h)",
            class.as_label(),
            policy.max_attempts,
            policy.attempt_timeout.as_secs(),
            worst_case_secs,
            policy.total_deadline.as_secs(),
            bounded_secs,
            bounded_secs / 3600
        );
        assert!(
            bounded_secs < old_worst_case,
            "{} must be strictly better than the old unbounded worst case",
            class.as_label()
        );
    }

    assert_eq!(
        interactive.attempt_timeout.as_secs() * interactive.max_attempts as u64,
        360,
        "the interactive worst case is six minutes"
    );
    assert_eq!(
        long.total_deadline.as_secs(),
        72_000,
        "the long-task total is twenty hours, down from the fifty-hour worst case"
    );
}

#[test]
fn the_shared_client_timeout_is_still_the_long_task_value() {
    // The per-attempt override is applied on top of the shared client rather than replacing it, so this
    // constant must not have been narrowed: a request class that is not clamped still inherits it.
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is part of this crate");

    let line = src
        .lines()
        .find(|l| l.contains("const HTTP_REQUEST_TIMEOUT_SECS:"))
        .expect("HTTP_REQUEST_TIMEOUT_SECS must still exist");
    let value: u64 = line
        .split('=')
        .nth(1)
        .and_then(|v| v.trim().trim_end_matches(';').trim().parse().ok())
        .unwrap_or_else(|| panic!("could not parse the value from: {}", line.trim()));

    assert_eq!(
        value, SHARED_CLIENT_TIMEOUT_SECS,
        "the shared client timeout is what the long-task class inherits and video generation depends on"
    );
}
