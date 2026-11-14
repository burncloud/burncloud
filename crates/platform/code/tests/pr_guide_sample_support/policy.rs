use super::types::{AnalysisMode, GuideRequest, ReportPlan};

const DRAFT_SECTIONS: &[&str] = &["一句话目的", "影响范围", "变更关系图", "验证情况"];
const READY_SECTIONS: &[&str] = &[
    "一句话目的",
    "修改前后对比",
    "影响范围",
    "变更关系图",
    "风险与待确认事项",
    "验证情况",
];
const EMPTY_SECTIONS: &[&str] = &["一句话目的", "验证情况"];

/// 根据 PR 状态和模型可用性选择报告计划。
pub(crate) fn select_plan(request: GuideRequest) -> ReportPlan {
    let mode = if request.model_available {
        AnalysisMode::ModelEnhanced
    } else {
        AnalysisMode::Deterministic
    };
    let sections = if request.changed_files == 0 {
        EMPTY_SECTIONS
    } else if request.draft {
        DRAFT_SECTIONS
    } else {
        READY_SECTIONS
    };
    let warning = (!request.model_available).then_some("模型不可用，保留确定性事实摘要");

    ReportPlan {
        mode,
        sections,
        warning,
    }
}
