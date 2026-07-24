use std::collections::HashSet;
use std::path::{Path, PathBuf};

use chrono::Utc;
use regex::Regex;
use serde_json::{json, Value};

use crate::gates::GateCategory;
use crate::paths::{aesthetic_artifacts_dir, css_visual_artifacts_dir, jobs_aesthetic_run_dir};

const DIMENSIONS: &[&str] = &["C", "F", "D", "A", "R", "P", "E"];

#[derive(Debug, Clone)]
struct Failure {
    layer: String,
    page: String,
    error: String,
}

pub struct PromptInput {
    pub iteration: u32,
    pub phase: String,
    pub css_ok: bool,
    pub metrics_ok: bool,
    pub review_ok: bool,
    pub focus_page: String,
    pub completed_pages: Vec<String>,
    pub check_log: Option<PathBuf>,
    pub root: PathBuf,
}

pub fn build_jobs_aesthetic_prompt(input: &PromptInput) -> anyhow::Result<PathBuf> {
    let artifacts = aesthetic_artifacts_dir(&input.root);
    let run_dir = jobs_aesthetic_run_dir(&input.root);
    std::fs::create_dir_all(&artifacts)?;
    std::fs::create_dir_all(&run_dir)?;

    let out_md = run_dir.join("agent-prompt.md");
    let out_json = run_dir.join("agent-prompt.json");
    let metrics_path = artifacts.join("metrics.json");
    let review_path = artifacts.join("review.json");
    let css_visual = css_visual_artifacts_dir(&input.root);

    let mut failures: Vec<Failure> = Vec::new();
    let mut screenshots: Vec<String> = Vec::new();
    let mut priority_pages: Vec<String> = Vec::new();

    let check_log = input.check_log.clone().or_else(|| latest_check_log(&run_dir));
    let focus = input.focus_page.clone();
    if let Some(log_path) = check_log.as_ref() {
        parse_check_log(log_path, &focus, &mut failures, &mut priority_pages);
    }

    collect_fail_screenshots(
        &artifacts,
        &css_visual,
        &focus,
        &mut failures,
        &mut screenshots,
        &mut priority_pages,
    );
    parse_metrics_file(&metrics_path, &focus, &mut failures, &mut priority_pages);
    parse_review_file(
        &review_path,
        &focus,
        input.css_ok,
        input.metrics_ok,
        input.review_ok,
        &input.phase,
        &mut failures,
        &mut priority_pages,
    );

    failures.retain(|f| failure_applies_to_focus(f, &focus));
    priority_pages.retain(|p| p == &focus);

    let unique_priority: Vec<String> = priority_pages
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    let vp = artifacts.join(format!("{}-viewport.png", input.focus_page));
    if vp.exists() {
        let vp_s = vp.to_string_lossy().into_owned();
        if !screenshots.contains(&vp_s) {
            screenshots.insert(0, vp_s);
        }
    }
    let full = artifacts.join(format!("{}-full.png", input.focus_page));
    if full.exists() {
        let full_s = full.to_string_lossy().into_owned();
        if !screenshots.contains(&full_s) {
            screenshots.push(full_s);
        }
    }

    let max_files = if input.phase == "css" { 8 } else { 4 };
    let gate_hint = if !input.css_ok {
        "Gate 1: fix CSS until `cargo run -p burncloud-loops -- gate css-naming` exits 0".to_string()
    } else if !input.metrics_ok {
        format!(
            "Gate 2: fix J1 on `{}` until metrics pass",
            input.focus_page
        )
    } else {
        format!(
            "Gate 3: score + polish `{}` only in review.json (all C/F/D/A/R/P/E ge 4, p0 empty)",
            input.focus_page
        )
    };

    let completed_note = if input.completed_pages.is_empty() {
        "(none yet)".to_string()
    } else {
        input.completed_pages.join(", ")
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "# Jobs aesthetic loop - iteration {} - phase: {} - page: {}",
        input.iteration, input.phase, input.focus_page
    ));
    lines.push(String::new());
    lines.push("## Mission".to_string());
    lines.push(format!(
        "Fix ONLY `{}` this round. DO NOT edit other pages. Code root: {}",
        input.focus_page,
        input.root.display()
    ));
    lines.push(format!(
        "Completed pages (frozen — observers can preview these): {completed_note}"
    ));
    lines.push(
        "When this page passes review, the loop advances to the next page automatically."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("## Voice".to_string());
    lines.push(
        "You are Steve Jobs reviewing this screen. Use your own words — direct, impatient with \
         mediocrity, obsessed with simplicity. Do not recite checklists, dimension codes, or \
         canned quotes. The failures below are facts; your critique and fixes are yours."
            .to_string(),
    );
    lines.push(String::new());
    lines.push("## Failures (fix in order)".to_string());
    if failures.is_empty() {
        lines.push(format!(
            "- Open `{}-viewport.png`; improve layout until all J3 scores ge 4",
            input.focus_page
        ));
    } else {
        for f in failures.iter().take(12) {
            lines.push(format!("- [{}] {}: {}", f.layer, f.page, f.error));
        }
    }
    lines.push(String::new());
    lines.push("## Screenshots (this page only)".to_string());
    if screenshots.is_empty() {
        lines.push(format!(
            "- {}",
            artifacts.join(format!("{}-viewport.png", input.focus_page)).display()
        ));
    } else {
        for s in screenshots.iter().take(4) {
            lines.push(format!("- {s}"));
        }
    }
    lines.push(String::new());
    lines.push("## This round".to_string());
    lines.push(format!("- {gate_hint}"));
    lines.push(format!("- Focus page: {}", input.focus_page));
    lines.push(format!("- Max {max_files} files (this page only)"));
    lines.push("- Do NOT touch completed pages or other routes".to_string());
    lines.push("- Run: cd crates/client; cargo check -p burncloud-client-shared".to_string());
    lines.push(format!(
        "- Update review.json entry for `{}` only; leave other pages unchanged",
        input.focus_page
    ));
    lines.push("- Constraints: crates/loops/acceptance/jobs-aesthetic-agent-prompt.md".to_string());
    if let Some(log) = check_log.as_ref() {
        lines.push(format!("- Check log: {}", log.display()));
    }

    let md = lines.join("\n") + "\n";
    let iter_md = run_dir.join(format!("agent-prompt-{}.md", input.iteration));
    let iter_json = run_dir.join(format!("agent-prompt-{}.json", input.iteration));
    std::fs::write(&out_md, &md)?;
    std::fs::write(&iter_md, &md)?;

    let failure_export: Vec<Value> = failures
        .iter()
        .map(|f| {
            json!({
                "layer": f.layer,
                "page": f.page,
                "error": f.error,
            })
        })
        .collect();
    let payload = json!({
        "iteration": input.iteration,
        "phase": input.phase,
        "css_ok": input.css_ok,
        "metrics_ok": input.metrics_ok,
        "review_ok": input.review_ok,
        "failures": failure_export,
        "focus_page": input.focus_page,
        "completed_pages": input.completed_pages,
        "priority_pages": unique_priority.iter().take(5).collect::<Vec<_>>(),
        "screenshots": screenshots,
        "generated_at": Utc::now().to_rfc3339(),
    });
    std::fs::write(&out_json, serde_json::to_string_pretty(&payload)?)?;
    std::fs::write(&iter_json, serde_json::to_string_pretty(&payload)?)?;

    eprintln!("Generated agent prompt: {}", out_md.display());
    Ok(out_md)
}

fn latest_check_log(run_dir: &Path) -> Option<PathBuf> {
    let mut best: Option<(i32, PathBuf)> = None;
    for entry in std::fs::read_dir(run_dir).ok()? {
        let entry = entry.ok()?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if let Some(num) = name
            .strip_prefix("loop-check-")
            .and_then(|s| s.strip_suffix(".log"))
            .and_then(|s| s.parse::<i32>().ok())
        {
            if best.as_ref().is_none_or(|(n, _)| num > *n) {
                best = Some((num, entry.path()));
            }
        }
    }
    best.map(|(_, p)| p)
}

fn read_log_tail(path: &Path, max_bytes: usize) -> anyhow::Result<String> {
    let data = std::fs::read(path)?;
    if data.len() <= max_bytes {
        return Ok(String::from_utf8_lossy(&data).into_owned());
    }
    let start = data.len() - max_bytes;
    Ok(String::from_utf8_lossy(&data[start..]).into_owned())
}

fn path_to_aesthetic_key(path: &str) -> Option<String> {
    match path {
        "/" | "/preview/home" => Some("aesthetic-home".to_string()),
        "/login" | "/preview/login" => Some("aesthetic-login".to_string()),
        p if p.starts_with("/preview/console/") => {
            let slug = p.trim_start_matches("/preview/console/");
            Some(format!("aesthetic-{slug}"))
        }
        p if p.starts_with("/console/") => {
            let slug = p.trim_start_matches("/console/");
            Some(format!("aesthetic-{slug}"))
        }
        _ => None,
    }
}

fn failure_applies_to_focus(f: &Failure, focus: &str) -> bool {
    match f.layer.as_str() {
        "css-naming" | "infra" => true,
        "j3" | "j3-p0" | "j1" | "screenshot" => f.page == focus || f.page.contains(focus),
        "css-visual" => f.page == focus,
        "j4" => false, // deferred until last page
        "review" => f.page == "review.json" && f.error.contains(focus),
        _ => f.page == focus,
    }
}

fn parse_check_log(
    path: &Path,
    focus: &str,
    failures: &mut Vec<Failure>,
    priority_pages: &mut Vec<String>,
) {
    let Ok(text) = read_log_tail(path, 512_000) else {
        return;
    };

    let page_load = Regex::new(r"Page (/[^\s]+) did not load:?\s*([^\r\n]*)").expect("regex");
    for cap in page_load.captures_iter(&text) {
        let path = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        if path_to_aesthetic_key(path).as_deref() != Some(focus) {
            continue;
        }
        let detail = cap.get(2).map(|m| m.as_str()).unwrap_or("").trim();
        let msg = cap.get(0).map(|m| m.as_str()).unwrap_or("").trim();
        let layer = if path.contains("/preview/") || path.starts_with("/console/") {
            "j1"
        } else {
            "css-visual"
        };
        failures.push(Failure {
            layer: layer.into(),
            page: path.into(),
            error: if detail.is_empty() {
                msg.into()
            } else {
                format!("{msg} ({detail})")
            },
        });
        if let Some(key) = path_to_aesthetic_key(path) {
            if key == focus {
                priority_pages.push(key);
            }
        }
    }
    let panic_load =
        Regex::new(r"panicked at [^:]+:\d+:\d+:\s*Page (/[^\s]+) did not load").expect("regex");
    for cap in panic_load.captures_iter(&text) {
        let path = &cap[1];
        if failures.iter().any(|f| f.page == path) {
            continue;
        }
        if path_to_aesthetic_key(path).as_deref() != Some(focus) {
            continue;
        }
        failures.push(Failure {
            layer: "j1".into(),
            page: path.to_string(),
            error: "page did not load (see gate log / FAIL-load screenshot)".into(),
        });
        if let Some(key) = path_to_aesthetic_key(path) {
            priority_pages.push(key);
        }
    }

    if let Some(cap) = Regex::new(r"Server failed to start at (http[^\r\n]+)")
        .expect("Invalid regex pattern for server start error")
        .captures(&text)
    {
        failures.push(Failure {
            layer: "infra".into(),
            page: "server".into(),
            error: format!("Server failed to start at {}", &cap[1]),
        });
    }

    let layout = Regex::new(r"Layout check failed on ([^:]+): ([^\r\n]+)").expect("regex");
    for cap in layout.captures_iter(&text) {
        failures.push(Failure {
            layer: "css-visual".into(),
            page: cap[1].to_string(),
            error: cap[2].to_string(),
        });
    }

    let naming = Regex::new(r"::error::([^\r\n]+)").expect("regex");
    for cap in naming.captures_iter(&text) {
        failures.push(Failure {
            layer: "css-naming".into(),
            page: "console".into(),
            error: cap[1].to_string(),
        });
    }

    let metrics = Regex::new(r"Aesthetic metrics failed on ([^:]+): ([^\r\n]+)").expect("regex");
    for cap in metrics.captures_iter(&text) {
        let page_key = cap[1].to_string();
        if page_key != focus {
            continue;
        }
        let err = cap[2].to_string();
        failures.push(Failure {
            layer: "j1".into(),
            page: page_key.clone(),
            error: err.clone(),
        });
        priority_pages.push(page_key);
    }

    for line in text.lines().rev().take(400) {
        if let Some(rest) = line.trim().strip_prefix("- ") {
            if rest.contains(focus) {
                failures.push(Failure {
                    layer: "review".into(),
                    page: "review.json".into(),
                    error: rest.to_string(),
                });
            }
        }
    }
}

fn collect_fail_screenshots(
    artifacts: &Path,
    css_visual: &Path,
    focus: &str,
    failures: &mut Vec<Failure>,
    screenshots: &mut Vec<String>,
    priority_pages: &mut Vec<String>,
) {
    for dir in [artifacts, css_visual] {
        if !dir.is_dir() {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(dir) else { continue };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if !name.starts_with("FAIL-") || !name.ends_with(".png") {
                continue;
            }
            let base = name.trim_end_matches(".png");
            let page_key = base
                .strip_prefix("FAIL-load-")
                .or_else(|| base.strip_prefix("FAIL-metrics-"))
                .or_else(|| base.strip_prefix("FAIL-layout-"));
            let Some(page_key) = page_key else { continue };
            let normalized = if let Some(p) = page_key.strip_prefix("visual-") {
                format!("aesthetic-{p}")
            } else {
                page_key.to_string()
            };
            if normalized != focus {
                continue;
            }
            let path = entry.path().to_string_lossy().into_owned();
            screenshots.push(path.clone());
            if base.starts_with("FAIL-load-") {
                failures.push(Failure {
                    layer: "screenshot".into(),
                    page: normalized.clone(),
                    error: format!("page load or check failed (see {name})"),
                });
                priority_pages.push(normalized);
            } else if base.starts_with("FAIL-metrics-") {
                failures.push(Failure {
                    layer: "j1".into(),
                    page: normalized.clone(),
                    error: format!("metrics failed (see {name})"),
                });
                priority_pages.push(normalized);
            } else if base.starts_with("FAIL-layout-") {
                failures.push(Failure {
                    layer: "css-visual".into(),
                    page: normalized,
                    error: "layout check failed".into(),
                });
            }
        }
    }
}

fn parse_metrics_file(
    path: &Path,
    focus: &str,
    failures: &mut Vec<Failure>,
    priority_pages: &mut Vec<String>,
) {
    let Ok(text) = std::fs::read_to_string(path) else { return };
    let Ok(metrics) = serde_json::from_str::<Value>(&text) else { return };
    let Some(pages) = metrics.get("pages").and_then(|p| p.as_object()) else {
        return;
    };
    for (page_key, entry) in pages {
        if page_key != focus {
            continue;
        }
        if entry.get("status").and_then(|s| s.as_str()) == Some("fail") {
            let err = entry
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("metric fail");
            failures.push(Failure {
                layer: "j1".into(),
                page: page_key.clone(),
                error: err.into(),
            });
            priority_pages.push(page_key.clone());
        }
    }
}

fn parse_review_file(
    path: &Path,
    focus: &str,
    css_ok: bool,
    metrics_ok: bool,
    review_ok: bool,
    phase: &str,
    failures: &mut Vec<Failure>,
    priority_pages: &mut Vec<String>,
) {
    if review_ok {
        return;
    }
    let include_review = !review_ok && (metrics_ok || phase == "review");
    let Ok(text) = std::fs::read_to_string(path) else { return };
    let Ok(review) = serde_json::from_str::<Value>(&text) else { return };

    if include_review {
        if let Some(pages) = review.get("pages").and_then(|p| p.as_object()) {
            if let Some(entry) = pages.get(focus) {
                if let Some(p0) = entry.get("p0").and_then(|v| v.as_array()) {
                    for item in p0 {
                        if let Some(s) = item.as_str() {
                            if s == "not-reviewed" {
                                if css_ok && metrics_ok {
                                    failures.push(Failure {
                                        layer: "j3".into(),
                                        page: focus.to_string(),
                                        error: "not reviewed".into(),
                                    });
                                    priority_pages.push(focus.to_string());
                                }
                            } else {
                                failures.push(Failure {
                                    layer: "j3-p0".into(),
                                    page: focus.to_string(),
                                    error: format!("P0: {s}"),
                                });
                                priority_pages.push(focus.to_string());
                            }
                        }
                    }
                }
                if let Some(scores) = entry.get("scores").and_then(|s| s.as_object()) {
                    for dim in DIMENSIONS {
                        if let Some(s) = scores.get(*dim).and_then(|v| v.as_i64()) {
                            if s < 4 {
                                failures.push(Failure {
                                    layer: "j3".into(),
                                    page: focus.to_string(),
                                    error: format!("{dim}={s} (need ge 4)"),
                                });
                                priority_pages.push(focus.to_string());
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(dead_code)]
pub fn required_pages() -> &'static [&'static str] {
    GateCategory::JOBS_AESTHETIC_PAGES
}
