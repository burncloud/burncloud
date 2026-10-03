#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Contract-purity guard (review feedback on #602, items 1 and 3).
//!
//! `burncloud-commerce-contracts` is the Commerce domain contract. A domain contract must not grow
//! a database framework, a web server, an HTTP client, a UI toolkit or a logging implementation:
//! those are adapter/infrastructure concerns. If one of them is genuinely needed, it belongs in the
//! adapter crate, not here.
//!
//! This test reads the crate's own manifest and fails when such a dependency appears, so the
//! boundary is enforced by CI instead of by review attention alone.

use std::fs;
use std::path::PathBuf;

/// Crates that must never be a dependency of the domain contract.
const FORBIDDEN: &[&str] = &[
    "sqlx",
    "axum",
    "reqwest",
    "dioxus",
    "dioxus-liveview",
    "tokio",
    "tracing",
    "tracing-subscriber",
    "hyper",
    "tower",
    "tower-http",
    "bcrypt",
    "jsonwebtoken",
];

fn manifest() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The dependency tables only, so a mention inside a comment or the description is not a failure.
fn dependency_section(text: &str) -> String {
    let mut section = String::new();
    let mut in_table = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_table = trimmed.contains("dependencies");
            continue;
        }
        if in_table {
            section.push_str(line);
            section.push('\n');
        }
    }
    section
}

#[test]
fn contract_has_no_infrastructure_dependencies() {
    let deps = dependency_section(&manifest());
    let mut violations = Vec::new();
    for line in deps.lines() {
        let name = line
            .split(['=', '.'])
            .next()
            .unwrap_or_default()
            .trim()
            .to_string();
        if name.is_empty() {
            continue;
        }
        if FORBIDDEN.contains(&name.as_str()) {
            violations.push(name);
        }
    }
    assert!(
        violations.is_empty(),
        "the Commerce contract must not depend on infrastructure crates: {violations:?}\n\
         dependency section was:\n{deps}"
    );
}

#[test]
fn contract_does_not_depend_on_other_burncloud_crates() {
    // A domain contract is a leaf: if it needs another BurnCloud crate, the contract has been
    // split in the wrong place (it would also risk a package cycle, see #601).
    let deps = dependency_section(&manifest());
    let internal: Vec<String> = deps
        .lines()
        .filter_map(|line| {
            let name = line.split(['=', '.']).next().unwrap_or_default().trim();
            name.starts_with("burncloud-").then(|| name.to_string())
        })
        .collect();
    assert!(
        internal.is_empty(),
        "the Commerce contract must stay a leaf package, found: {internal:?}"
    );
}

#[test]
fn contract_source_has_no_database_or_logging_code() {
    // Belt and braces for the review point: even with the manifest clean, the source must not
    // reach for those crates. Comments are ignored so the docs may name the adapter that owns the
    // row structs.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offenders = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let text = fs::read_to_string(&path).unwrap();
            for (idx, line) in text.lines().enumerate() {
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                for banned in ["sqlx", "tracing", "FromRow", "axum", "reqwest"] {
                    if code.contains(banned) {
                        offenders.push(format!("{}:{}: {line}", path.display(), idx + 1));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "infrastructure code found in the domain contract:\n{}",
        offenders.join("\n")
    );
}
