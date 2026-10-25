mod pr_guide_sample_support;

use pr_guide_sample_support::{select_plan, AnalysisMode, GuideRequest};

#[test]
fn draft_without_model_keeps_a_deterministic_report() {
    let plan = select_plan(GuideRequest {
        draft: true,
        model_available: false,
        changed_files: 3,
    });

    assert_eq!(plan.mode, AnalysisMode::Deterministic);
    assert!(plan.sections.contains(&"变更关系图"));
    assert_eq!(plan.warning, Some("模型不可用，保留确定性事实摘要"));
}

#[test]
fn ready_pr_with_model_enables_semantic_guidance() {
    let plan = select_plan(GuideRequest {
        draft: false,
        model_available: true,
        changed_files: 2,
    });

    assert_eq!(plan.mode, AnalysisMode::ModelEnhanced);
    assert!(plan.sections.contains(&"风险与待确认事项"));
    assert_eq!(plan.warning, None);
}

#[test]
fn empty_change_set_uses_the_minimum_report() {
    let plan = select_plan(GuideRequest {
        draft: false,
        model_available: false,
        changed_files: 0,
    });

    assert_eq!(plan.mode, AnalysisMode::Deterministic);
    assert_eq!(plan.sections, ["一句话目的", "验证情况"]);
}
