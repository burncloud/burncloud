#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Test-only integration file: unwrap/expect is the intended failure signal, and the \
              assertions compare values the crate returns rather than trusting the call to succeed."
)]
//! The gate-vocabulary contract plus the no-side-effect proof (#635, scope
//! `loops-run-lock-and-structured-log-contract`).
//!
//! Two groups live here:
//!
//! 1. `GateCategory` / `GateSuite` name parsing and the labels the structured log is keyed by. These
//!    are the pure, total-or-erroring functions the CLI routes through, and they had no test.
//! 2. A proof that **this crate's own test code invokes no real gate**. `run_css_all`,
//!    `run_aesthetic_full`, `run_jobs_fast_gates`, `run_single_gate` and `run_gate_suite` execute
//!    cargo, browsers and other processes; this scope forbids calling them, and "we did not call it"
//!    is exactly the kind of claim that rots silently, so it is asserted against the source.
//!
//! Nothing in this file starts a process, touches the network, or writes outside a temporary file.

use burncloud_loops::gates::{parse_gate_name, parse_gate_suite, GateCategory, GateSuite};

/// Every legal CLI spelling, paired with the category it must resolve to.
const LEGAL_NAMES: &[(&str, GateCategory)] = &[
    ("css-naming", GateCategory::CssNaming),
    ("naming", GateCategory::CssNaming),
    ("css-all", GateCategory::CssAll),
    ("all", GateCategory::CssAll),
    ("css-visual", GateCategory::CssVisual),
    ("visual", GateCategory::CssVisual),
    ("aesthetic-metrics", GateCategory::AestheticMetrics),
    ("metrics", GateCategory::AestheticMetrics),
    ("aesthetic-review", GateCategory::AestheticReview),
    ("review", GateCategory::AestheticReview),
];

/// `GateCategory` is `PartialEq`, so categories are compared directly. `GateSuite` is not, and is
/// compared through its `Debug` form -- which is what a test failure prints anyway.

/// Every documented spelling resolves to its category, and the short alias agrees with the long one.
#[test]
fn every_legal_gate_name_parses_to_its_category() {
    for (input, expected) in LEGAL_NAMES {
        let parsed = parse_gate_name(input)
            .unwrap_or_else(|e| panic!("{input:?} is a documented gate name but failed: {e}"));
        assert_eq!(
            parsed, *expected,
            "{input:?} must resolve to {expected:?}"
        );
    }

    // The aliases must not merely parse -- they must agree with their long form.
    for (short, long) in [
        ("naming", "css-naming"),
        ("all", "css-all"),
        ("visual", "css-visual"),
        ("metrics", "aesthetic-metrics"),
        ("review", "aesthetic-review"),
    ] {
        assert_eq!(
            parse_gate_name(short).expect("alias must parse"),
            parse_gate_name(long).expect("long form must parse"),
            "{short:?} and {long:?} must name the same category"
        );
    }
}

/// Illegal gate names are refused, including the empty string and anything differing only by case.
///
/// Case matters because the values arrive from a CLI: accepting `CSS-All` would make two spellings
/// for one gate and two log labels for one run.
#[test]
fn an_illegal_gate_name_is_refused_and_the_message_lists_the_legal_ones() {
    for bad in [
        "",
        " ",
        "css",
        "CSS-ALL",
        "Css-All",
        "aesthetic_metrics",
        "gate:css-naming",
        "unknown",
        "css-naming ",
    ] {
        let err = parse_gate_name(bad)
            .err()
            .unwrap_or_else(|| panic!("{bad:?} must not be accepted as a gate name"));
        let message = err.to_string();
        assert!(
            message.contains("unknown gate"),
            "the refusal must name the problem for {bad:?}: {message}"
        );
        assert!(
            message.contains("css-naming") && message.contains("aesthetic-review"),
            "the refusal must list the legal values so a caller can correct itself: {message}"
        );
    }
}

/// Every legal suite name parses, and illegal ones are refused with the legal list.
#[test]
fn gate_suite_names_parse_and_illegal_ones_are_refused() {
    for (input, expected) in [
        ("jobs-fast", GateSuite::JobsFast),
        ("jobs-full", GateSuite::JobsFull),
        ("css-full", GateSuite::CssFull),
        ("aesthetic-full", GateSuite::AestheticFull),
    ] {
        let parsed = parse_gate_suite(input)
            .unwrap_or_else(|e| panic!("{input:?} is a documented suite but failed: {e}"));
        assert_eq!(
            format!("{parsed:?}"),
            format!("{expected:?}"),
            "{input:?} must resolve to {expected:?}"
        );
    }

    for bad in ["", "jobs", "JOBS-FAST", "jobs_fast", "css", "aesthetic"] {
        let err = parse_gate_suite(bad)
            .err()
            .unwrap_or_else(|| panic!("{bad:?} must not be accepted as a suite name"));
        let message = err.to_string();
        assert!(
            message.contains("unknown suite") && message.contains("jobs-fast"),
            "the refusal for {bad:?} must name the problem and list the legal values: {message}"
        );
    }
}

/// Every category has a non-empty, unique log label and a non-empty, unique description.
///
/// A duplicated label would make two different gates indistinguishable in the log, which is the one
/// thing a structured log must not do.
#[test]
fn every_category_has_a_distinct_non_empty_label_and_description() {
    let categories = [
        GateCategory::CssNaming,
        GateCategory::CssAll,
        GateCategory::CssVisual,
        GateCategory::AestheticMetrics,
        GateCategory::AestheticReview,
    ];

    let mut labels = Vec::new();
    let mut descriptions = Vec::new();
    for category in categories {
        let label = category.log_label();
        let description = category.description().to_string();

        assert!(!label.trim().is_empty(), "a label must not be blank");
        assert!(
            label.starts_with("gate:"),
            "labels are the log's gate key and carry the documented prefix: {label:?}"
        );
        assert!(
            !description.trim().is_empty(),
            "every category needs a human-readable description: {label:?}"
        );

        labels.push(label);
        descriptions.push(description);
    }

    let mut sorted = labels.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        sorted.len(),
        labels.len(),
        "log labels must be unique or the log cannot tell two gates apart: {labels:?}"
    );

    let mut sorted_desc = descriptions.clone();
    sorted_desc.sort();
    sorted_desc.dedup();
    assert_eq!(
        sorted_desc.len(),
        descriptions.len(),
        "descriptions must be distinct: {descriptions:?}"
    );
}

/// The call forms that would start a real gate, assembled from halves so that **this file does not
/// contain any of them as a literal**.
///
/// That matters for a scanner that reads its own source: a naive needle list would match itself and
/// the check would fail for the wrong reason. Splitting each needle keeps the detector honest about
/// what it is looking for without weakening what it detects.
const GATE_RUNNER_CALLS: &[&str] = &[
    concat!("run_css", "_all("),
    concat!("run_aesthetic", "_full("),
    concat!("run_jobs", "_fast_gates("),
    concat!("run_single", "_gate("),
    concat!("run_gate", "_suite("),
];

/// The ways a test could start an external process instead of calling a gate runner.
const PROCESS_SPAWN_CALLS: &[&str] = &[
    concat!("Command", "::new("),
    concat!("std::process", "::Command"),
    concat!("tokio::process", "::Command"),
];

/// Every crate source file, with line comments removed.
///
/// Comments are stripped because the module documentation *names* the forbidden functions when it
/// explains why they are forbidden; matching raw text would flag the explanation instead of a call.
fn crate_sources() -> Vec<(String, String)> {
    fn collect(dir: &std::path::Path, out: &mut Vec<(String, String)>) {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return,
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.is_dir() {
                collect(&path, out);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let Ok(text) = std::fs::read_to_string(&path) else {
                continue;
            };
            let stripped: String = text
                .lines()
                .map(|line| match line.find("//") {
                    Some(index) => &line[..index],
                    None => line,
                })
                .collect::<Vec<_>>()
                .join("\n");
            out.push((path.display().to_string(), stripped));
        }
    }

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    collect(&root.join("src"), &mut out);
    collect(&root.join("tests"), &mut out);
    out
}

/// The test files a forbidden call could realistically appear in: every test file **except the
/// files of this increment**.
///
/// The exclusion is what makes the check sound rather than convenient. These files necessarily
/// contain the needles as string literals and in their own panic messages, so a scan that included
/// them would match itself and report a false positive that says nothing about the other tests.
/// The other test files are the ones a future author would add a gate call to, and they are fully
/// scanned.
fn scanned_test_sources() -> Vec<(String, String)> {
    crate_sources()
        .into_iter()
        .filter(|(path, _)| !path.ends_with("gates_vocabulary.rs") && !path.ends_with("log_contract.rs"))
        .collect()
}

/// AC6: no source file of this crate calls a real gate runner.
///
/// The five functions below execute cargo, browsers and other processes. A test that called one
/// would run an acceptance gate inside `cargo test` -- slow, environment-dependent, and forbidden by
/// this scope. The check reads the crate's own source so that "we did not call it" cannot quietly
/// stop being true.
#[test]
fn no_crate_source_invokes_a_real_gate_runner() {
    let sources = scanned_test_sources();
    assert!(
        !sources.is_empty(),
        "the scan must find the crate's other test files, otherwise it proves nothing"
    );

    let mut hits = Vec::new();
    for (path, text) in &sources {
        for needle in GATE_RUNNER_CALLS {
            if text.contains(needle) {
                hits.push(format!("{path}: {needle}"));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "a test must not run a real gate (it would execute cargo or a browser): {hits:?}"
    );
}

/// AC6, second half: no other test in this crate shells out.
///
/// The gate runners are the obvious way to start a process; this catches the indirect one, so a
/// future test cannot quietly spawn `cargo` instead.
#[test]
fn no_other_crate_test_starts_an_external_process() {
    let mut hits = Vec::new();
    for (path, text) in scanned_test_sources() {
        for needle in PROCESS_SPAWN_CALLS {
            if text.contains(needle) {
                hits.push(format!("{path}: {needle}"));
            }
        }
    }

    assert!(
        hits.is_empty(),
        "the loops tests must stay in-process: {hits:?}"
    );
}
