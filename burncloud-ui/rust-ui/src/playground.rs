use crate::{
    auth::escape_html,
    backend::{CatalogModel, TokenSummary},
    overview::{ShellContext, icon, render_buyer_shell},
};

pub fn render_playground(
    shell: &ShellContext,
    catalog: &[CatalogModel],
    tokens: &[TokenSummary],
    requested_model: Option<&str>,
    warnings: &[String],
) -> String {
    let active_token = tokens.iter().find(|token| token.status == "active");
    let selected_id = requested_model
        .filter(|requested| catalog.iter().any(|model| model.id == *requested))
        .or_else(|| catalog.first().map(|model| model.id.as_str()));
    let options = if catalog.is_empty() {
        r#"<option value="">数据库中没有已启用的模型</option>"#.to_string()
    } else {
        catalog
            .iter()
            .map(|model| {
                let input = model.input_price_per_million.unwrap_or_default();
                let output = model.output_price_per_million.unwrap_or_default();
                format!(
                    r#"<option value="{}" data-input-price="{input:.9}" data-output-price="{output:.9}"{}>{} (${input:.4} / ${output:.4})</option>"#,
                    escape_html(&model.id),
                    if selected_id == Some(model.id.as_str()) { " selected" } else { "" },
                    escape_html(&model.id),
                )
            })
            .collect::<String>()
    };
    let ready = active_token.is_some() && !catalog.is_empty() && warnings.is_empty();
    let readiness = if !warnings.is_empty() {
        ("warning", "操练场数据连接不完整", warnings.join("；"))
    } else if catalog.is_empty() {
        (
            "warning",
            "尚无可用模型",
            "请先在 BurnCloud 后端启用至少一个包含模型的渠道。".to_string(),
        )
    } else if active_token.is_none() {
        (
            "warning",
            "需要有效的 API 密钥",
            "请先创建或启用当前账户的 API 密钥，推理密钥不会暴露给浏览器。".to_string(),
        )
    } else {
        (
            "healthy",
            "真实推理服务已就绪",
            format!(
                "{} 个数据库模型 · {} 个有效 API 密钥 · 请求通过 BurnCloud 数据面路由",
                catalog.len(),
                tokens
                    .iter()
                    .filter(|token| token.status == "active")
                    .count()
            ),
        )
    };
    let token_ref = active_token
        .map(|token| token.token.as_str())
        .unwrap_or_default();
    let disabled = if ready { "" } else { " disabled" };
    let model_name = selected_id.unwrap_or("未选择模型");

    let content = format!(
        r#"<div class="playground-page" data-playground data-token-ref="{}">
          <header class="page-header"><div class="page-heading"><p class="eyebrow">BUYER WORKSPACE</p><h1>交互式推理操练场</h1><p>使用当前账户的真实 API 密钥引用验证模型、路由与上游响应，密钥只保留在后端。</p></div></header>
          <section class="conclusion {} playground-conclusion" aria-live="polite">{}<div><strong>{}</strong><p id="playground-summary">{}</p></div></section>
          <div class="playground-grid"><aside class="playground-controls" aria-label="模型和调用参数">
            <section class="tool-panel settings-panel" aria-labelledby="settings-title"><div class="panel-title">{}<h2 id="settings-title">模型与路由</h2></div>
              <div class="field"><label for="model-select">选择数据库模型</label><select id="model-select"{disabled}>{options}</select></div>
              <fieldset class="field tier-field"><legend>算力优化路由等级</legend><div class="segmented-control" aria-label="路由等级"><button type="button" data-tier="economy" aria-pressed="false">经济级</button><button type="button" data-tier="standard" class="active" aria-pressed="true">标准级</button><button type="button" data-tier="performance" aria-pressed="false">性能级</button></div><p id="tier-description">路由等级会写入可复制代码示例；真实路由仍遵循后端当前策略。</p></fieldset>
              <div class="parameter-group"><div class="parameter-label"><label for="temperature">Temperature</label><output id="temperature-value">0.7</output></div><input id="temperature" type="range" min="0" max="2" step="0.1" value="0.7"><div class="parameter-label"><label for="max-tokens">Max Tokens</label><output id="max-tokens-value">2048</output></div><input id="max-tokens" type="range" min="256" max="8192" step="256" value="2048"></div>
            </section>
            <section class="tool-panel code-panel" aria-labelledby="code-title"><div class="code-panel-header"><div class="panel-title">{}<h2 id="code-title">API 调用代码</h2></div><div class="code-tabs" role="tablist" aria-label="代码语言"><button type="button" role="tab" data-code-tab="curl" aria-selected="true">cURL</button><button type="button" role="tab" data-code-tab="python" aria-selected="false">Python</button><button type="button" role="tab" data-code-tab="node" aria-selected="false">Node</button></div></div><pre class="code-preview"><code id="code-output"></code></pre><button type="button" class="button secondary copy-button" id="copy-code">{}<span>复制代码</span></button><p class="copy-status" id="copy-status" role="status" aria-live="polite"></p></section>
          </aside>
          <section class="tool-panel inference-panel" aria-labelledby="inference-title"><div class="panel-title">{}<h2 id="inference-title">真实路由测试</h2></div>
            <div class="field"><label for="system-prompt">系统提示词 <span>可选</span></label><textarea id="system-prompt" rows="3">You are an expert full-stack engineer and distributed systems architect. Provide precise, actionable technical answers.</textarea></div>
            <div class="field"><label for="user-prompt">用户提示词</label><textarea id="user-prompt" rows="5">Explain how hardware attestation guarantees secure multi-tenant LLM token generation.</textarea></div>
            <div class="execution-bar"><div class="attestation">{}<span>管理 JWT 与推理密钥隔离 · 密钥服务端托管</span></div><div class="execution-actions"><button type="button" class="button ghost" id="clear-output">{}<span>清空</span></button><button type="button" class="button primary" id="run-inference"{disabled}>{}<span>{}</span></button></div></div>
            <section class="output-panel" aria-labelledby="output-title"><div class="output-header"><h3 id="output-title">RESPONSE OUTPUT</h3><span class="stream-status" id="stream-status" hidden><i></i>正在通过真实路由请求</span></div><div class="response-output" id="response-output"><span class="output-placeholder">点击“运行真实推理”后执行数据库中配置的模型路由。</span></div><dl class="execution-stats" id="execution-stats" hidden><div><dt>首包耗时</dt><dd id="stat-ttft">--</dd></div><div><dt>总耗时</dt><dd id="stat-total">--</dd></div><div><dt>Token</dt><dd id="stat-tokens">--</dd></div><div><dt>费用估算</dt><dd class="cost" id="stat-cost">--</dd></div></dl><div class="receipt" id="execution-receipt" hidden><span>真实路由证据</span><code></code></div></section>
          </section></div></div>"#,
        escape_html(token_ref),
        readiness.0,
        if ready {
            icon("check-circle")
        } else {
            icon("alert-triangle")
        },
        readiness.1,
        escape_html(&readiness.2),
        icon("cpu"),
        icon("code"),
        icon("copy"),
        icon("terminal"),
        icon("shield"),
        icon("rotate"),
        icon("play"),
        if ready {
            "运行真实推理"
        } else {
            "等待配置"
        },
    );

    render_buyer_shell(
        content,
        shell,
        "/buyer/playground",
        "Buyer Playground - BurnCloud",
        &format!("BurnCloud Buyer 真实模型推理操练场，当前模型 {model_name}"),
        r#"<script src="/assets/playground.js" defer></script>"#,
    )
}

#[cfg(test)]
mod tests {
    use super::render_playground;
    use crate::{
        backend::{CatalogModel, CurrentAccount, TokenSummary},
        overview::ShellContext,
    };

    #[test]
    fn renders_database_models_and_opaque_token_reference() {
        let shell = ShellContext {
            account: CurrentAccount {
                username: "buyer".to_string(),
                status: 1,
                ..CurrentAccount::default()
            },
            balance_label: "$10.00".to_string(),
            attention: true,
        };
        let page = render_playground(
            &shell,
            &[CatalogModel {
                id: "db-model".to_string(),
                available_channels: 1,
                ..CatalogModel::default()
            }],
            &[TokenSummary {
                token: "tok_safe_reference".to_string(),
                status: "active".to_string(),
                ..TokenSummary::default()
            }],
            Some("db-model"),
            &[],
        );
        assert!(page.contains("value=\"db-model\""));
        assert!(page.contains("data-token-ref=\"tok_safe_reference\""));
        assert!(!page.contains("private-key-material"));
    }
}
