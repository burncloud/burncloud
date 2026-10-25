use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::types::{Analysis, Evidence, FactBundle, MarkerState, RelationFact, RunState};

pub(crate) fn write_outputs(
    output: &Path,
    facts: &FactBundle,
    analysis: &Analysis,
    state: &RunState,
) -> Result<()> {
    fs::create_dir_all(output).with_context(|| format!("failed to create {}", output.display()))?;
    write_json(&output.join("facts.json"), facts)?;
    write_json(&output.join("analysis.json"), analysis)?;
    write_json(&output.join("state.json"), state)?;
    fs::write(output.join("comment.md"), markdown(facts, analysis, state))
        .context("failed to write PR comment")?;
    fs::write(output.join("report.html"), html(facts, analysis, state))
        .context("failed to write HTML report")?;
    Ok(())
}

pub(crate) fn write_reuse_state(output: &Path, state: &RunState) -> Result<()> {
    fs::create_dir_all(output).with_context(|| format!("failed to create {}", output.display()))?;
    write_json(&output.join("state.json"), state)
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(value)?;
    fs::write(path, bytes).with_context(|| format!("failed to write {}", path.display()))
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "the fixed report order is intentionally visible in one Markdown renderer"
)]
fn markdown(facts: &FactBundle, analysis: &Analysis, state: &RunState) -> String {
    let mut output = String::new();
    let marker = MarkerState {
        fingerprint: state.fingerprint.clone(),
        head_sha: state.head_sha.clone(),
        status: state.analysis_status.clone(),
    };
    let marker_json = serde_json::to_string(&marker).unwrap_or_else(|_| "{}".to_owned());
    writeln!(output, "<!-- burncloud-pr-guide:v1 {marker_json} -->").ok();
    writeln!(output, "## PR 结构化导读").ok();
    writeln!(
        output,
        "> {} **PR #{}** · `{}` → `{}` · {} · {}",
        if facts.pull_request.draft {
            "草稿"
        } else {
            "Ready"
        },
        facts.pull_request.number,
        md(&facts.pull_request.base_ref),
        short_sha(&facts.pull_request.head_sha),
        md(&state.analysis_status),
        md(&state.generated_at)
    )
    .ok();
    writeln!(output).ok();
    writeln!(output, "### 1. 一句话目的").ok();
    writeln!(output).ok();
    writeln!(output, "{}", md(&analysis.purpose)).ok();
    writeln!(output).ok();
    writeln!(output, "### 2. 修改前后对比").ok();
    writeln!(output).ok();
    if analysis.before_after.is_empty() {
        writeln!(
            output,
            "未发现可程序化确认的行为或签名对比；请查看详细报告中的验证缺口。"
        )
        .ok();
    } else {
        writeln!(output, "| 条件/输入 | 修改前 | 修改后 | 证据性质 |").ok();
        writeln!(output, "|---|---|---|---|").ok();
        for row in analysis.before_after.iter().take(6) {
            writeln!(
                output,
                "| {} | {} | {} | {} {} |",
                table(&row.condition),
                table(&row.before),
                table(&row.after),
                certainty_label(&row.certainty),
                evidence_links(&row.evidence_ids, &facts.evidence)
            )
            .ok();
        }
    }
    writeln!(output).ok();
    writeln!(output, "### 3. 影响范围").ok();
    writeln!(output).ok();
    for group in analysis.impact_groups.iter().take(6) {
        writeln!(
            output,
            "- **{}**：{} 原因：{} 调用方：{} {}",
            md(&group.name),
            md(&group.changed),
            md(&group.reason),
            md(&group.callers),
            evidence_links(&group.evidence_ids, &facts.evidence)
        )
        .ok();
    }
    if analysis.impact_groups.is_empty() {
        writeln!(output, "- 未形成可靠的业务分组；详细报告保留文件级事实。").ok();
    }
    writeln!(output).ok();
    writeln!(output, "### 4. 接口与数据结构变化").ok();
    writeln!(output).ok();
    if analysis.api_changes.is_empty() {
        writeln!(output, "未发现相关变化。").ok();
    } else {
        for item in analysis.api_changes.iter().take(10) {
            writeln!(
                output,
                "- `{}`：{}；兼容性：{}；{} {}",
                code(&item.symbol),
                md(&item.change),
                md(&item.compatibility),
                certainty_label(&item.certainty),
                evidence_links(&item.evidence_ids, &facts.evidence)
            )
            .ok();
        }
    }
    writeln!(output).ok();
    writeln!(output, "### 5. 变更关系图").ok();
    writeln!(output).ok();
    output.push_str(&mermaid(facts, analysis));
    writeln!(output).ok();
    writeln!(output, "### 6. 建议阅读顺序").ok();
    writeln!(output).ok();
    for (index, step) in analysis.reading_order.iter().take(8).enumerate() {
        let link = step
            .evidence_id
            .as_deref()
            .and_then(|id| facts.evidence.iter().find(|item| item.id == id))
            .map(|item| format!("[{}]({})", md(&step.path), item.url))
            .unwrap_or_else(|| format!("`{}`", code(&step.path)));
        writeln!(output, "{}. {}：{}", index + 1, link, md(&step.reason)).ok();
    }
    if analysis.reading_order.is_empty() {
        writeln!(output, "1. 从变更文件列表开始；当前无法可靠排序。").ok();
    }
    writeln!(output).ok();
    writeln!(output, "<details>").ok();
    writeln!(
        output,
        "<summary><strong>7. 风险、验证情况与完整范围</strong></summary>"
    )
    .ok();
    writeln!(output).ok();
    writeln!(output, "#### 风险与待确认事项").ok();
    if analysis.risks.is_empty() {
        writeln!(
            output,
            "未生成具体风险结论。这不等于未发现风险；当前事实不足或模型不可用时，必须由审查者继续确认。"
        )
        .ok();
    } else {
        for risk in &analysis.risks {
            writeln!(
                output,
                "- **{}**：{}；{} {}",
                md(&risk.topic),
                md(&risk.detail),
                certainty_label(&risk.certainty),
                evidence_links(&risk.evidence_ids, &facts.evidence)
            )
            .ok();
        }
    }
    writeln!(output).ok();
    output.push_str(&verification_markdown(facts));
    writeln!(output).ok();
    writeln!(output, "#### 分析范围与限制").ok();
    for item in facts.coverage.checked.iter().take(20) {
        writeln!(output, "- 已检查：{}", md(item)).ok();
    }
    for item in facts.coverage.omitted.iter().take(20) {
        writeln!(output, "- 未检查：{}", md(item)).ok();
    }
    for item in facts.coverage.warnings.iter().take(20) {
        writeln!(output, "- 警告：{}", md(item)).ok();
    }
    let hidden_coverage = facts.coverage.checked.len().saturating_sub(20)
        + facts.coverage.omitted.len().saturating_sub(20)
        + facts.coverage.warnings.len().saturating_sub(20);
    if hidden_coverage > 0 {
        writeln!(
            output,
            "- 其余 {} 项范围记录保存在 HTML 和 facts.json artifact 中。",
            hidden_coverage
        )
        .ok();
    }
    for item in &analysis.unknowns {
        writeln!(output, "- 未知：{}", md(&item.text)).ok();
    }
    writeln!(output).ok();
    writeln!(output, "</details>").ok();
    writeln!(output).ok();
    writeln!(
        output,
        "**比较范围**：目标分支 `{}`；merge-base `{}`；head `{}`；实际差异 `{}`。",
        short_sha(&facts.compare.target_branch_sha),
        short_sha(&facts.compare.merge_base_sha),
        short_sha(&facts.compare.head_sha),
        md(&facts.compare.notation)
    )
    .ok();
    writeln!(
        output,
        "**完整 HTML**：在 [本次工作流运行]({}) 的 Artifacts 中下载 `{}`。Artifact 是下载文件，不是在线预览。",
        workflow_url(),
        md(&state.artifact_name)
    )
    .ok();
    writeln!(
        output,
        "**模型状态**：`{}`{}。本导读只辅助理解，不构成正确性证明，也不会批准、关闭或合并 PR。",
        md(&state.model_status),
        state
            .model_name
            .as_ref()
            .map(|name| format!(" / `{}`", code(name)))
            .unwrap_or_default()
    )
    .ok();
    output
}

fn verification_markdown(facts: &FactBundle) -> String {
    let mut output = String::new();
    writeln!(
        output,
        "#### 验证情况（仅限当前 head `{}`）",
        short_sha(&facts.pull_request.head_sha)
    )
    .ok();
    if facts.checks.is_empty() {
        writeln!(
            output,
            "- 当前无法确认：GitHub 未返回此 head 的 check-run 记录。"
        )
        .ok();
    }
    for check in facts.checks.iter().take(30) {
        let category = match (check.status.as_str(), check.conclusion.as_deref()) {
            ("completed", Some("success")) => "已执行并通过",
            ("completed", Some("neutral" | "skipped")) => "已执行但未形成通过证据",
            ("completed", _) => "已执行但失败或未通过",
            _ => "当前仍在执行或排队",
        };
        let name = check
            .details_url
            .as_ref()
            .map(|url| format!("[{}]({url})", md(&check.name)))
            .unwrap_or_else(|| md(&check.name));
        writeln!(
            output,
            "- {}：{}（状态 `{}`，结论 `{}`，SHA `{}`）",
            category,
            name,
            code(&check.status),
            code(check.conclusion.as_deref().unwrap_or("尚无")),
            short_sha(&check.head_sha)
        )
        .ok();
    }
    if facts.checks.len() > 30 {
        writeln!(
            output,
            "- 其余 {} 项 check-run 保存在完整报告中。",
            facts.checks.len() - 30
        )
        .ok();
    }
    let has_test_reference = facts
        .context
        .iter()
        .any(|item| item.kind == "test_reference");
    let has_test_check = facts
        .checks
        .iter()
        .any(|check| check.name.to_ascii_lowercase().contains("test"));
    if has_test_reference && !has_test_check {
        writeln!(
            output,
            "- 有测试代码但未执行：发现相关测试文本引用，但当前 head 没有名称含 `test` 的 check-run 证据。"
        )
        .ok();
    }
    output
}

fn mermaid(facts: &FactBundle, analysis: &Analysis) -> String {
    let relations = combined_relations(facts, analysis);
    if relations.is_empty() || analysis.diagram_kind == "none" {
        return "无可确认的关系变化，因此不强行生成架构图。\n".to_owned();
    }
    let mut nodes = BTreeMap::new();
    for relation in relations.iter().take(16) {
        if nodes.len() >= 12 {
            break;
        }
        let next = nodes.len() + 1;
        nodes.entry(relation.from.clone()).or_insert(next);
        let next = nodes.len() + 1;
        nodes.entry(relation.to.clone()).or_insert(next);
    }
    let mut output = String::from("```mermaid\n");
    match analysis.diagram_kind.as_str() {
        "sequence" => {
            output.push_str("sequenceDiagram\n");
            for (label, id) in &nodes {
                writeln!(output, "  participant N{id} as {}", mermaid_label(label)).ok();
            }
            for relation in relations.iter().take(16) {
                let (Some(from), Some(to)) = (nodes.get(&relation.from), nodes.get(&relation.to))
                else {
                    continue;
                };
                writeln!(
                    output,
                    "  N{from}->>N{to}: {}",
                    mermaid_relation_label(relation)
                )
                .ok();
            }
        }
        "state" => {
            output.push_str("stateDiagram-v2\n");
            for (label, id) in &nodes {
                writeln!(output, "  state \"{}\" as N{id}", mermaid_label(label)).ok();
            }
            for relation in relations.iter().take(16) {
                let (Some(from), Some(to)) = (nodes.get(&relation.from), nodes.get(&relation.to))
                else {
                    continue;
                };
                writeln!(
                    output,
                    "  N{from} --> N{to}: {}",
                    mermaid_relation_label(relation)
                )
                .ok();
            }
        }
        _ => {
            output.push_str("flowchart TD\n");
            for (label, id) in &nodes {
                writeln!(output, "  N{id}[\"{}\"]", mermaid_label(label)).ok();
            }
            for relation in relations.iter().take(16) {
                let (Some(from), Some(to)) = (nodes.get(&relation.from), nodes.get(&relation.to))
                else {
                    continue;
                };
                let label = mermaid_relation_label(relation);
                if relation.certainty.starts_with("confirmed") {
                    writeln!(output, "  N{from} -->|\"{label}\"| N{to}").ok();
                } else {
                    writeln!(output, "  N{from} -.->|\"{label}\"| N{to}").ok();
                }
            }
        }
    }
    output.push_str("```\n\n实线标记程序化确认的导入/依赖；虚线标记 AI 推断。文字标签同时说明状态，不依赖颜色。\n");
    output
}

fn mermaid_relation_label(relation: &RelationFact) -> String {
    mermaid_label(&format!(
        "{} · {} · {}",
        relation.relation_type,
        relation.change,
        if relation.certainty.starts_with("confirmed") {
            "源码事实"
        } else {
            "推断"
        }
    ))
}

#[allow(
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    reason = "the standalone artifact keeps its escaped data binding and inline template together"
)]
fn html(facts: &FactBundle, analysis: &Analysis, state: &RunState) -> String {
    let relations = combined_relations(facts, analysis);
    let relation_types: BTreeSet<&str> = relations
        .iter()
        .map(|relation| relation.relation_type.as_str())
        .collect();
    let mut comparison_rows = String::new();
    for row in &analysis.before_after {
        write!(
            comparison_rows,
            "<tr><th scope=\"row\">{}</th><td data-side=\"before\">{}</td><td data-side=\"after\">{}</td><td>{} {}</td></tr>",
            h(&row.condition),
            h(&row.before),
            h(&row.after),
            h(certainty_label(&row.certainty)),
            html_evidence(&row.evidence_ids, &facts.evidence)
        )
        .ok();
    }
    if comparison_rows.is_empty() {
        comparison_rows
            .push_str("<tr><td colspan=\"4\">没有可可靠对照的行为或签名变化。</td></tr>");
    }
    let mut node_names = BTreeSet::new();
    for relation in &relations {
        node_names.insert(relation.from.as_str());
        node_names.insert(relation.to.as_str());
    }
    let mut nodes_html = String::new();
    for node in node_names.iter().take(24) {
        let status = facts
            .files
            .iter()
            .find(|file| file.module == *node || file.path == *node)
            .map(|file| file.status.as_str())
            .unwrap_or("context");
        write!(
            nodes_html,
            "<div class=\"node\" data-status=\"{}\"><span>{}</span><small>{}</small></div>",
            attr(status),
            h(node),
            h(status)
        )
        .ok();
    }
    let mut relation_rows = String::new();
    for relation in &relations {
        write!(
            relation_rows,
            "<li data-relation=\"{}\"><span class=\"endpoint\">{}</span><span class=\"arrow {}\">{}</span><span class=\"endpoint\">{}</span><span class=\"edge-meta\">{} · {} · {} {}</span></li>",
            attr(&relation.relation_type),
            h(&relation.from),
            if relation.certainty.starts_with("confirmed") { "solid" } else { "dashed" },
            if relation.certainty.starts_with("confirmed") { "→" } else { "⇢" },
            h(&relation.to),
            h(&relation.relation_type),
            h(&relation.change),
            h(&relation.certainty),
            html_evidence(&relation.evidence_ids, &facts.evidence)
        )
        .ok();
    }
    if relation_rows.is_empty() {
        relation_rows.push_str("<li>无可确认的关系变化；未强行绘制架构关系。</li>");
    }
    let mut filters = String::new();
    for kind in relation_types {
        write!(
            filters,
            "<label><input type=\"checkbox\" data-filter=\"{}\" checked> {}</label>",
            attr(kind),
            h(kind)
        )
        .ok();
    }
    let mut impact = String::new();
    for group in &analysis.impact_groups {
        write!(impact, "<article><h3>{}</h3><p>{}</p><p><strong>原因：</strong>{}</p><p><strong>调用方：</strong>{}</p><p>{}</p></article>", h(&group.name), h(&group.changed), h(&group.reason), h(&group.callers), html_evidence(&group.evidence_ids, &facts.evidence)).ok();
    }
    let mut api = String::new();
    for item in &analysis.api_changes {
        write!(
            api,
            "<tr><th scope=\"row\"><code>{}</code></th><td>{}</td><td>{}</td><td>{} {}</td></tr>",
            h(&item.symbol),
            h(&item.change),
            h(&item.compatibility),
            h(certainty_label(&item.certainty)),
            html_evidence(&item.evidence_ids, &facts.evidence)
        )
        .ok();
    }
    if api.is_empty() {
        api.push_str("<tr><td colspan=\"4\">未发现相关变化。</td></tr>");
    }
    let mut reading = String::new();
    for step in &analysis.reading_order {
        let link = step
            .evidence_id
            .as_deref()
            .and_then(|id| facts.evidence.iter().find(|item| item.id == id))
            .map(|item| format!("<a href=\"{}\">{}</a>", attr(&item.url), h(&step.path)))
            .unwrap_or_else(|| format!("<code>{}</code>", h(&step.path)));
        write!(
            reading,
            "<li><strong>{link}</strong><span>{}</span></li>",
            h(&step.reason)
        )
        .ok();
    }
    let mut risks = String::new();
    for risk in &analysis.risks {
        write!(
            risks,
            "<li><strong>{}</strong><span>{}</span><small>{} {}</small></li>",
            h(&risk.topic),
            h(&risk.detail),
            h(certainty_label(&risk.certainty)),
            html_evidence(&risk.evidence_ids, &facts.evidence)
        )
        .ok();
    }
    if risks.is_empty() {
        risks.push_str("<li><strong>没有具体风险结论</strong><span>这不表示没有风险。当前事实不足或模型不可用时，需要人工继续确认。</span></li>");
    }
    let verification = verification_html(facts);
    let mut coverage = String::new();
    for item in &facts.coverage.checked {
        write!(
            coverage,
            "<li data-kind=\"checked\">已检查：{}</li>",
            h(item)
        )
        .ok();
    }
    for item in &facts.coverage.omitted {
        write!(
            coverage,
            "<li data-kind=\"omitted\">未检查：{}</li>",
            h(item)
        )
        .ok();
    }
    for item in &facts.coverage.warnings {
        write!(coverage, "<li data-kind=\"warning\">警告：{}</li>", h(item)).ok();
    }
    for item in &analysis.unknowns {
        write!(
            coverage,
            "<li data-kind=\"unknown\">未知：{}</li>",
            h(&item.text)
        )
        .ok();
    }

    format!(
        r#"<!doctype html>
<html lang="zh-CN" data-theme="light">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data:">
<title>PR #{pr} 结构化导读</title>
<style>
:root{{--bg:#f5f7fa;--paper:#fff;--ink:#17202b;--muted:#526173;--line:#cfd8e3;--accent:#185fa5;--fact:#11734b;--warn:#9a5b00;--danger:#a33131;--soft:#eaf1f8;--shadow:0 10px 30px rgba(23,32,43,.08)}}
@media(prefers-color-scheme:dark){{:root{{--bg:#111820;--paper:#18222d;--ink:#edf3f8;--muted:#a7b5c4;--line:#344354;--accent:#71b7f2;--fact:#6fd4a6;--warn:#f3be68;--danger:#ff8b8b;--soft:#223245;--shadow:none}}}}
:root[data-theme="dark"]{{--bg:#111820;--paper:#18222d;--ink:#edf3f8;--muted:#a7b5c4;--line:#344354;--accent:#71b7f2;--fact:#6fd4a6;--warn:#f3be68;--danger:#ff8b8b;--soft:#223245;--shadow:none}}
:root[data-theme="light"]{{--bg:#f5f7fa;--paper:#fff;--ink:#17202b;--muted:#526173;--line:#cfd8e3;--accent:#185fa5;--fact:#11734b;--warn:#9a5b00;--danger:#a33131;--soft:#eaf1f8;--shadow:0 10px 30px rgba(23,32,43,.08)}}
*{{box-sizing:border-box}}body{{margin:0;background:var(--bg);color:var(--ink);font:16px/1.55 ui-sans-serif,system-ui,-apple-system,"Segoe UI",sans-serif}}a{{color:var(--accent)}}code,.mono{{font-family:ui-monospace,SFMono-Regular,Consolas,monospace;font-variant-numeric:tabular-nums}}header,main,footer{{width:min(1180px,calc(100% - 32px));margin:auto}}header{{padding:36px 0 24px;display:grid;gap:16px}}h1,h2,h3{{font-family:Georgia,"Noto Serif SC",serif;text-wrap:balance;margin:0}}h1{{font-size:clamp(2rem,5vw,4.4rem);line-height:1.02}}h2{{font-size:1.55rem}}h3{{font-size:1.05rem}}.eyebrow{{font-size:.78rem;letter-spacing:.13em;text-transform:uppercase;color:var(--muted)}}.summary{{max-width:65ch;font-size:1.16rem}}.meta{{display:flex;flex-wrap:wrap;gap:8px}}.chip{{border:1px solid var(--line);padding:5px 9px;background:var(--paper);font-size:.82rem}}button{{border:1px solid var(--line);background:var(--paper);color:var(--ink);padding:8px 11px;cursor:pointer}}button:focus-visible,input:focus-visible,a:focus-visible{{outline:3px solid var(--accent);outline-offset:2px}}main{{display:grid;gap:18px;padding-bottom:44px}}section{{background:var(--paper);border:1px solid var(--line);box-shadow:var(--shadow);padding:clamp(18px,3vw,30px);display:grid;gap:18px}}.section-head{{display:flex;align-items:flex-start;justify-content:space-between;gap:12px}}.scroll{{overflow-x:auto}}table{{width:100%;border-collapse:collapse;min-width:720px}}th,td{{border-bottom:1px solid var(--line);text-align:left;vertical-align:top;padding:10px}}th{{font-weight:650}}.impact-grid,.node-grid{{display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:12px}}article,.node{{border:1px solid var(--line);padding:14px;background:var(--bg)}}article p{{margin:.6em 0}}.node{{border-left:5px solid var(--accent)}}.node[data-status="added"]{{border-left-style:double;border-left-color:var(--fact)}}.node[data-status="removed"]{{border-left-style:dashed;border-left-color:var(--danger)}}.node[data-status="modified"]{{border-left-color:var(--warn)}}.node small{{display:block;color:var(--muted)}}.filters{{display:flex;flex-wrap:wrap;gap:10px}}.edge-list,.reading,.risk-list,.coverage{{list-style:none;padding:0;margin:0;display:grid;gap:9px}}.edge-list li{{display:grid;grid-template-columns:minmax(120px,1fr) 50px minmax(120px,1fr) minmax(190px,1.4fr);align-items:center;gap:8px;border-top:1px solid var(--line);padding:10px 0}}.endpoint{{font-family:ui-monospace,monospace;overflow-wrap:anywhere}}.arrow{{text-align:center;font-size:1.4rem}}.arrow.dashed{{border-bottom:2px dashed var(--warn)}}.arrow.solid{{border-bottom:2px solid var(--fact)}}.edge-meta,small{{color:var(--muted)}}.reading li,.risk-list li{{display:grid;gap:4px;border-left:3px solid var(--line);padding-left:12px}}.coverage li[data-kind="omitted"],.coverage li[data-kind="warning"]{{color:var(--warn)}}.coverage li[data-kind="unknown"]{{color:var(--danger)}}.evidence{{font-family:ui-monospace,monospace;font-size:.78rem;white-space:nowrap}}footer{{padding:0 0 42px;color:var(--muted);max-width:75ch}}[hidden]{{display:none!important}}@media(max-width:720px){{header,main,footer{{width:min(100% - 20px,1180px)}}header{{padding-top:22px}}section{{padding:16px}}.edge-list li{{grid-template-columns:1fr auto 1fr}}.edge-meta{{grid-column:1/-1}}}}@media(prefers-reduced-motion:reduce){{*{{scroll-behavior:auto!important;transition:none!important;animation:none!important}}}}@media print{{button,.filters{{display:none}}body{{background:#fff;color:#000}}section{{box-shadow:none;break-inside:avoid}}}}
</style>
</head>
<body>
<header>
<div class="section-head"><div><p class="eyebrow">BurnCloud · pull request evidence map</p><h1>PR #{pr} 结构化导读</h1></div><button id="theme" type="button" aria-label="切换明暗主题">切换主题</button></div>
<p class="summary">{purpose}</p>
<div class="meta"><span class="chip">{draft}</span><span class="chip">base {base}</span><span class="chip">merge-base {merge}</span><span class="chip">head {head}</span><span class="chip">{status}</span><span class="chip">{generated}</span></div>
</header>
<main>
<section><div class="section-head"><h2>修改前后对比</h2><div><button type="button" data-side-button="all">并排</button> <button type="button" data-side-button="before">只看修改前</button> <button type="button" data-side-button="after">只看修改后</button></div></div><div class="scroll"><table><thead><tr><th>条件/输入</th><th data-side="before">修改前</th><th data-side="after">修改后</th><th>证据性质</th></tr></thead><tbody>{comparison_rows}</tbody></table></div></section>
<section><h2>影响范围</h2><div class="impact-grid">{impact}</div></section>
<section><h2>接口与数据结构变化</h2><div class="scroll"><table><thead><tr><th>符号</th><th>变化</th><th>兼容性</th><th>证据性质</th></tr></thead><tbody>{api}</tbody></table></div></section>
<section><div class="section-head"><div><h2>变更关系图</h2><p>实线＝程序化确认的导入/依赖；虚线＝推断。状态同时以文字和线型表达。</p></div><span class="chip">{diagram_kind}</span></div><div class="filters" aria-label="关系过滤">{filters}</div><div class="node-grid">{nodes_html}</div><ol class="edge-list">{relation_rows}</ol></section>
<section><h2>建议阅读顺序</h2><ol class="reading">{reading}</ol></section>
<section><h2>风险与待确认事项</h2><ul class="risk-list">{risks}</ul></section>
<section><h2>验证情况</h2>{verification}</section>
<section><h2>分析范围与限制</h2><ul class="coverage">{coverage}</ul><p class="mono">比较范围：{range}</p><p>模型状态：<code>{model_status}</code>{model_name}</p></section>
</main>
<footer><p>本报告依据绑定提交 SHA 的源码、Git 差异和当前 head 的 GitHub check-runs 生成。它辅助理解，不构成正确性证明，也不会批准、关闭或合并 PR。</p><p>Effective HTML html-diagram 参考版本：<code>975b047f6da6874e176db54f048c0c31c30e9f65</code>。</p></footer>
<script>
const root=document.documentElement;document.getElementById('theme').addEventListener('click',()=>{{root.dataset.theme=root.dataset.theme==='dark'?'light':'dark'}});
document.querySelectorAll('[data-side-button]').forEach(button=>button.addEventListener('click',()=>{{const side=button.dataset.sideButton;document.querySelectorAll('[data-side]').forEach(cell=>{{cell.hidden=side!=='all'&&cell.dataset.side!==side}})}}));
document.querySelectorAll('[data-filter]').forEach(input=>input.addEventListener('change',()=>{{document.querySelectorAll(`[data-relation="${{CSS.escape(input.dataset.filter)}}"]`).forEach(row=>row.hidden=!input.checked)}}));
</script>
</body></html>"#,
        pr = facts.pull_request.number,
        purpose = h(&analysis.purpose),
        draft = if facts.pull_request.draft {
            "草稿 PR"
        } else {
            "Ready PR"
        },
        base = h(&short_sha(&facts.compare.target_branch_sha)),
        merge = h(&short_sha(&facts.compare.merge_base_sha)),
        head = h(&short_sha(&facts.compare.head_sha)),
        status = h(&state.analysis_status),
        generated = h(&state.generated_at),
        comparison_rows = comparison_rows,
        impact = impact,
        api = api,
        diagram_kind = h(&analysis.diagram_kind),
        filters = filters,
        nodes_html = nodes_html,
        relation_rows = relation_rows,
        reading = reading,
        risks = risks,
        verification = verification,
        coverage = coverage,
        range = h(&facts.compare.notation),
        model_status = h(&state.model_status),
        model_name = state
            .model_name
            .as_ref()
            .map(|name| format!(" / <code>{}</code>", h(name)))
            .unwrap_or_default(),
    )
}

fn verification_html(facts: &FactBundle) -> String {
    let mut output = String::from("<ul class=\"coverage\">");
    if facts.checks.is_empty() {
        output.push_str(
            "<li data-kind=\"unknown\">当前无法确认：GitHub 未返回此 head 的 check-run 记录。</li>",
        );
    }
    for check in &facts.checks {
        let category = match (check.status.as_str(), check.conclusion.as_deref()) {
            ("completed", Some("success")) => "已执行并通过",
            ("completed", Some("neutral" | "skipped")) => "已执行但未形成通过证据",
            ("completed", _) => "已执行但失败或未通过",
            _ => "当前仍在执行或排队",
        };
        let name = check
            .details_url
            .as_ref()
            .map(|url| format!("<a href=\"{}\">{}</a>", attr(url), h(&check.name)))
            .unwrap_or_else(|| h(&check.name));
        write!(output, "<li>{category}：{name}（状态 <code>{}</code>，结论 <code>{}</code>，SHA <code>{}</code>）</li>", h(&check.status), h(check.conclusion.as_deref().unwrap_or("尚无")), h(&short_sha(&check.head_sha))).ok();
    }
    output.push_str("</ul>");
    output
}

fn combined_relations(facts: &FactBundle, analysis: &Analysis) -> Vec<RelationFact> {
    let mut relations = facts.relations.clone();
    for relation in &analysis.relations {
        if !relations.iter().any(|item| {
            item.from == relation.from
                && item.to == relation.to
                && item.relation_type == relation.relation_type
        }) {
            relations.push(relation.clone());
        }
    }
    relations.truncate(40);
    relations
}

fn evidence_links(ids: &[String], evidence: &[Evidence]) -> String {
    let links: Vec<String> = ids
        .iter()
        .filter_map(|id| evidence.iter().find(|item| item.id == *id))
        .map(|item| format!("[{}]({})", item.id, item.url))
        .collect();
    links.join(" ")
}

fn html_evidence(ids: &[String], evidence: &[Evidence]) -> String {
    let links: Vec<String> = ids
        .iter()
        .filter_map(|id| evidence.iter().find(|item| item.id == *id))
        .map(|item| {
            format!(
                "<a class=\"evidence\" href=\"{}\">{}</a>",
                attr(&item.url),
                h(&item.id)
            )
        })
        .collect();
    links.join(" ")
}

fn certainty_label(value: &str) -> &'static str {
    match value {
        "programmatically_confirmed_signature" | "programmatically_matched_symbol" => "程序化事实",
        "confirmed_import" => "程序化依赖事实",
        "source_supported_inference" => "有源码依据的推论",
        "inference" => "AI 推断",
        _ => "未知/待确认",
    }
}

fn workflow_url() -> String {
    let server =
        std::env::var("GITHUB_SERVER_URL").unwrap_or_else(|_| "https://github.com".to_owned());
    let repository =
        std::env::var("GITHUB_REPOSITORY").unwrap_or_else(|_| "burncloud/burncloud".to_owned());
    let run = std::env::var("GITHUB_RUN_ID").unwrap_or_else(|_| "unknown".to_owned());
    format!("{server}/{repository}/actions/runs/{run}")
}

fn short_sha(value: &str) -> String {
    value.chars().take(12).collect()
}

fn mermaid_label(value: &str) -> String {
    value
        .chars()
        .filter(|ch| !matches!(ch, '"' | '[' | ']' | '{' | '}' | '`'))
        .take(80)
        .collect()
}

fn h(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn attr(value: &str) -> String {
    h(value).replace('`', "&#96;")
}

fn md(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('*', "\\*")
        .replace('_', "\\_")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\n', " ")
}

fn table(value: &str) -> String {
    md(value).replace('|', "\\|")
}

fn code(value: &str) -> String {
    value.replace('`', "'").replace('\n', " ")
}
