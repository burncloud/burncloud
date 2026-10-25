/// 报告的分析能力等级。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AnalysisMode {
    Deterministic,
    ModelEnhanced,
}

/// 影响报告计划的输入事实。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct GuideRequest {
    pub(crate) draft: bool,
    pub(crate) model_available: bool,
    pub(crate) changed_files: usize,
}

/// 渲染器需要执行的确定性计划。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReportPlan {
    pub(crate) mode: AnalysisMode,
    pub(crate) sections: &'static [&'static str],
    pub(crate) warning: Option<&'static str>,
}
