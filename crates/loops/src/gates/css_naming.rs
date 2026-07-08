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

const GUEST_PAGES: &[&str] = &[
    "src/pages/home.rs",
    "src/pages/login.rs",
    "src/pages/forgot_password.rs",
    "src/pages/reset_password.rs",
];

struct CheckRule {
    label: &'static str,
    pattern: &'static str,
    skip_numeric_zero: bool,
    require_non_identifier_prefix: bool,
}

const CHECK_RULES: &[CheckRule] = &[
    CheckRule {
        label: "[A1] Legacy spacing short name (09_*.css) - use *-bc-* per docs/ui/naming.md SS5",
        pattern: r"\b(gap-xs|gap-sm|gap-md|gap-lg|gap-xl|gap-xxl|gap-xxxl|p-xs|p-sm|p-md|p-lg|p-xl|p-xxl|p-xxxl|m-xs|m-sm|m-md|m-lg|m-xl|m-xxl|m-xxxl|mb-xs|mb-sm|mb-md|mb-lg|mb-xl|mb-xxl|mb-xxxl|mt-xs|mt-sm|mt-md|mt-lg|mt-xl|mt-xxl|mt-xxxl|ml-xs|ml-sm|ml-md|ml-lg|mr-xs|mr-sm|mr-md|mr-lg|mx-xs|mx-sm|mx-md|mx-lg|my-xs|my-sm|my-md|my-lg|px-xs|px-sm|px-md|px-lg|px-xxl|px-xxxl|py-xs|py-sm|py-md|py-lg|py-xxl|py-xxxl|pl-xs|pl-sm|pl-md|pl-lg|pr-xs|pr-sm|pr-md|pr-lg|pt-xs|pt-sm|pt-md|pt-lg|pb-xs|pb-sm|pb-md|pb-lg)\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[A2] Legacy utility (25_*.css) bc-gap-* / bc-pl-* - use gap-bc-* per naming.md",
        pattern: r"\bbc-(gap|pl|pr|pt|pb|mt|mb|ml|mr|mx|my|px|py)-[0-9]+\b|\bbc-text-brand\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[A3] Tailwind numeric spacing - use *-bc-* (m-0/mb-0/p-0 allowed)",
        pattern: r"\b(gap|p|m|mb|mt|ml|mr|mx|my|px|py|pl|pr|pt|pb)-([0-9]+)\b",
        skip_numeric_zero: true,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[B1] border-[var(--bc-border)] - use border-bc-border",
        pattern: r"border-\[var\(--bc-border\)\]",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[B2] shadcn class in console - use *-bc-* per naming.md SS2",
        pattern: r"\b(text-muted-foreground|bg-muted|text-foreground|bg-background|bg-card|border-border)\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[B3] Tailwind default gray palette - use text-bc-* / bg-bc-*",
        pattern: r"\b(text|bg|border|ring|divide|from|to|via)-(gray|slate|zinc|neutral|stone)-[0-9]+\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[C1] Tailwind rounded-md/lg/xl - use rounded-bc-sm|md|lg",
        pattern: r"\brounded-(sm|md|lg|xl)\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[C2] Tailwind shadow-sm - use shadow-bc-sm",
        pattern: r"\bshadow-sm\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[C3] Arbitrary rounded/shadow - use rounded-bc-* / shadow-bc-* or component CSS",
        pattern: r"\b(rounded|shadow)-\[[^\]]+\]",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[D1] Tailwind default text size - use text-title|text-body|text-caption|text-bc-*",
        pattern: r"text-(2xl|3xl|4xl|5xl|6xl|base|lg)\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: true,
    },
    CheckRule {
        label: "[D2] Legacy text-xxs - use text-caption or text-bc-sm",
        pattern: r"\btext-xxs\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[D3] Legacy text-display - use text-large-title",
        pattern: r"\btext-display\b",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
    CheckRule {
        label: "[D4] Arbitrary text-[Npx] - use text-caption or text-bc-sm",
        pattern: r"\btext-\[[0-9]+px\]",
        skip_numeric_zero: false,
        require_non_identifier_prefix: false,
    },
];

pub fn run_css_naming(root: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let client = client_crate_dir(root);
    let mut lines = vec![
        "CSS optimize acceptance (see crates/loops/acceptance/css-optimize-acceptance.md)".to_string(),
        String::new(),
    ];
    let mut violations = 0usize;

    let rules: Vec<(&CheckRule, Regex)> = CHECK_RULES
        .iter()
        .map(|r| {
            Regex::new(r.pattern)
                .map(|re| (r, re))
                .map_err(|e| anyhow::anyhow!("bad rule regex {}: {e}", r.label))
        })
        .collect::<Result<_, _>>()?;

    fn line_matches_rule(line: &str, rule: &CheckRule, re: &Regex) -> bool {
        let Some(mat) = re.find(line) else {
            return false;
        };
        if rule.skip_numeric_zero {
            if let Some(caps) = re.captures(line) {
                if caps.get(2).is_some_and(|m| m.as_str() == "0") {
                    return false;
                }
            }
        }
        if rule.require_non_identifier_prefix {
            let start = mat.start();
            if start > 0 {
                let prev = line.as_bytes()[start - 1];
                if prev.is_ascii_alphanumeric() || prev == b'-' || prev == b'_' {
                    return false;
                }
            }
        }
        true
    }

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
            let rel = path
                .strip_prefix(&client)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");
            if GUEST_PAGES.contains(&rel.as_str()) {
                continue;
            }

            let content = std::fs::read_to_string(path)?;
            for (line_num, line) in content.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                if !line.contains("class:") {
                    continue;
                }
                for (rule, re) in &rules {
                    if !line_matches_rule(line, rule, re) {
                        continue;
                    }
                    let label = rule.label;
                    violations += 1;
                    if !lines.iter().any(|l| l.starts_with("::error::") && l.contains(label)) {
                        lines.push(format!("::error::{label}"));
                    }
                    lines.push(format!(
                        "{}:{}:{}",
                        rel,
                        line_num + 1,
                        line.trim()
                    ));
                }
            }
        }
    }

    let (ui_ok, ui_lines) = super::ui_conventions::run_ui_conventions(root)?;
    if !ui_ok {
        violations += 1;
        lines.extend(ui_lines);
    } else {
        lines.push("Console UI button conventions OK".to_string());
    }

    if violations > 0 {
        lines.push(String::new());
        lines.push(format!(
            "FAIL: {violations} issue(s). Fix per crates/loops/acceptance/css-optimize-acceptance.md"
        ));
        Ok((false, lines))
    } else {
        lines.push(String::new());
        lines.push("PASS: Console CSS naming OK (all acceptance rules)".to_string());
        Ok((true, lines))
    }
}
