#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "integration test: an unwrap/expect failure is the intended failure signal"
)]
//! Contract-purity guard for the Traffic contract (S1-E), same rule as the Commerce and Supply
//! contracts.
//!
//! A Traffic contract that grows a database framework, a web server, an HTTP client or a logging
//! implementation has stopped being a contract. The specific rule this crate must not break is that
//! it cannot depend on the router implementation package: Identity names `TrafficColor`, and the
//! whole point of moving that name into a contract was to avoid an Identity -> router dependency.

use std::fs;
use std::path::PathBuf;

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
    "dashmap",
];

fn manifest() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// Dependency tables only, so a mention in a comment or the description is not a failure.
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
    let violations: Vec<String> = deps
        .lines()
        .filter_map(|line| {
            let name = line.split(['=', '.']).next().unwrap_or_default().trim();
            FORBIDDEN.contains(&name).then(|| name.to_string())
        })
        .collect();
    assert!(
        violations.is_empty(),
        "the Traffic contract must not depend on infrastructure crates: {violations:?}\n{deps}"
    );
}

#[test]
fn contract_does_not_depend_on_other_burncloud_crates() {
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
        "the Traffic contract must stay a leaf package (in particular it must not depend on \
         burncloud-router or burncloud-common), found: {internal:?}"
    );
}

#[test]
fn contract_source_has_no_database_or_runtime_code() {
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
                for banned in ["sqlx", "tracing", "FromRow", "axum", "reqwest", "tokio"] {
                    if code.contains(banned) {
                        offenders.push(format!("{}:{}: {line}", path.display(), idx + 1));
                    }
                }
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "infrastructure code found in the Traffic contract:\n{}",
        offenders.join("\n")
    );
}
