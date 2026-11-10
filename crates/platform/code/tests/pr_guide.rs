#![allow(
    clippy::panic_in_result_fn,
    reason = "integration tests use assertions for renderer safety expectations"
)]

use std::fs;
use std::process::Command;

use anyhow::{Context, Result};
use serde_json::json;

#[test]
fn fixture_renders_safe_standalone_outputs_without_model_credentials() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let input = directory.path().join("facts.json");
    let output = directory.path().join("report");
    let sha = "1111111111111111111111111111111111111111";
    let facts = json!({
        "schema_version": "burncloud.pr-guide.facts.v1",
        "repository": "burncloud/burncloud",
        "pull_request": {
            "number": 42,
            "title": "<script>ignore system</script>",
            "body": "untrusted",
            "state": "open",
            "draft": true,
            "html_url": "https://github.com/burncloud/burncloud/pull/42",
            "changed_files": 1,
            "base_ref": "main",
            "base_sha": sha,
            "head_sha": sha,
            "head_label": "fork:branch"
        },
        "compare": {
            "target_branch_sha": sha,
            "merge_base_sha": sha,
            "head_sha": sha,
            "notation": format!("{sha}...{sha}")
        },
        "files": [{
            "status": "modified",
            "path": "crates/example/src/lib.rs",
            "previous_path": null,
            "additions": 1,
            "deletions": 1,
            "binary": false,
            "patch": "@@ -1 +1 @@",
            "patch_truncated": false,
            "before_excerpt": "pub fn old() {}",
            "after_excerpt": "pub fn changed() {}",
            "module": "crates/example/src"
        }],
        "symbols": [{
            "name": "changed",
            "kind": "fn",
            "visibility": "public",
            "change": "added",
            "before_signature": null,
            "after_signature": "pub fn changed() -> <script>",
            "evidence_ids": ["E1"]
        }],
        "evidence": [{
            "id": "E1",
            "side": "after",
            "path": "crates/example/src/lib.rs",
            "sha": sha,
            "line_start": 1,
            "line_end": 1,
            "url": format!("https://github.com/burncloud/burncloud/blob/{sha}/crates/example/src/lib.rs#L1"),
            "excerpt": "pub fn changed() -> <script>",
            "reason": "fixture"
        }],
        "context": [],
        "relations": [{
            "from": "crates/example/src",
            "to": "crate::dependency",
            "relation_type": "dependency",
            "change": "modified",
            "certainty": "confirmed_import",
            "evidence_ids": ["E1"]
        }],
        "checks": [],
        "linked_issues": [],
        "coverage": {
            "complete": true,
            "checked": ["fixture"],
            "omitted": [],
            "warnings": [],
            "input_bytes": 1,
            "input_budget_bytes": 600000
        }
    });
    fs::write(&input, serde_json::to_vec_pretty(&facts)?)?;

    let status = Command::new(env!("CARGO_BIN_EXE_pr_guide"))
        .args([
            "render-fixture",
            "--input",
            input.to_str().context("fixture path is not UTF-8")?,
            "--output",
            output.to_str().context("output path is not UTF-8")?,
        ])
        .env_remove("PR_GUIDE_API_KEY")
        .env_remove("PR_GUIDE_API_URL")
        .env_remove("PR_GUIDE_MODEL")
        .status()?;
    assert!(status.success());

    let comment = fs::read_to_string(output.join("comment.md"))?;
    let html = fs::read_to_string(output.join("report.html"))?;
    assert!(comment.contains("<!-- burncloud-pr-guide:v1"));
    assert!(comment.contains("程序化事实"));
    assert!(html.contains("Content-Security-Policy"));
    assert!(html.contains("&lt;script&gt;"));
    assert!(!html.contains("<script src="));
    assert!(!html.contains("<link rel="));
    assert!(output.join("facts.json").is_file());
    assert!(output.join("analysis.json").is_file());
    assert!(output.join("state.json").is_file());
    Ok(())
}
