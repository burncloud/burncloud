// Aesthetic review artifacts are intentionally schemaless JSON produced by external tools.
#![allow(clippy::disallowed_types)]

use serde_json::Value;
use std::path::Path;

use crate::paths::aesthetic_artifacts_dir;

const REQUIRED_PAGES: &[&str] = &[
    "aesthetic-home",
    "aesthetic-login",
    "aesthetic-dashboard",
    "aesthetic-models",
    "aesthetic-access",
    "aesthetic-settings",
    "aesthetic-finance",
    "aesthetic-monitor",
    "aesthetic-playground",
];

const DIMENSIONS: &[&str] = &["C", "F", "D", "A", "R", "P", "E"];

const J4_KEYS: &[&str] = &[
    "page_header_consistent",
    "sidebar_consistent",
    "card_consistent",
    "button_consistent",
    "table_consistent",
    "empty_state_consistent",
    "spacing_consistent",
    "motion_consistent",
];

/// When set, review gate only requires the current page (plus regression on completed pages).
#[derive(Debug, Clone)]
pub struct ReviewScope {
    pub current_page: String,
    pub completed_pages: Vec<String>,
    /// When true (last page in queue), also require global_j4 + review.pass.
    pub require_global_j4: bool,
}

pub fn run_aesthetic_review(root: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    run_aesthetic_review_scoped(root, None)
}

pub fn run_aesthetic_review_scoped(
    root: &Path,
    scope: Option<&ReviewScope>,
) -> anyhow::Result<(bool, Vec<String>)> {
    let artifacts = aesthetic_artifacts_dir(root);
    let review_path = artifacts.join("review.json");
    let manifest_path = artifacts.join("manifest.json");
    let mut lines = vec![match scope {
        Some(s) => format!(
            "Aesthetic review validation (J3 scoped to {})",
            s.current_page
        ),
        None => "Aesthetic review validation (J3 + J4)".to_string(),
    }];
    let mut errors: Vec<String> = Vec::new();

    if !review_path.exists() {
        errors.push(format!(
            "missing {} ? run aesthetic-metrics first, then agent fills review.json",
            review_path.display()
        ));
    }
    if !manifest_path.exists() {
        errors.push("missing manifest.json ? run aesthetic-metrics first".to_string());
    }

    if let Ok(text) = std::fs::read_to_string(&manifest_path) {
        if let Ok(manifest) = serde_json::from_str::<Value>(&text) {
            if manifest.get("status").and_then(|s| s.as_str()) != Some("pass") {
                errors.push(format!(
                    "manifest status is {:?} (expected pass)",
                    manifest.get("status")
                ));
            }
        }
    }

    if review_path.exists() {
        let text = std::fs::read_to_string(&review_path)?;
        let review: Value = serde_json::from_str(&text)?;

        let pages_to_check: Vec<&str> = match scope {
            Some(s) => {
                let mut list: Vec<&str> = s.completed_pages.iter().map(|p| p.as_str()).collect();
                if !list.iter().any(|p| *p == s.current_page.as_str()) {
                    list.push(s.current_page.as_str());
                }
                list
            }
            None => REQUIRED_PAGES.to_vec(),
        };

        if scope.is_none() || scope.is_some_and(|s| s.require_global_j4) {
            if review.get("pass").and_then(|v| v.as_bool()) != Some(true) {
                errors.push("review.pass is false (set true when all criteria met)".to_string());
            }
        }

        let pages = review.get("pages").and_then(|p| p.as_object());
        for page in pages_to_check {
            let Some(pages) = pages else {
                errors.push("missing pages object in review.json".to_string());
                break;
            };
            let Some(entry) = pages.get(page) else {
                errors.push(format!("missing page entry: {page}"));
                continue;
            };
            validate_page_entry(page, entry, &mut errors);
        }

        if scope.is_none() || scope.is_some_and(|s| s.require_global_j4) {
            if let Some(j4) = review.get("global_j4") {
                for key in J4_KEYS {
                    if j4.get(*key).and_then(|v| v.as_bool()) != Some(true) {
                        errors.push(format!("global_j4.{key} is not true"));
                    }
                }
            } else {
                errors.push("missing global_j4".to_string());
            }
        }
    }

    if errors.is_empty() {
        lines.push(match scope {
            Some(s) if !s.require_global_j4 => format!(
                "PASS: page '{}' meets J3 (scores ge 4, no P0)",
                s.current_page
            ),
            _ => "PASS: aesthetic review OK (all pages ge 4, no P0, J4 complete)".to_string(),
        });
        Ok((true, lines))
    } else {
        lines.push("FAIL: review.json validation".to_string());
        for e in &errors {
            lines.push(format!("  - {e}"));
        }
        Ok((false, lines))
    }
}

fn validate_page_entry(page: &str, entry: &Value, errors: &mut Vec<String>) {
    for dim in DIMENSIONS {
        let score = entry
            .get("scores")
            .and_then(|s| s.get(*dim))
            .and_then(|v| v.as_i64());
        match score {
            None => errors.push(format!("{page} : missing score {dim}")),
            Some(s) if s < 4 => errors.push(format!("{page} : {dim} = {s} (need ge 4)")),
            _ => {}
        }
    }
    if let Some(p0) = entry.get("p0").and_then(|v| v.as_array()) {
        for item in p0 {
            if let Some(s) = item.as_str() {
                if s == "not-reviewed" {
                    errors.push(format!("{page} : not reviewed yet"));
                } else {
                    errors.push(format!("{page} : p0 item '{s}'"));
                }
            }
        }
    }
}
