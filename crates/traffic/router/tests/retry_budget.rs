#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! The effective retry budget of the failover loop (#633, plan section 5 item 12).
//!
//! The plan lists "retry budget" among the behaviours to cover for this crate. There is **no such concept in
//! the code**: a search for `retry_budget`, `max_retries`, `attempt_budget`, `retries_left` and similar across
//! `crates/traffic` returns nothing, and neither `config.rs` nor `state.rs` has a retry setting. What the
//! failover loop has instead is a count derived from two places, and this file pins both so the effective
//! budget is a documented number rather than an emergent one:
//!
//! ```text
//! model_router.rs:327   ranked.into_iter().take(5)     -> at most 5 candidates, hard-coded
//! lib.rs:2429           for (attempt, upstream) in candidates.iter().enumerate()
//!                                                        -> one attempt per candidate, no repeats
//! ```
//!
//! So the budget is **five attempts**, it is not configurable, and it is the same constant that the module
//! documentation calls the "top-5 failover list".
//!
//! ## Why the number matters
//!
//! Each attempt runs under the shared HTTP client's timeout, `HTTP_REQUEST_TIMEOUT_SECS = 36000`
//! (`lib.rs:205`), whose own comment says "Total time for request completion (600 minutes)" -- ten hours, which
//! is deliberate for long-running work such as video generation. Nothing bounds the **total**: `elapsed()` is
//! only ever read to record a latency (`lib.rs:1883`, `:1899`, `:2647`, `:3296`), never compared against a
//! deadline.
//!
//! The worst case for one client request is therefore five attempts of up to ten hours each. That is the
//! reason a retry budget exists as a concept, and it is filed separately rather than asserted here -- a test
//! cannot demonstrate a ten-hour timeout without waiting ten hours.
//!
//! What this file can do is pin the parts that are structural: how many candidates the router produces, that
//! the loop cannot revisit one, and that the cap is a literal rather than a setting.

/// The candidate cap is written as a literal in the router.
///
/// This test reads the source rather than calling a function, because there is no function: the number is not
/// exposed, not configurable, and `grep` is the only way to see it. That is itself the finding -- a budget
/// that only exists as a literal in one expression cannot be adjusted or asserted from outside.
#[test]
fn the_candidate_cap_is_a_hard_coded_literal_of_five() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/model_router.rs"))
        .expect("model_router.rs is part of this crate");

    let cap_line = src
        .lines()
        .find(|l| l.contains("into_iter().take("))
        .expect(
            "the candidate cap expression was not found; if it moved, this test needs updating",
        );

    println!("candidate cap: {}", cap_line.trim());

    assert!(
        cap_line.contains("take(5)"),
        "the cap is expected to be five; the line now reads: {}",
        cap_line.trim()
    );

    // No configuration path exists for it, so a change here is a code change rather than a setting.
    for file in ["config.rs", "state.rs"] {
        let path = format!("{}/src/{file}", env!("CARGO_MANIFEST_DIR"));
        let text = std::fs::read_to_string(&path).expect("the file exists");
        let mentions_retrying = text.lines().any(|l| {
            let code = l.split("//").next().unwrap_or("");
            let lower = code.to_ascii_lowercase();
            lower.contains("retry")
                || lower.contains("max_attempts")
                || lower.contains("attempt_budget")
        });
        assert!(
            !mentions_retrying,
            "{file} now mentions retrying in code, which means the budget has become configurable and this \
             test's premise (a hard-coded literal) no longer holds"
        );
    }
}

/// Every attempt in the failover loop is a distinct candidate.
///
/// The loop is `for (attempt, upstream) in candidates.iter().enumerate()`, and each `continue` advances the
/// iterator, so no candidate is retried within a request. Combined with the cap of five, the budget is exactly
/// `min(candidates.len(), 5)` attempts.
///
/// Asserted against the source because the loop lives inside a 4651-line function that needs a database, a
/// scheduler and a mock upstream to reach -- the structural property is checkable without any of that, and the
/// end-to-end behaviour is already covered by the failover tests in `shaper_tests.rs`.
#[test]
fn the_loop_takes_each_candidate_once_and_never_retries_one() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is part of this crate");

    let loop_line = src
        .lines()
        .find(|l| l.contains("for (attempt, upstream) in candidates.iter().enumerate()"))
        .expect("the failover loop header was not found; if it moved, this test needs updating");

    println!("failover loop: {}", loop_line.trim());

    // The loop iterates the candidate vector directly. If it ever becomes a `loop { }` with a manual index, or
    // wraps the candidates in a retry iterator, the one-attempt-per-candidate property stops being structural.
    assert!(
        loop_line.contains("candidates.iter()"),
        "the loop must iterate the candidate vector directly so that each candidate is attempted once"
    );
    assert!(
        !loop_line.contains("cycle") && !loop_line.contains("repeat"),
        "an iterator that revisits its items would make the budget unbounded"
    );
}

/// The budget is `min(candidates, 5)`, written where the reader will look for it.
///
/// This is an arithmetic statement rather than a test of code, and it is here because the two facts above are
/// in two different files and neither says the number "5" is a retry budget. A reader looking for "retry
/// budget" finds nothing; a reader looking for `take(5)` finds a routing detail.
#[test]
fn the_effective_budget_is_the_candidate_count_capped_at_five() {
    for (candidates, expected_attempts) in
        [(0usize, 0usize), (1, 1), (3, 3), (5, 5), (8, 5), (100, 5)]
    {
        let budget = candidates.min(5);
        println!("candidates={candidates} -> attempts={budget}");
        assert_eq!(
            budget, expected_attempts,
            "the budget is min(candidates, 5); if this changes, the cap moved"
        );
    }

    // The consequence, stated where a reader will find it rather than left to be derived.
    let http_timeout_secs: u64 = 36000;
    let worst_case = http_timeout_secs * 5;
    println!(
        "worst case for one client request: {worst_case} seconds ({} hours) across five attempts, with no \
         overall deadline",
        worst_case / 3600
    );
    assert_eq!(
        worst_case / 3600,
        50,
        "five attempts of the configured HTTP timeout is fifty hours; if the timeout changed, this is a \
         reminder that nothing bounds the total"
    );
}

/// The configured timeouts, so a change to either is visible rather than silent.
#[test]
fn the_timeouts_that_multiply_into_the_worst_case_are_pinned() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("lib.rs is part of this crate");

    let find = |name: &str| -> u64 {
        let line = src
            .lines()
            .find(|l| l.contains(&format!("const {name}:")))
            .unwrap_or_else(|| panic!("{name} was not found"));
        line.split('=')
            .nth(1)
            .and_then(|v| v.trim().trim_end_matches(';').trim().parse::<u64>().ok())
            .unwrap_or_else(|| panic!("could not parse the value of {name} from: {}", line.trim()))
    };

    let connect = find("HTTP_CONNECT_TIMEOUT_SECS");
    let request = find("HTTP_REQUEST_TIMEOUT_SECS");
    println!("connect timeout = {connect}s, request timeout = {request}s");

    assert_eq!(
        connect, 30,
        "the connect timeout is what bounds a dead host"
    );
    assert_eq!(
        request, 36000,
        "the request timeout is ten hours, which the constant's own comment says is deliberate for \
         long-running work. It applies to every request through the shared client, including a chat \
         completion, and it multiplies by the attempt count"
    );
}
