//! Evidence chain helper for black-box test suites.
//!
//! Writes structured JSON artifacts to `evidence/{test_name}.json`.
//! File write failures are non-fatal — they emit a warning and continue,
//! so that test assertion failures are never masked by I/O issues.

use chrono::Utc;
use serde_json::{json, Value};
use std::path::Path;

/// Write a structured evidence artifact to `evidence/{test_name}.json`.
///
/// The directory is created if it does not exist.
/// Any I/O failure is logged with `eprintln!` but does NOT panic.
pub fn write_evidence(test_name: &str, payload: &Value) {
    let dir = Path::new("evidence");
    if let Err(e) = std::fs::create_dir_all(dir) {
        eprintln!("[EVIDENCE] warn: cannot create evidence/ dir: {e}");
        return;
    }
    let path = dir.join(format!("{test_name}.json"));
    match serde_json::to_string_pretty(payload) {
        Ok(s) => {
            if let Err(e) = std::fs::write(&path, s) {
                eprintln!("[EVIDENCE] warn: cannot write {path:?}: {e}");
            } else {
                println!("[EVIDENCE] written → {}", path.display());
            }
        }
        Err(e) => eprintln!("[EVIDENCE] warn: json serialization failed: {e}"),
    }
}

/// Build the base evidence object every test starts with.
pub fn base_evidence(test_name: &str, model: &str) -> Value {
    let commit = std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string();

    json!({
        "test": test_name,
        "timestamp": Utc::now().to_rfc3339(),
        "git_commit": commit,
        "model": model,
    })
}

/// Append a named assertion result to an evidence object's `assertions` array.
pub fn add_assertion(evidence: &mut Value, name: &str, expected: i64, actual: i64) {
    let result = if (actual - expected).abs() <= 1 {
        "PASS"
    } else {
        "FAIL"
    };
    let assertions = evidence
        .get_mut("assertions")
        .and_then(|a| a.as_array_mut());

    let entry = json!({
        "name": name,
        "expected": expected,
        "actual": actual,
        "delta": actual - expected,
        "result": result,
    });

    if let Some(arr) = assertions {
        arr.push(entry);
    } else {
        evidence["assertions"] = json!([entry]);
    }
}

/// Set the top-level verdict based on whether all assertions passed.
pub fn finalize_verdict(evidence: &mut Value) {
    let all_pass = evidence
        .get("assertions")
        .and_then(|a| a.as_array())
        .map(|arr| {
            arr.iter()
                .all(|e| e.get("result").and_then(|r| r.as_str()) == Some("PASS"))
        })
        .unwrap_or(true); // no assertions = trivially pass

    evidence["verdict"] = json!(if all_pass { "PASS" } else { "FAIL" });
}
