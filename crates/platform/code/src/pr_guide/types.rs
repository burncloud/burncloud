use serde::{Deserialize, Serialize};

pub(crate) const COMMENT_MARKER: &str = "<!-- burncloud-pr-guide:v1";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct PullRequest {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) body: String,
    pub(crate) state: String,
    pub(crate) draft: bool,
    pub(crate) html_url: String,
    pub(crate) changed_files: u64,
    pub(crate) base_ref: String,
    pub(crate) base_sha: String,
    pub(crate) head_sha: String,
    pub(crate) head_label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CompareRange {
    pub(crate) target_branch_sha: String,
    pub(crate) merge_base_sha: String,
    pub(crate) head_sha: String,
    pub(crate) notation: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct FileFact {
    pub(crate) status: String,
    pub(crate) path: String,
    pub(crate) previous_path: Option<String>,
    pub(crate) additions: Option<u64>,
    pub(crate) deletions: Option<u64>,
    pub(crate) binary: bool,
    pub(crate) patch: Option<String>,
    pub(crate) patch_truncated: bool,
    pub(crate) before_excerpt: Option<String>,
    pub(crate) after_excerpt: Option<String>,
    pub(crate) module: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct SymbolFact {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) visibility: String,
    pub(crate) change: String,
    pub(crate) before_signature: Option<String>,
    pub(crate) after_signature: Option<String>,
    pub(crate) evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Evidence {
    pub(crate) id: String,
    pub(crate) side: String,
    pub(crate) path: String,
    pub(crate) sha: String,
    pub(crate) line_start: u64,
    pub(crate) line_end: u64,
    pub(crate) url: String,
    pub(crate) excerpt: String,
    pub(crate) reason: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct ContextFact {
    pub(crate) kind: String,
    pub(crate) path: String,
    pub(crate) side: String,
    pub(crate) reason: String,
    pub(crate) excerpt: String,
    pub(crate) evidence_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct CheckFact {
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) conclusion: Option<String>,
    pub(crate) details_url: Option<String>,
    pub(crate) head_sha: String,
    pub(crate) completed_at: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct LinkedIssue {
    pub(crate) number: u64,
    pub(crate) title: String,
    pub(crate) state: String,
    pub(crate) html_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RelationFact {
    pub(crate) from: String,
    pub(crate) to: String,
    pub(crate) relation_type: String,
    pub(crate) change: String,
    pub(crate) certainty: String,
    pub(crate) evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct Coverage {
    pub(crate) complete: bool,
    pub(crate) checked: Vec<String>,
    pub(crate) omitted: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) input_bytes: usize,
    pub(crate) input_budget_bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct FactBundle {
    pub(crate) schema_version: String,
    pub(crate) repository: String,
    pub(crate) pull_request: PullRequest,
    pub(crate) compare: CompareRange,
    pub(crate) files: Vec<FileFact>,
    pub(crate) symbols: Vec<SymbolFact>,
    pub(crate) evidence: Vec<Evidence>,
    pub(crate) context: Vec<ContextFact>,
    pub(crate) relations: Vec<RelationFact>,
    pub(crate) checks: Vec<CheckFact>,
    pub(crate) linked_issues: Vec<LinkedIssue>,
    pub(crate) coverage: Coverage,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Analysis {
    pub(crate) purpose: String,
    pub(crate) before_after: Vec<BeforeAfter>,
    pub(crate) impact_groups: Vec<ImpactGroup>,
    pub(crate) api_changes: Vec<ApiChange>,
    pub(crate) relations: Vec<RelationFact>,
    pub(crate) diagram_kind: String,
    pub(crate) reading_order: Vec<ReadingStep>,
    pub(crate) risks: Vec<Risk>,
    pub(crate) verification_notes: Vec<Claim>,
    pub(crate) unknowns: Vec<Claim>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct BeforeAfter {
    pub(crate) condition: String,
    pub(crate) before: String,
    pub(crate) after: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) certainty: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct ImpactGroup {
    pub(crate) name: String,
    pub(crate) changed: String,
    pub(crate) reason: String,
    pub(crate) callers: String,
    pub(crate) evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct ApiChange {
    pub(crate) symbol: String,
    pub(crate) change: String,
    pub(crate) compatibility: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) certainty: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct ReadingStep {
    pub(crate) path: String,
    pub(crate) reason: String,
    pub(crate) evidence_id: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Risk {
    pub(crate) topic: String,
    pub(crate) detail: String,
    pub(crate) certainty: String,
    pub(crate) evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub(crate) struct Claim {
    pub(crate) text: String,
    pub(crate) certainty: String,
    pub(crate) evidence_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct RunState {
    pub(crate) reused: bool,
    pub(crate) fingerprint: String,
    pub(crate) head_sha: String,
    pub(crate) analysis_status: String,
    pub(crate) model_status: String,
    pub(crate) model_name: Option<String>,
    pub(crate) comment_id: Option<u64>,
    pub(crate) artifact_name: String,
    pub(crate) generated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(crate) struct MarkerState {
    pub(crate) fingerprint: String,
    pub(crate) head_sha: String,
    pub(crate) status: String,
}
