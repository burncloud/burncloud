#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only integration file: unwrap/expect is the intended failure signal, and the \
              assertions compare content the logger actually wrote rather than trusting it to have \
              written anything."
)]
//! The structured-log contract of `burncloud_loops::log` (#635, scope
//! `loops-run-lock-and-structured-log-contract`).
//!
//! `LoopLogger` is the only thing standing between a run and any record of what it did: gates report
//! through `gate_start` / `gate_end` / `gate_output`, and `timed_gate` wraps them. None of that had
//! a test, so "the log records the run" was an assumption.
//!
//! Everything here writes to a temporary file and asserts on what can be **read back**. Nothing
//! starts a real gate: `run_css_all`, `run_aesthetic_full`, `run_jobs_fast_gates`, `run_single_gate`
//! and `run_gate_suite` execute cargo, browsers and other processes, and this scope forbids them.
//! The one closure handed to `timed_gate` below is a pure in-process value, which is what makes the
//! timing/aggregation contract testable without running anything.

use burncloud_loops::gates::GateCategory;
use burncloud_loops::log::LoopLogger;
use std::path::{Path, PathBuf};

/// A unique directory under the temporary directory, removed by the owning guard.
struct Scratch {
    dir: PathBuf,
}

impl Scratch {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "bc_loops_log_{}_{}_{}",
            tag,
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&dir).expect("the scratch directory must be creatable");
        Self { dir }
    }

    fn file(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("the logger must have written {}: {e}", path.display()))
}

/// `[<ISO-8601 UTC>] [iter <n>] [<category>] <LEVEL> <rest>`.
///
/// Checked as a shape rather than with a regex crate: the point is that the stamp has the fields
/// the log is parsed by, and a hand-written check keeps the assertion readable.
fn assert_stamp(line: &str, iteration: u32, category: &str) {
    let rest = line
        .strip_prefix('[')
        .unwrap_or_else(|| panic!("a log line must start with the timestamp bracket: {line:?}"));
    let (stamp, rest) = rest
        .split_once("] ")
        .unwrap_or_else(|| panic!("the timestamp bracket must be closed: {line:?}"));
    assert_eq!(
        stamp.len(),
        "2026-10-12T05:00:00Z".len(),
        "the stamp must be an ISO-8601 UTC instant: {stamp:?}"
    );
    assert!(
        stamp.ends_with('Z') && stamp.contains('T'),
        "the stamp must be UTC, not a local time: {stamp:?}"
    );

    let expected = format!("[iter {iteration}] [{category}] ");
    assert!(
        rest.starts_with(&expected),
        "the line must carry iteration {iteration} and category {category:?}: {line:?}"
    );
}

/// `for_iteration` on a real path creates the file and prefixes every line with the iteration.
#[test]
fn a_logger_records_the_iteration_and_the_category_on_every_line() {
    let scratch = Scratch::new("iteration");
    let log = scratch.file("iter.log");
    let mut logger = LoopLogger::for_iteration(7, Some(&log)).expect("the logger must open");

    logger.line("run", "starting");
    logger.gate_start(GateCategory::CssNaming);

    let content = read(&log);
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "two calls must produce two lines: {content:?}"
    );

    assert_stamp(lines[0], 7, "run");
    assert!(
        lines[0].ends_with("starting"),
        "the message must be preserved verbatim: {content:?}"
    );

    assert_stamp(lines[1], 7, &GateCategory::CssNaming.log_label());
    assert!(
        lines[1].ends_with("START"),
        "gate_start must record START: {content:?}"
    );
}

/// `gate_end` distinguishes pass from fail, and records the elapsed time in the documented shape.
#[test]
fn a_failed_gate_is_recorded_as_fail_with_its_elapsed_time() {
    let scratch = Scratch::new("gate-end");
    let log = scratch.file("gate.log");
    let mut logger = LoopLogger::for_iteration(2, Some(&log)).expect("the logger must open");

    logger.gate_end(GateCategory::AestheticMetrics, false, 12.34);
    logger.gate_end(GateCategory::AestheticMetrics, true, 0.5);

    let content = read(&log);
    let lines: Vec<&str> = content.lines().collect();
    assert_eq!(lines.len(), 2, "expected two gate_end lines: {content:?}");

    let label = GateCategory::AestheticMetrics.log_label();
    assert_stamp(lines[0], 2, &label);
    assert!(
        lines[0].ends_with("FAIL (12.3s)"),
        "a failed gate must say FAIL and keep one decimal place: {content:?}"
    );

    assert_stamp(lines[1], 2, &label);
    assert!(
        lines[1].ends_with("PASS (0.5s)"),
        "a passing gate must say PASS: {content:?}"
    );
}

/// Gate output is indented and preserved line for line, in order.
#[test]
fn gate_output_is_written_indented_and_in_order() {
    let scratch = Scratch::new("gate-output");
    let log = scratch.file("output.log");
    let mut logger = LoopLogger::for_iteration(1, Some(&log)).expect("the logger must open");

    let lines = vec![
        "first finding".to_string(),
        String::new(),
        "third finding".to_string(),
    ];
    logger.gate_output(GateCategory::CssVisual, &lines);

    let content = read(&log);
    let written: Vec<&str> = content.split('\n').collect();
    // A trailing newline leaves one empty final element.
    assert_eq!(
        &written[..3],
        &["  | first finding", "  | ", "  | third finding"],
        "each reported line must be indented and kept in order: {content:?}"
    );
}

/// `timed_gate` runs the closure, reports its verdict, and returns the lines plus a real duration.
#[test]
fn a_timed_gate_reports_the_closure_verdict_and_a_non_negative_duration() {
    let scratch = Scratch::new("timed-pass");
    let log = scratch.file("timed.log");
    let mut logger = LoopLogger::for_iteration(3, Some(&log)).expect("the logger must open");

    let (passed, lines, elapsed) = logger.timed_gate(GateCategory::CssAll, || {
        Ok((true, vec!["all good".to_string()]))
    });

    assert!(passed, "the closure reported success");
    assert_eq!(
        lines,
        vec!["all good".to_string()],
        "the closure's lines must be returned unchanged"
    );
    assert!(
        elapsed >= 0.0 && elapsed.is_finite(),
        "the elapsed time must be a real, finite measurement: {elapsed}"
    );

    let content = read(&log);
    // `gate_start` goes through `line`, so the level sits between the category and START.
    let label = GateCategory::CssAll.log_label();
    assert!(
        content.contains(&format!("[{label}] INFO START")),
        "a timed gate must record its start: {content:?}"
    );
    assert!(
        content.contains("  | all good"),
        "a timed gate must record the closure's output: {content:?}"
    );
    assert!(
        content.contains("PASS ("),
        "a timed gate must record the verdict: {content:?}"
    );
}

/// A closure that **fails with an error** is turned into a failed gate whose output carries the
/// error, and the error text is not swallowed.
///
/// This is the branch that matters operationally: a gate that cannot even run must be visibly
/// FAIL, not silently absent, or a broken run looks like a clean one.
#[test]
fn a_timed_gate_that_errors_is_reported_as_failed_with_the_error_in_its_output() {
    let scratch = Scratch::new("timed-error");
    let log = scratch.file("timed-error.log");
    let mut logger = LoopLogger::for_iteration(4, Some(&log)).expect("the logger must open");

    let (passed, lines, elapsed) = logger.timed_gate(GateCategory::AestheticReview, || {
        anyhow::bail!("gate infrastructure unavailable")
    });

    assert!(!passed, "a gate that could not run must not report success");
    assert_eq!(
        lines.len(),
        1,
        "the error must be summarised in exactly one output line: {lines:?}"
    );
    assert!(
        lines[0].contains("ERROR") && lines[0].contains("gate infrastructure unavailable"),
        "the reported line must carry the failure reason: {lines:?}"
    );
    assert!(elapsed >= 0.0 && elapsed.is_finite(), "elapsed: {elapsed}");

    let content = read(&log);
    assert!(
        content.contains("FAIL ("),
        "a gate that errored must be recorded as FAIL: {content:?}"
    );
    assert!(
        content.contains("gate infrastructure unavailable"),
        "the reason must reach the log, not just the return value: {content:?}"
    );
}

/// Without a path the logger still works and writes nothing to disk.
///
/// Asserted by checking that the directory handed in stays empty, which is what "no file" means
/// here -- the alternative (`for_iteration(None)` panicking) would break every caller that runs
/// without per-iteration logs.
#[test]
fn a_logger_without_a_path_writes_no_file() {
    let scratch = Scratch::new("no-path");
    let mut logger = LoopLogger::for_iteration(5, None).expect("a pathless logger must be allowed");

    logger.line("run", "no file expected");
    logger.gate_start(GateCategory::CssVisual);

    let entries: Vec<_> = std::fs::read_dir(&scratch.dir)
        .expect("the scratch directory must be readable")
        .filter_map(Result::ok)
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        entries.is_empty(),
        "a logger without a path must not create files: {entries:?}"
    );
}
