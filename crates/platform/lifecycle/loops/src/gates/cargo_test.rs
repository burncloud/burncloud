use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use serde_json::{json, Value};

use crate::paths::{aesthetic_artifacts_dir, css_visual_artifacts_dir};

/// Build `api_tests` once per loop session; avoids re-linking while burncloud holds locks.
pub fn ensure_api_tests_built(root: &Path) -> anyhow::Result<PathBuf> {
    let output = Command::new("cargo")
        .args([
            "test",
            "-p",
            "burncloud-tests",
            "--test",
            "api_tests",
            "--no-run",
        ])
        .current_dir(root)
        .output()?;
    if !output.status.success() {
        let lines = filter_test_lines(&command_lines(&output));
        anyhow::bail!(
            "failed to build api_tests:\n{}",
            lines.into_iter().take(30).collect::<Vec<_>>().join("\n")
        );
    }
    find_api_tests_exe(root).ok_or_else(|| {
        anyhow::anyhow!("api_tests binary not found under target/debug/deps after --no-run")
    })
}

fn find_api_tests_exe(root: &Path) -> Option<PathBuf> {
    let deps = root.join("target").join("debug").join("deps");
    let mut candidates: Vec<PathBuf> = std::fs::read_dir(deps)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("api_tests-") && n.ends_with(".exe"))
        })
        .collect();
    candidates.sort_by_key(|p| {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
    });
    candidates.pop()
}

pub struct GateOutput {
    pub passed: bool,
    pub lines: Vec<String>,
    pub timings: Value,
}

fn run_api_test_binary(
    root: &Path,
    filter: &str,
    extra_env: impl FnOnce(),
) -> anyhow::Result<GateOutput> {
    extra_env();
    let exe = ensure_api_tests_built(root)?;
    let mut cmd = Command::new(&exe);
    cmd.args([filter, "--ignored", "--nocapture", "--test-threads=1"])
        .current_dir(root);
    for key in [
        "E2E_BASE_URL",
        "E2E_USE_PREVIEW",
        "E2E_FORCE_SPAWN",
        "AESTHETIC_ARTIFACTS_DIR",
        "AESTHETIC_FOCUS_PAGE",
        "CSS_VISUAL_ARTIFACTS_DIR",
        "NO_PROXY",
    ] {
        if let Ok(val) = std::env::var(key) {
            cmd.env(key, val);
        }
    }
    let output = cmd.output()?;
    let raw = command_lines(&output);
    let (lines, timings) = split_timings(filter_test_lines(&raw));
    Ok(GateOutput {
        passed: output.status.success(),
        lines,
        timings,
    })
}

pub fn run_aesthetic_metrics(root: &Path, focus_page: Option<&str>) -> anyhow::Result<(bool, Vec<String>)> {
    let artifacts = aesthetic_artifacts_dir(root);
    std::fs::create_dir_all(&artifacts)?;
    let artifacts_s = artifacts.clone();
    let focus_owned = focus_page.map(|s| s.to_string());
    let result = run_api_test_binary(root, "aesthetic_acceptance", || {
        std::env::set_var("AESTHETIC_ARTIFACTS_DIR", &artifacts_s);
        if let Some(ref focus) = focus_owned {
            std::env::set_var("AESTHETIC_FOCUS_PAGE", focus);
        } else {
            std::env::remove_var("AESTHETIC_FOCUS_PAGE");
        }
        if std::env::var("E2E_USE_PREVIEW").is_err() {
            std::env::set_var("E2E_USE_PREVIEW", "1");
        }
    })?;
    write_timings_file(&artifacts.join("timings.json"), &result.timings)?;
    let mut lines = result.lines;
    if result.passed {
        lines.push(format!(
            "PASS: aesthetic capture + J1 metrics OK. Dir: {}",
            artifacts.display()
        ));
    } else {
        lines.push(format!(
            "FAIL: aesthetic metrics/capture. See {}",
            artifacts.display()
        ));
    }
    Ok((result.passed, lines))
}

pub fn run_css_visual(root: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let artifacts = css_visual_artifacts_dir(root);
    std::fs::create_dir_all(&artifacts)?;
    let artifacts_s = artifacts.clone();
    if std::env::var("E2E_BASE_URL").is_err() {
        std::env::set_var("E2E_FORCE_SPAWN", "1");
    }
    std::env::set_var("NO_PROXY", "*");
    let result = run_api_test_binary(root, "css_visual_acceptance", || {
        std::env::set_var("CSS_VISUAL_ARTIFACTS_DIR", &artifacts_s);
    })?;
    write_timings_file(&artifacts.join("timings.json"), &result.timings)?;
    let mut lines = result.lines;
    if result.passed {
        lines.push(format!(
            "PASS: visual acceptance OK. Screenshots: {}",
            artifacts.display()
        ));
        lines.push(format!(
            "  manifest: {}",
            artifacts.join("manifest.json").display()
        ));
    } else {
        lines.push(format!(
            "FAIL: visual acceptance. See {}",
            artifacts.display()
        ));
    }
    Ok((result.passed, lines))
}

fn write_timings_file(path: &Path, timings: &Value) -> anyhow::Result<()> {
    if timings.get("pages").and_then(|p| p.as_array()).is_some_and(|a| !a.is_empty()) {
        std::fs::write(path, serde_json::to_string_pretty(timings)?)?;
    }
    Ok(())
}

fn command_lines(output: &std::process::Output) -> Vec<String> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    stdout
        .lines()
        .chain(stderr.lines())
        .map(|s| s.to_string())
        .collect()
}

/// Drop cargo noise; keep test output and agent-browser lines.
fn filter_test_lines(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|line| {
            let t = line.trim();
            if t.is_empty() {
                return true;
            }
            if t.starts_with("Compiling ")
                || (t.starts_with("warning:") && (t.contains("crates\\") || t.contains("crates/")))
                || t.starts_with("error: could not compile")
                || t.contains(" --> crates")
                || (t.starts_with("   |") && !t.contains("panicked"))
                || t.contains("^^^^")
                || t.starts_with("   = note:")
                || (t.starts_with("note:") && t.contains("rustc"))
            {
                return false;
            }
            true
        })
        .cloned()
        .collect()
}

fn split_timings(lines: Vec<String>) -> (Vec<String>, Value) {
    let timing_re =
        Regex::new(r"\[E2E-TIMING\] page=(\S+) path=(\S+) open_ms=(\d+) wait_ms=(\d+) total_ms=(\d+) status=(\w+)")
            .expect("timing regex");
    let browser_re =
        Regex::new(r"\[agent-browser\] (\S+)(?: .+)? took (\d+)ms").expect("browser regex");

    let mut pages = Vec::new();
    let mut browser_ops = Vec::new();
    let mut out = Vec::new();

    for line in lines {
        if let Some(cap) = timing_re.captures(&line) {
            pages.push(json!({
                "page": cap[1],
                "path": cap[2],
                "open_ms": cap[3].parse::<u64>().unwrap_or(0),
                "wait_ms": cap[4].parse::<u64>().unwrap_or(0),
                "total_ms": cap[5].parse::<u64>().unwrap_or(0),
                "status": cap[6],
            }));
            out.push(line);
            continue;
        }
        if let Some(cap) = browser_re.captures(&line) {
            browser_ops.push(json!({
                "op": cap[1],
                "ms": cap[2].parse::<u64>().unwrap_or(0),
            }));
            out.push(line);
            continue;
        }
        out.push(line);
    }

    (
        out,
        json!({
            "pages": pages,
            "browser_ops_sample": browser_ops.into_iter().take(50).collect::<Vec<_>>(),
        }),
    )
}

/// Parse agent-browser timings from a gate log (for iteration summary).
pub fn timings_from_log_lines(lines: &[String]) -> Value {
    split_timings(lines.to_vec()).1
}
