use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct LoopState {
    #[serde(rename = "loop")]
    pub loop_name: String,
    pub iteration: u32,
    pub max_iterations: u32,
    pub phase: String,
    pub css_ok: bool,
    pub metrics_ok: bool,
    pub review_ok: bool,
    pub next_action: String,
    pub agent_prompt: String,
    pub fast_mode: bool,
    pub preview_routes: bool,
    pub pages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_page: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub completed_pages: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pages_remaining: Option<usize>,
    pub updated_at: String,
}

pub fn write_loop_state(path: &Path, state: &LoopState) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn phase_from_gates(css_ok: bool, metrics_ok: bool, review_ok: bool) -> &'static str {
    if review_ok {
        "done"
    } else if !css_ok {
        "css"
    } else if !metrics_ok {
        "metrics"
    } else {
        "review"
    }
}

pub fn next_action_from_phase(phase: &str) -> &'static str {
    match phase {
        "css" => "fix-css",
        "metrics" => "fix-metrics-and-layout",
        "review" => "fix-review-scores",
        "done" => "none",
        _ => "increase-MaxIterations-or-fix-manually",
    }
}
