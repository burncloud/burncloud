use std::path::Path;

use regex::Regex;
use walkdir::WalkDir;

use crate::paths::client_crate_dir;

const SCAN_DIRS: &[&str] = &[
    "crates/client-shared/src",
    "crates/client-access/src",
    "crates/client-connect/src",
    "crates/client-log/src",
    "crates/client-models/src",
    "crates/client-monitor/src",
    "crates/client-playground/src",
    "crates/client-settings/src",
    "crates/client-users/src",
    "src",
];

pub fn run_ui_conventions(root: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let client = client_crate_dir(root);
    let patterns: &[(&str, &str)] = &[
        (
            "Raw button with btn-* class - use BCButton",
            r#"button\s*\{[^}]*class:\s*"btn-(primary|secondary|danger|ghost|black)"#,
        ),
        (
            "BCButton duplicates variant in class prop",
            r#"BCButton[^}]*class:\s*"(btn-primary|btn-secondary|btn-danger|btn-ghost|btn-black)"#,
        ),
    ];

    let compiled: Vec<(&str, Regex)> = patterns
        .iter()
        .map(|(label, pat)| {
            Regex::new(pat)
                .map(|re| (*label, re))
                .map_err(|e| anyhow::anyhow!("bad ui convention regex: {e}"))
        })
        .collect::<Result<_, _>>()?;

    let mut lines = Vec::new();
    let mut violations = 0usize;

    for dir in SCAN_DIRS {
        let scan_root = client.join(dir);
        if !scan_root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&scan_root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
        {
            let path = entry.path();
            let content = std::fs::read_to_string(path)?;
            for (label, re) in &compiled {
                if re.is_match(&content) {
                    violations += 1;
                    lines.push(format!("::error::{label}"));
                    lines.push(format!("{}: matched", path.display()));
                }
            }
        }
    }

    if violations > 0 {
        lines.push(format!(
            "Found {violations} UI convention violation(s). See docs/ui/components.md"
        ));
        Ok((false, lines))
    } else {
        Ok((true, lines))
    }
}
