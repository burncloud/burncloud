mod aesthetic_review;
mod cargo_test;
mod css_naming;
mod ui_conventions;

use std::path::Path;

pub use aesthetic_review::{run_aesthetic_review, run_aesthetic_review_scoped, ReviewScope};
pub use cargo_test::{ensure_api_tests_built, run_aesthetic_metrics, run_css_visual, timings_from_log_lines};
pub use css_naming::run_css_naming;

/// Gate categories for agent loops. Each maps to one acceptance layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateCategory {
    /// Static CSS / naming rules (A1–E1).
    CssNaming,
    /// Naming + 11-page visual (full css gate).
    CssAll,
    /// Screenshot + layout JS rules (V1–V4), 11 console pages.
    CssVisual,
    /// Browser capture + J1 pixel metrics, 9 aesthetic pages.
    AestheticMetrics,
    /// Human/agent review.json validation (J3 + J4).
    AestheticReview,
}

impl GateCategory {
    pub const JOBS_AESTHETIC_PAGES: &'static [&'static str] = &[
        "aesthetic-home",
        "aesthetic-login",
        "aesthetic-dashboard",
        "aesthetic-models",
        "aesthetic-access",
        "aesthetic-settings",
        "aesthetic-finance",
        "aesthetic-monitor",
        "aesthetic-playground",
    ];

    pub fn log_label(self) -> String {
        match self {
            Self::CssNaming => "gate:css-naming".to_string(),
            Self::CssAll => "gate:css-all".to_string(),
            Self::CssVisual => "gate:css-visual".to_string(),
            Self::AestheticMetrics => "gate:aesthetic-metrics".to_string(),
            Self::AestheticReview => "gate:aesthetic-review".to_string(),
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Self::CssNaming => "Console CSS naming (docs/ui/naming.md + BCButton conventions)",
            Self::CssAll => "CSS naming + 11-page visual acceptance",
            Self::CssVisual => "CSS visual acceptance (11 pages, screenshots + layout JS)",
            Self::AestheticMetrics => "Aesthetic J1 metrics (9 pages, preview mock routes)",
            Self::AestheticReview => "Aesthetic J3/J4 review.json scores",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GateRunResult {
    pub category: GateCategory,
    pub passed: bool,
    pub lines: Vec<String>,
    pub elapsed_secs: f64,
}

pub struct JobsFastGates {
    pub css_ok: bool,
    pub metrics_ok: bool,
    pub review_ok: bool,
    pub results: Vec<GateRunResult>,
}

pub fn run_css_all(root: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let (naming_ok, mut lines) = run_css_naming(root)?;
    if !naming_ok {
        return Ok((false, lines));
    }
    let (visual_ok, visual_lines) = run_css_visual(root)?;
    lines.extend(visual_lines);
    Ok((visual_ok, lines))
}

/// Full aesthetic acceptance: css-all + metrics + review.
pub fn run_aesthetic_full(root: &Path) -> anyhow::Result<(bool, Vec<String>)> {
    let (css_ok, mut lines) = run_css_all(root)?;
    if !css_ok {
        lines.insert(0, "FAIL: CSS baseline (naming + visual)".to_string());
        return Ok((false, lines));
    }
    let (metrics_ok, m_lines) = run_aesthetic_metrics(root, None)?;
    lines.extend(m_lines);
    if !metrics_ok {
        return Ok((false, lines));
    }
    let (review_ok, r_lines) = run_aesthetic_review(root)?;
    lines.extend(r_lines);
    Ok((review_ok, lines))
}

/// Fast path: naming + aesthetic metrics + review (no 11-page css_visual).
pub fn run_jobs_fast_gates(
    root: &Path,
    full_css: bool,
    review_scope: Option<&ReviewScope>,
    logger: &mut crate::log::LoopLogger,
) -> JobsFastGates {
    let mut results = Vec::new();

    let css_gate = if full_css {
        GateCategory::CssAll
    } else {
        GateCategory::CssNaming
    };

    let (css_ok, lines, elapsed) = logger.timed_gate(css_gate, || {
        if full_css {
            run_css_all(root)
        } else {
            run_css_naming(root)
        }
    });
    results.push(GateRunResult {
        category: css_gate,
        passed: css_ok,
        lines,
        elapsed_secs: elapsed,
    });

    if !css_ok {
        return JobsFastGates {
            css_ok: false,
            metrics_ok: false,
            review_ok: false,
            results,
        };
    }

    let (metrics_ok, lines, elapsed) = logger.timed_gate(GateCategory::AestheticMetrics, || {
        let focus = review_scope.map(|s| s.current_page.as_str());
        run_aesthetic_metrics(root, focus)
    });
    results.push(GateRunResult {
        category: GateCategory::AestheticMetrics,
        passed: metrics_ok,
        lines,
        elapsed_secs: elapsed,
    });

    if !metrics_ok {
        return JobsFastGates {
            css_ok: true,
            metrics_ok: false,
            review_ok: false,
            results,
        };
    }

    let (review_ok, lines, elapsed) = logger.timed_gate(GateCategory::AestheticReview, || {
        run_aesthetic_review_scoped(root, review_scope)
    });
    results.push(GateRunResult {
        category: GateCategory::AestheticReview,
        passed: review_ok,
        lines,
        elapsed_secs: elapsed,
    });

    JobsFastGates {
        css_ok: true,
        metrics_ok: true,
        review_ok,
        results,
    }
}

/// Run a single gate (`burncloud-loop gate <name>`).
pub fn run_single_gate(root: &Path, gate: GateCategory) -> anyhow::Result<(bool, Vec<String>)> {
    match gate {
        GateCategory::CssNaming => run_css_naming(root),
        GateCategory::CssAll => run_css_all(root),
        GateCategory::CssVisual => run_css_visual(root),
        GateCategory::AestheticMetrics => run_aesthetic_metrics(root, None),
        GateCategory::AestheticReview => run_aesthetic_review(root),
    }
}

pub fn parse_gate_name(name: &str) -> anyhow::Result<GateCategory> {
    match name {
        "css-naming" | "naming" => Ok(GateCategory::CssNaming),
        "css-all" | "all" => Ok(GateCategory::CssAll),
        "css-visual" | "visual" => Ok(GateCategory::CssVisual),
        "aesthetic-metrics" | "metrics" => Ok(GateCategory::AestheticMetrics),
        "aesthetic-review" | "review" => Ok(GateCategory::AestheticReview),
        other => anyhow::bail!(
            "unknown gate '{other}'. Use: css-naming, css-all, css-visual, aesthetic-metrics, aesthetic-review"
        ),
    }
}

pub fn parse_gate_suite(name: &str) -> anyhow::Result<GateSuite> {
    match name {
        "jobs-fast" => Ok(GateSuite::JobsFast),
        "jobs-full" => Ok(GateSuite::JobsFull),
        "css-full" => Ok(GateSuite::CssFull),
        "aesthetic-full" => Ok(GateSuite::AestheticFull),
        other => anyhow::bail!(
            "unknown suite '{other}'. Use: jobs-fast, jobs-full, css-full, aesthetic-full"
        ),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GateSuite {
    JobsFast,
    JobsFull,
    CssFull,
    AestheticFull,
}

pub fn run_gate_suite(
    root: &Path,
    suite: GateSuite,
    logger: &mut crate::log::LoopLogger,
) -> bool {
    match suite {
        GateSuite::JobsFast => {
            let g = run_jobs_fast_gates(root, false, None, logger);
            g.css_ok && g.metrics_ok && g.review_ok
        }
        GateSuite::JobsFull => {
            let g = run_jobs_fast_gates(root, true, None, logger);
            g.css_ok && g.metrics_ok && g.review_ok
        }
        GateSuite::CssFull => {
            let (ok, _, _) = logger.timed_gate(GateCategory::CssAll, || run_css_all(root));
            ok
        }
        GateSuite::AestheticFull => {
            let (ok, _, _) =
                logger.timed_gate(GateCategory::AestheticReview, || run_aesthetic_full(root));
            ok
        }
    }
}
