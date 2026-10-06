#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only file: the assertions are the test."
)]
//! Every failover path must name why it skipped a candidate (#662).
//!
//! When the failover loop runs out of candidates the client receives
//!
//! ```json
//! {"error":{"message":"All upstreams failed. Last error: ","type":"upstream_error",
//!           "code":"all_upstreams_failed"}}
//! ```
//!
//! and `Last error: ` was empty — no category, no channel, no clue — on the one
//! response that most needs a clue. The structure that produced it: the loop had
//! 22 `last_error` assignments against 24 `continue` statements, and the paths
//! that skipped a candidate without recording anything were the ones whose
//! sibling paths *did* record (the circuit-breaker skip set a reason, the L2
//! Shaper skip did not).
//!
//! This file pins the invariant in two ways:
//!
//! 1. **Statically**, over the loop body read from `src/lib.rs`: every
//!    `continue` that advances to the next *candidate* must be preceded by a
//!    `last_error` assignment. `continue` statements inside an inner loop (SSE
//!    line parsing) are excluded, and so are the ones that leave the loop with a
//!    real upstream status — those return the upstream's own body, not the
//!    generic 502, so they never reach the empty-message path.
//! 2. **Behaviourally**, on the reason text itself: each rejected-response
//!    category must describe itself rather than rendering as a placeholder.

use std::fs;
use std::path::PathBuf;

fn lib_rs() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The failover loop body: from `for (attempt, upstream) in candidates` to the
/// end of that function, where the after-loop branches live.
fn failover_loop_body(src: &str) -> &str {
    let start = src
        .find("for (attempt, upstream) in candidates.iter().enumerate()")
        .expect("the failover loop over candidates must exist");
    // The after-loop branches are the first thing that follows the loop; taking
    // the rest of the file keeps them while giving the scan a definite end.
    &src[start..]
}

/// True when this `continue` is inside a nested loop rather than the failover loop.
///
/// The scanner is deliberately simple: a `continue` that appears after an inner
/// `for ` line which has not yet closed is not a candidate skip. The two real
/// inner loops here both iterate `text.lines()`.
fn is_inner_loop_continue(body_before_continue: &str) -> bool {
    let loop_body_start = body_before_continue.rfind("for line in text.lines()");
    let Some(loop_body_start) = loop_body_start else {
        return false;
    };
    // Count braces after the inner `for` opened; if the inner loop is still open
    // at the `continue`, the `continue` belongs to it.
    let mut depth: i32 = 0;
    for ch in body_before_continue[loop_body_start..].chars() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            _ => {}
        }
    }
    // Depth 0 means the inner `for`'s braces are balanced again, so this
    // `continue` is not inside it.
    depth > 0
}

#[test]
fn every_candidate_skip_records_a_reason() {
    let src = lib_rs();
    let body = failover_loop_body(&src);

    let mut candidate_continues = 0usize;
    let mut without_reason: Vec<String> = Vec::new();
    let mut inner_loop_continues = 0usize;

    for (idx, _) in body.match_indices("continue;") {
        let before = &body[..idx];
        if is_inner_loop_continue(before) {
            inner_loop_continues += 1;
            continue;
        }
        candidate_continues += 1;

        // The reason must be set in the same statement run as the `continue`.
        //
        // A window of the preceding 1600 characters covers the longest path here — a `last_error = format!` with
        // its arguments, a `tracing::warn!`, and a `record_failover_attempt` call — while still being local to
        // this `continue`. It was 900 until the quality-check path grew a multi-line `format!`, which is exactly
        // the kind of drift this window has to absorb without either missing a site or reaching into the
        // previous iteration.
        let window_start = before.len().saturating_sub(1600);
        let window = &before[window_start..];
        if !window.contains("last_error =") {
            let line = body[..idx].lines().count();
            without_reason.push(format!("line {line} of the loop body"));
        }
    }

    assert_eq!(
        inner_loop_continues, 6,
        "the loop has exactly six `continue`s inside its three SSE line-parsing loops (two each); \
         {inner_loop_continues} were detected, so the inner-loop exclusion is mis-classifying candidate skips"
    );
    assert_eq!(
        candidate_continues, 18,
        "the failover loop has 18 `continue`s that move to the next candidate (24 in total, minus the six \
         inside its three SSE line-parsing loops); {candidate_continues} were detected. If a candidate path \
         was added or removed, update this number deliberately — it is the denominator for the assertion below"
    );
    assert!(
        without_reason.is_empty(),
        "these candidate skips `continue` without setting `last_error`, so a request where every candidate \
         fails that way returns `All upstreams failed. Last error: ` with nothing after the colon: {without_reason:#?}"
    );
}

#[test]
fn the_shaper_skip_is_not_the_only_silent_one() {
    // The specific asymmetry #662 named: the circuit-breaker skip recorded a
    // reason and its sibling, the L2 Shaper rejection, did not. Restated as its
    // own case because it is the one that is easy to reintroduce.
    let src = lib_rs();
    let body = failover_loop_body(&src);

    for marker in ["L2 Shaper rejected", "Circuit Breaker Open"] {
        assert!(
            body.contains(marker),
            "the loop must still have a {marker:?} path; if it was renamed, update this test rather than \
             deleting it"
        );
    }

    let shaper_skip = body
        .find("L2 Shaper rejected: {}")
        .expect("the L2 Shaper skip must name the channel it rejected");
    let window_start = shaper_skip.saturating_sub(600);
    assert!(
        body[window_start..shaper_skip].contains("last_error ="),
        "the L2 Shaper skip must assign `last_error` before `continue`, exactly as the circuit-breaker skip does"
    );
}

#[test]
fn the_all_upstreams_failed_body_carries_the_last_error() {
    // The wire contract, read from the source so a rename of the field is caught:
    // `code` stays `all_upstreams_failed` and the message interpolates the reason.
    let src = lib_rs();
    // Search for the response format string, not the phrase: the phrase also appears in the comment that
    // explains why this path used to be empty.
    let start = src
        .find(r#"All upstreams failed. Last error: {}"#)
        .expect("the all-upstreams-failed response must exist");
    let window = &src[start.saturating_sub(40)..(start + 220).min(src.len())];

    assert!(
        window.contains("all_upstreams_failed"),
        "the client-visible `code` is part of the contract and must not change"
    );
    assert!(
        window.contains("upstream_error"),
        "and neither must the `type`"
    );
}
