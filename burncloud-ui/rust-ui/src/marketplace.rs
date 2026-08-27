use crate::{
    auth::escape_html,
    backend::CatalogModel,
    overview::{ShellContext, icon, render_buyer_shell},
};

pub fn render_marketplace(
    shell: &ShellContext,
    catalog: &[CatalogModel],
    warnings: &[String],
) -> String {
    let cards = catalog.iter().map(render_card).collect::<String>();
    let templates = catalog
        .iter()
        .map(render_detail_template)
        .collect::<String>();
    let (tone, title, summary) = if !warnings.is_empty() {
        ("warning", "模型目录连接不完整", warnings.join("；"))
    } else if catalog.is_empty() {
        (
            "warning",
            "尚无可购买的模型服务",
            "数据库中没有已启用且声明模型的渠道。".to_string(),
        )
    } else {
        (
            "healthy",
            "模型服务目录已与数据库同步",
            format!(
                "{} 个模型由已启用渠道提供，价格与能力来自 BurnCloud 数据库。",
                catalog.len()
            ),
        )
    };
    let empty_attr = if catalog.is_empty() { "" } else { " hidden" };
    let content = format!(
        r#"<div class="marketplace-page" data-marketplace><header class="page-header"><div class="page-heading"><p class="eyebrow">BUYER WORKSPACE</p><h1>模型市场与基准评测</h1><p>发现、比较并接入 BurnCloud 数据库中实际启用的模型服务。</p></div></header>
          <section class="conclusion {tone} marketplace-conclusion" aria-live="polite">{}<div><strong>{title}</strong><p>{}</p></div></section>
          <section class="marketplace-toolbar" aria-label="筛选模型"><div class="category-filter" role="group" aria-label="模型类别"><button type="button" class="active" data-category="all" aria-pressed="true">全量模型</button><button type="button" data-category="general" aria-pressed="false">通用大模型</button><button type="button" data-category="reasoning" aria-pressed="false">深度推理与数学</button><button type="button" data-category="coding" aria-pressed="false">代码与智能体</button><button type="button" data-category="multimodal" aria-pressed="false">多模态</button></div><label class="marketplace-search" for="model-search">{}<span class="sr-only">搜索模型</span><input id="model-search" type="search" autocomplete="off" placeholder="搜索模型、厂商或能力..."><kbd>/</kbd></label></section>
          <div class="marketplace-results"><p id="result-count" role="status" aria-live="polite">共 {} 个可用模型</p><span>价格按每 100 万 Token 计费</span></div><section class="model-grid" id="model-grid" aria-label="模型目录">{cards}</section>
          <section class="marketplace-empty" id="marketplace-empty"{empty_attr}>{}<h2>没有匹配的模型</h2><p>尝试更换类别、缩短搜索关键词，或在后端启用模型渠道。</p><button type="button" class="button secondary" id="clear-filters">清除筛选</button></section>{templates}
          <div class="drawer-layer" id="model-drawer" hidden><button type="button" class="drawer-backdrop" data-close-drawer tabindex="-1" aria-label="关闭模型详情"></button><aside class="model-drawer" role="dialog" aria-modal="true" aria-labelledby="drawer-title" aria-describedby="drawer-subtitle"><header class="drawer-header"><div><p class="eyebrow">MODEL SPECIFICATIONS</p><h2 id="drawer-title"></h2><p id="drawer-subtitle"></p></div><button type="button" class="icon-button" data-close-drawer aria-label="关闭面板">{}</button></header><div class="drawer-body" id="drawer-body"></div></aside></div></div>"#,
        if tone == "healthy" {
            icon("check-circle")
        } else {
            icon("alert-triangle")
        },
        escape_html(&summary),
        marketplace_icon("search"),
        catalog.len(),
        marketplace_icon("search"),
        icon("x"),
    );
    render_buyer_shell(
        content,
        shell,
        "/buyer/marketplace",
        "Buyer Model Marketplace - BurnCloud",
        "BurnCloud Buyer 数据库模型市场与真实价格目录",
        r#"<script src="/assets/marketplace.js" defer></script>"#,
    )
}

fn render_card(model: &CatalogModel) -> String {
    let family = family(model);
    let category = category(model);
    let capabilities = capabilities(model);
    let context = context_label(model.context_window);
    let latency = model
        .p95_latency_ms
        .map_or_else(|| "待采样".to_string(), |value| format!("P95 {value}ms"));
    let providers = if model.providers.is_empty() {
        "BurnCloud Router".to_string()
    } else {
        model.providers.join(" / ")
    };
    format!(
        r#"<article class="model-card" data-model-card data-category="{category}" data-search="{} {} {} {}"><div class="model-card-header"><span class="model-family">{}</span><span class="status healthy"><span></span>{} 个渠道</span></div><div class="model-card-copy"><h2>{}</h2><p>{}</p></div><div class="model-price-strip"><div><span>输入 / 输出价格</span><strong>{} <i>/</i> {}</strong><small>每 100 万 Token</small></div><div><span>CONTEXT</span><strong>{context}</strong><small>可用 · {latency}</small></div></div><div class="model-tiers"><span>CAPABILITIES</span>{capabilities}</div><footer class="model-card-actions"><button type="button" class="text-button" data-open-model="{}">查看参数规格</button><a class="button primary compact" href="/buyer/playground?model={}"><span>在操练场体验</span>{}</a></footer></article>"#,
        escape_html(&model.id.to_lowercase()),
        escape_html(&family.to_lowercase()),
        escape_html(&providers.to_lowercase()),
        escape_html(&capabilities.to_lowercase()),
        escape_html(&family),
        model.available_channels,
        escape_html(&model.id),
        escape_html(&format!(
            "由 {providers} 提供，能力和价格从数据库实时汇总。"
        )),
        price(model.input_price_per_million),
        price(model.output_price_per_million),
        escape_html(&model.id),
        query_component(&model.id),
        marketplace_icon("arrow-right")
    )
}

fn render_detail_template(model: &CatalogModel) -> String {
    let family = family(model);
    let category = category(model);
    let providers = if model.providers.is_empty() {
        "BurnCloud Router".to_string()
    } else {
        model.providers.join("、")
    };
    let max_output = model
        .max_output_tokens
        .map_or_else(|| "数据库未声明".to_string(), compact_integer);
    let latency = model
        .p95_latency_ms
        .map_or_else(|| "待运行采样".to_string(), |value| format!("{value} ms"));
    format!(
        r#"<template id="model-detail-{}" data-title="{}" data-subtitle="{} · {}"><div class="detail-section"><h3>数据库服务信息</h3><p>当前模型由 {} 提供，共有 {} 个已启用渠道。页面不返回渠道密钥或内部地址。</p></div><section class="detail-pricing"><div class="detail-pricing-heading"><strong>BurnCloud 数据库费率</strong><span>LIVE DATA</span></div><div class="detail-price-grid"><div><span>输入价格</span><strong>{}</strong><small>/ 100万 Token</small></div><div><span>输出价格</span><strong>{}</strong><small>/ 100万 Token</small></div></div></section><section class="detail-section"><h3>模型能力</h3><div class="benchmark-grid"><div><span>视觉输入</span><strong>{}</strong></div><div><span>函数调用</span><strong>{}</strong></div><div><span>类型</span><strong>{}</strong></div></div></section><section class="detail-section"><h3>推荐使用场景</h3><p class="recommendation">{}</p></section><section class="advanced-section"><button type="button" class="advanced-toggle" aria-expanded="false">{}<span>运行规格与路由数据</span>{}</button><dl hidden><div><dt>渠道响应延迟</dt><dd>{latency}</dd></div><div><dt>上下文窗口</dt><dd>{}</dd></div><div><dt>最大输出</dt><dd>{max_output}</dd></div><div><dt>可用渠道</dt><dd class="healthy">{} 个</dd></div><div><dt>数据来源</dt><dd>BurnCloud Database</dd></div></dl></section><a class="button primary drawer-cta" href="/buyer/playground?model={}">{}<span>在操练场体验 {}</span></a></template>"#,
        escape_html(&model.id),
        escape_html(&model.id),
        escape_html(&family),
        category_label(category),
        escape_html(&providers),
        model.available_channels,
        price(model.input_price_per_million),
        price(model.output_price_per_million),
        yes_no(model.supports_vision),
        yes_no(model.supports_function_calling),
        escape_html(model.model_type.as_deref().unwrap_or("chat")),
        escape_html(&recommendation(model)),
        marketplace_icon("activity"),
        marketplace_icon("chevron-down"),
        context_label(model.context_window),
        model.available_channels,
        query_component(&model.id),
        icon("terminal"),
        escape_html(&model.id)
    )
}

fn family(model: &CatalogModel) -> String {
    model.providers.first().cloned().unwrap_or_else(|| {
        let id = model.id.to_lowercase();
        if id.contains("deepseek") {
            "DeepSeek"
        } else if id.contains("qwen") {
            "Qwen"
        } else if id.contains("claude") {
            "Anthropic"
        } else if id.contains("gemini") {
            "Google"
        } else if id.contains("gpt") {
            "OpenAI"
        } else if id.contains("llama") {
            "Meta"
        } else {
            "BurnCloud"
        }
        .to_string()
    })
}

fn category(model: &CatalogModel) -> &'static str {
    let id = model.id.to_lowercase();
    if model.supports_vision
        || model
            .model_type
            .as_deref()
            .is_some_and(|kind| kind != "chat")
    {
        "multimodal"
    } else if id.contains("reason") || id.contains("r1") || id.contains("o1") || id.contains("o3") {
        "reasoning"
    } else if id.contains("code") || id.contains("coder") || id.contains("claude") {
        "coding"
    } else {
        "general"
    }
}

fn capabilities(model: &CatalogModel) -> String {
    let mut values = vec![r#"<span class="market-tier">文本</span>"#.to_string()];
    if model.supports_vision {
        values.push(r#"<span class="market-tier performance">视觉</span>"#.to_string());
    }
    if model.supports_function_calling {
        values.push(r#"<span class="market-tier economy">工具调用</span>"#.to_string());
    }
    values.join("")
}

fn recommendation(model: &CatalogModel) -> String {
    match category(model) {
        "reasoning" => "复杂数学、架构推理和多步决策工作负载。",
        "coding" => "代码生成、重构、智能体工具调用与工程分析。",
        "multimodal" => "图像理解、语音或其他多模态输入输出任务。",
        _ => "通用对话、内容生成、分类与企业知识应用。",
    }
    .to_string()
}

fn category_label(category: &str) -> &'static str {
    match category {
        "reasoning" => "深度推理与数学",
        "coding" => "代码与智能体",
        "multimodal" => "多模态",
        _ => "通用大模型",
    }
}

fn price(value: Option<f64>) -> String {
    value.map_or_else(|| "待定价".to_string(), |number| format!("${number:.4}"))
}
fn yes_no(value: bool) -> &'static str {
    if value { "支持" } else { "未声明" }
}
fn context_label(value: Option<i64>) -> String {
    value.map_or_else(|| "未声明".to_string(), compact_integer)
}
fn compact_integer(value: i64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M tokens", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{}K tokens", value / 1_000)
    } else {
        format!("{value} tokens")
    }
}
fn query_component(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

fn marketplace_icon(name: &str) -> &'static str {
    match name {
        "search" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg>"#
        }
        "arrow-right" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12h14M13 6l6 6-6 6"/></svg>"#
        }
        "activity" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 12h4l3-8 4 16 3-8h4"/></svg>"#
        }
        "chevron-down" => {
            r#"<svg class="chevron" viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>"#
        }
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::render_marketplace;
    use crate::{
        backend::{CatalogModel, CurrentAccount},
        overview::ShellContext,
    };

    #[test]
    fn renders_only_models_received_from_database_catalog() {
        let shell = ShellContext {
            account: CurrentAccount {
                username: "buyer".to_string(),
                ..CurrentAccount::default()
            },
            balance_label: "$0.00".to_string(),
            attention: false,
        };
        let models = vec![CatalogModel {
            id: "db/model-a".to_string(),
            providers: vec!["provider-a".to_string()],
            available_channels: 2,
            input_price_per_million: Some(0.1),
            output_price_per_million: Some(0.2),
            ..CatalogModel::default()
        }];
        let page = render_marketplace(&shell, &models, &[]);
        assert_eq!(page.matches("data-model-card").count(), 1);
        assert!(page.contains("db/model-a"));
        assert!(page.contains("provider-a"));
        assert!(!page.contains("channel.key"));
    }
}
