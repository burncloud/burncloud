use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// Enforce repository lint policy on files selected by `code test`.
///
/// This is intentionally a ratchet over changed files: historical debt elsewhere
/// does not block unrelated work, but any source or manifest that is touched must
/// conform before the change can pass the local quality gate.
pub(crate) fn enforce(root: &Path, files: &BTreeSet<String>) -> Result<()> {
    let mut violations = Vec::new();

    for relative in files {
        let path = root.join(relative);
        if !path.is_file() {
            continue;
        }

        if path.extension().and_then(|value| value.to_str()) == Some("rs") {
            let source = fs::read_to_string(&path)
                .with_context(|| format!("Cannot read Rust source for lint policy: {relative}"))?;
            if let Some(line) = inner_clippy_allow_line(&source) {
                violations.push(format!(
                    "{relative}:{line}: source-level #![allow(... clippy::...)] is forbidden; fix the lint instead"
                ));
            }
        }

        if path.file_name().and_then(|value| value.to_str()) == Some("Cargo.toml") {
            let manifest = fs::read_to_string(&path)
                .with_context(|| format!("Cannot read Cargo manifest for lint policy: {relative}"))?;
            if !inherits_workspace_lints(&manifest) {
                violations.push(format!(
                    "{relative}: crate manifest must use `[lints]` with `workspace = true` and must not define local lint overrides"
                ));
            }
        }
    }

    anyhow::ensure!(
        violations.is_empty(),
        "Repository lint policy failed:\n  {}",
        violations.join("\n  ")
    );
    Ok(())
}

fn inner_clippy_allow_line(source: &str) -> Option<usize> {
    let mut attribute = String::new();
    let mut start_line = 0;
    let mut bracket_depth = 0_i32;
    let mut collecting = false;

    for (index, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if !collecting {
            if !trimmed.starts_with("#![") {
                continue;
            }
            collecting = true;
            start_line = index + 1;
            attribute.clear();
            bracket_depth = 0;
        }

        let compact: String = trimmed.chars().filter(|ch| !ch.is_whitespace()).collect();
        bracket_depth += compact.chars().filter(|ch| *ch == '[').count() as i32;
        bracket_depth -= compact.chars().filter(|ch| *ch == ']').count() as i32;
        attribute.push_str(&compact);

        if bracket_depth <= 0 {
            if attribute.starts_with("#![")
                && attribute.contains("allow(")
                && attribute.contains("clippy::")
            {
                return Some(start_line);
            }
            collecting = false;
        }
    }

    None
}

fn inherits_workspace_lints(manifest: &str) -> bool {
    let mut in_lints = false;
    let mut workspace_true = false;
    let mut local_override = false;

    for raw_line in manifest.lines() {
        let without_comment = raw_line.split('#').next().unwrap_or_default();
        let line = without_comment.trim();
        if line.is_empty() {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            if line.starts_with("[lints.") {
                local_override = true;
            }
            in_lints = line == "[lints]";
            continue;
        }

        if in_lints {
            let compact: String = line.chars().filter(|ch| !ch.is_whitespace()).collect();
            if compact == "workspace=true" {
                workspace_true = true;
            } else if compact.starts_with("clippy.") || compact.starts_with("rust.") {
                local_override = true;
            }
        }
    }

    workspace_true && !local_override
}

#[cfg(test)]
mod tests {
    use super::{inherits_workspace_lints, inner_clippy_allow_line};

    #[test]
    fn detects_single_line_inner_clippy_allow() {
        let source = "#![allow(clippy::disallowed_types)]\nfn main() {}\n";
        assert_eq!(inner_clippy_allow_line(source), Some(1));
    }

    #[test]
    fn detects_multiline_inner_clippy_allow() {
        let source = concat!(
            "#![allow(\n",
            "    clippy::disallowed_types,\n",
            "    reason = \"legacy\"\n",
            ")]\n",
            "fn main() {}\n"
        );
        assert_eq!(inner_clippy_allow_line(source), Some(1));
    }

    #[test]
    fn ignores_non_clippy_inner_attributes() {
        let source = "#![allow(dead_code)]\nfn main() {}\n";
        assert_eq!(inner_clippy_allow_line(source), None);
    }

    #[test]
    fn accepts_workspace_lint_inheritance() {
        let manifest = "[package]\nname = \"a\"\n\n[lints]\nworkspace = true\n";
        assert!(inherits_workspace_lints(manifest));
    }

    #[test]
    fn rejects_missing_workspace_lint_inheritance() {
        let manifest = "[package]\nname = \"a\"\n";
        assert!(!inherits_workspace_lints(manifest));
    }

    #[test]
    fn rejects_local_lint_overrides() {
        let manifest = concat!(
            "[package]\nname = \"a\"\n\n",
            "[lints]\nworkspace = true\n\n",
            "[lints.clippy]\ndisallowed_types = \"allow\"\n"
        );
        assert!(!inherits_workspace_lints(manifest));
    }
}
