//! Local check history and the matching commit-message receipt.
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::test::git;

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Counts {
    pub passed: u64,
    pub failed: u64,
    pub ignored: u64,
}

#[derive(Default)]
struct TestCounts {
    unit: Option<Counts>,
    integration: Option<Counts>,
    doc: Option<Counts>,
    other: Option<Counts>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Step {
    pub name: String,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
    pub log: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Summary {
    pub version: u32,
    pub status: String,
    pub mode: String,
    pub head: String,
    pub staged_tree: Option<String>,
    pub paths: Vec<String>,
    pub packages: Vec<String>,
    pub started_ms: u128,
    pub finished_ms: Option<u128>,
    pub steps: Vec<Step>,
    pub unit: Option<Counts>,
    pub integration: Option<Counts>,
    pub doc: Option<Counts>,
    pub other: Option<Counts>,
    pub error: Option<String>,
    pub directory: String,
}

fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn history(root: &Path) -> Result<PathBuf> {
    Ok(root
        .join(git(root, &["rev-parse", "--git-dir"])?.trim())
        .join("burncloud/checks"))
}

fn head(root: &Path) -> String {
    git(root, &["rev-parse", "--verify", "HEAD"])
        .map(|s| s.trim().to_owned())
        .unwrap_or_default()
}

pub(crate) fn new(
    root: &Path,
    mode: &str,
    paths: Vec<String>,
    packages: Vec<String>,
) -> Result<Summary> {
    let started_ms = now();
    let directory = history(root)?.join(format!("{started_ms}-{}", std::process::id()));
    fs::create_dir_all(&directory)?;
    let summary = Summary {
        version: 1,
        status: "running".into(),
        mode: mode.into(),
        head: head(root),
        staged_tree: (mode == "staged")
            .then(|| git(root, &["write-tree"]).map(|s| s.trim().to_owned()))
            .transpose()?,
        paths,
        packages,
        started_ms,
        finished_ms: None,
        steps: Vec::new(),
        unit: None,
        integration: None,
        doc: None,
        other: None,
        error: None,
        directory: directory.to_string_lossy().into_owned(),
    };
    save(root, &summary)?;
    Ok(summary)
}

pub(crate) fn save(root: &Path, summary: &Summary) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(summary)?;
    fs::write(Path::new(&summary.directory).join("summary.json"), &bytes)?;
    fs::write(history(root)?.join("latest.json"), bytes)?;
    Ok(())
}

pub(crate) fn finish(
    root: &Path,
    summary: &mut Summary,
    status: &str,
    error: Option<String>,
) -> Result<()> {
    summary.status = status.into();
    summary.error = error;
    summary.finished_ms = Some(now());
    save(root, summary)?;
    show(summary);
    Ok(())
}

pub(crate) fn execute(
    root: &Path,
    summary: &mut Summary,
    name: &str,
    args: &[String],
) -> Result<ExitStatus> {
    let log =
        Path::new(&summary.directory).join(format!("{:02}-{name}.log", summary.steps.len() + 1));
    let writer = File::create(&log)?;
    let mut reader = File::open(&log)?;
    let started = Instant::now();
    summary.steps.push(Step {
        name: name.into(),
        command: format!("cargo {}", args.join(" ")),
        status: "running".into(),
        exit_code: None,
        duration_ms: 0,
        log: log.to_string_lossy().into_owned(),
    });
    save(root, summary)?;
    let result = (|| -> Result<ExitStatus> {
        let mut child = Command::new("cargo")
            .args(args)
            .current_dir(root)
            .stdout(Stdio::from(writer.try_clone()?))
            .stderr(Stdio::from(writer))
            .spawn()
            .context("Cannot start Cargo check")?;
        let mut out = std::io::stdout().lock();
        loop {
            let mut buf = [0u8; 8192];
            let size = reader.read(&mut buf)?;
            if size > 0 {
                out.write_all(&buf[..size])?;
                out.flush()?;
                continue;
            }
            if let Some(status) = child.try_wait()? {
                let mut rest = Vec::new();
                reader.read_to_end(&mut rest)?;
                out.write_all(&rest)?;
                out.flush()?;
                return Ok(status);
            }
            thread::sleep(Duration::from_millis(50));
        }
    })();
    if name == "test" {
        let content = fs::read_to_string(&log).unwrap_or_default();
        let parsed = parse_tests(&content);
        summary.unit = parsed.unit;
        summary.integration = parsed.integration;
        summary.doc = parsed.doc;
        summary.other = parsed.other;
    }
    let step = summary
        .steps
        .last_mut()
        .context("internal error: check step disappeared after insertion")?;
    step.duration_ms = started.elapsed().as_millis();
    step.exit_code = result.as_ref().ok().and_then(ExitStatus::code);
    step.status = if result.as_ref().is_ok_and(ExitStatus::success) {
        "passed"
    } else {
        "failed"
    }
    .into();
    let passed = step.status == "passed";
    let duration = step.duration_ms;
    save(root, summary)?;
    println!(
        "{} {name} ({} ms) | {}",
        if passed { "✅" } else { "❌" },
        duration,
        log.display()
    );
    result
}

fn parse_tests(content: &str) -> TestCounts {
    let mut parsed = TestCounts::default();
    let mut kind = 3;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("Running unittests ") {
            kind = 0;
        } else if trimmed.starts_with("Running tests/") || trimmed.starts_with("Running ") {
            kind = 1;
        } else if trimmed.starts_with("Doc-tests ") {
            kind = 2;
        }
        if !trimmed.starts_with("test result:") {
            continue;
        }
        let words: Vec<_> = trimmed.split_whitespace().collect();
        let count = |label: &str| -> Option<u64> {
            words
                .windows(2)
                .find(|pair| pair[1].trim_end_matches(';') == label)
                .and_then(|pair| pair[0].parse().ok())
        };
        if let (Some(passed), Some(failed), Some(ignored)) =
            (count("passed"), count("failed"), count("ignored"))
        {
            let slot = match kind {
                0 => &mut parsed.unit,
                1 => &mut parsed.integration,
                2 => &mut parsed.doc,
                _ => &mut parsed.other,
            };
            let total = slot.get_or_insert_with(Counts::default);
            total.passed += passed;
            total.failed += failed;
            total.ignored += ignored;
        }
    }
    parsed
}

fn counts(summary: &Summary) -> String {
    let mut total = Counts::default();
    let mut parts = Vec::new();
    for (name, value) in [
        ("unit", &summary.unit),
        ("integration", &summary.integration),
        ("doc", &summary.doc),
        ("other", &summary.other),
    ] {
        if let Some(v) = value {
            total.passed += v.passed;
            total.failed += v.failed;
            total.ignored += v.ignored;
            parts.push(format!(
                "{name} {} passed, {} failed, {} ignored",
                v.passed, v.failed, v.ignored
            ));
        }
    }
    if parts.is_empty() {
        return "count unavailable".into();
    }
    format!(
        "{} passed, {} failed, {} ignored; {}",
        total.passed,
        total.failed,
        total.ignored,
        parts.join("; ")
    )
}

fn show(summary: &Summary) {
    println!(
        "Check history: {} | {} | {}",
        summary.status.to_uppercase(),
        summary.mode,
        summary.directory
    );
    for step in &summary.steps {
        let marker = match step.status.as_str() {
            "passed" => "✅",
            "failed" => "❌",
            _ => "⏳",
        };
        println!(
            "  {marker} {}: {} ({} ms) | {}",
            step.name, step.status, step.duration_ms, step.log
        );
    }
    if summary.steps.iter().any(|step| step.name == "test") {
        println!("  Tests: {}", counts(summary));
    }
    if let Some(error) = &summary.error {
        println!("  Error: {error}");
    }
}

fn latest(root: &Path) -> Result<Summary> {
    serde_json::from_slice(
        &fs::read(history(root)?.join("latest.json")).context("No saved code test result yet")?,
    )
    .context("Invalid saved code test result")
}

pub(crate) fn show_latest(root: &Path) -> Result<()> {
    show(&latest(root)?);
    Ok(())
}

pub(crate) fn stamp(message: &Path) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let root = PathBuf::from(git(&cwd, &["rev-parse", "--show-toplevel"])?.trim());
    let summary = latest(&root)?;
    anyhow::ensure!(
        summary.mode == "staged" && matches!(summary.status.as_str(), "passed" | "skipped"),
        "No successful code test --staged result; run cargo run -- code test --staged"
    );
    anyhow::ensure!(
        summary.head == head(&root)
            && summary.staged_tree.as_deref() == Some(git(&root, &["write-tree"])?.trim()),
        "Staged tree or HEAD changed after checks; rerun cargo run -- code test --staged"
    );
    anyhow::ensure!(
        git(
            &root,
            &[
                "diff",
                "--name-only",
                "--no-renames",
                "--ignore-submodules=none",
                "-z",
                "--"
            ]
        )?
        .is_empty()
            && git(&root, &["ls-files", "--others", "--exclude-standard", "-z"])?.is_empty(),
        "Working tree changed after checks; rerun cargo run -- code test --staged"
    );
    if summary.status == "passed" {
        let test_steps = summary
            .steps
            .iter()
            .filter(|step| step.name == "test")
            .count();
        anyhow::ensure!(
            test_steps == 1
                && summary.steps.iter().all(|step| {
                    matches!(step.name.as_str(), "prepare" | "test") && step.status == "passed"
                }),
            "Incomplete saved code test result"
        );
    } else {
        anyhow::ensure!(summary.steps.is_empty(), "Invalid skipped check result");
    }
    let raw = fs::read_to_string(message).context("Cannot read commit message")?;
    let mut lines: Vec<&str> = raw.lines().collect();
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let trailer = |line: &str| {
        [
            "BurnCloud-Checks:",
            "BurnCloud-Fmt:",
            "BurnCloud-Tests:",
            "BurnCloud-Clippy:",
            "BurnCloud-Deny:",
            "BurnCloud-Checks-Tree:",
        ]
        .iter()
        .any(|prefix| line.starts_with(prefix))
    };
    while lines.last().is_some_and(|line| trailer(line)) {
        lines.pop();
    }
    while lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let (overall, checks, tests) = if summary.status == "skipped" {
        (
            "✅ PASS (tests skipped: no test-relevant changes)",
            "✅ PASS",
            "➖ SKIP".to_owned(),
        )
    } else {
        (
            "✅ PASS",
            "✅ PASS",
            format!("✅ PASS ({})", counts(&summary)),
        )
    };
    let body = format!("{}\n\nBurnCloud-Checks: {overall}\nBurnCloud-Fmt: {checks}\nBurnCloud-Tests: {tests}\nBurnCloud-Clippy: {checks}\nBurnCloud-Deny: {checks}\nBurnCloud-Checks-Tree: {}\n", lines.join("\n"), summary.staged_tree.as_deref().unwrap_or_default());
    fs::write(message, body)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_tests;

    #[test]
    fn parses_suite_counts() {
        let parsed = parse_tests("Running unittests src/lib.rs\ntest result: ok. 3 passed; 1 failed; 2 ignored; 0 measured\nRunning tests/a.rs\ntest result: ok. 4 passed; 0 failed; 0 ignored; 0 measured\nDoc-tests foo\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured");
        assert_eq!(parsed.unit.unwrap().passed, 3);
        assert_eq!(parsed.integration.unwrap().passed, 4);
        assert_eq!(parsed.doc.unwrap().passed, 1);
        assert!(parsed.other.is_none());
    }
}
