use crate::{
    auth::{escape_html, render_language_switcher},
    backend::{BillingSummary, CatalogModel, CurrentAccount, Recharge, TokenSummary},
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ShellContext {
    pub account: CurrentAccount,
    pub balance_label: String,
    pub attention: bool,
}

pub struct OverviewData {
    pub shell: ShellContext,
    pub billing: BillingSummary,
    pub catalog: Vec<CatalogModel>,
    pub tokens: Vec<TokenSummary>,
    pub recharges: Vec<Recharge>,
    pub warnings: Vec<String>,
}

pub fn render_overview(data: &OverviewData) -> String {
    let account = &data.shell.account;
    let balance = account.balance();
    let low_balance = balance < 20.0;
    let has_models = !data.catalog.is_empty();
    let active_keys = data
        .tokens
        .iter()
        .filter(|token| token.status == "active")
        .count();
    let conclusion = if !data.warnings.is_empty() {
        ("warning", "部分实时数据暂不可用", data.warnings.join("；"))
    } else if account.status != 1 {
        (
            "critical",
            "账户当前不可用",
            "账户状态已被后端标记为停用，请联系管理员恢复后再发送请求。".to_string(),
        )
    } else if low_balance {
        (
            "warning",
            "余额偏低，需要及时充值",
            "当前余额可能影响后续推理请求，请在服务暂停前补充余额。".to_string(),
        )
    } else if !has_models || active_keys == 0 {
        (
            "info",
            "工作区还需要完成配置",
            format!(
                "数据库中检测到 {} 个可用模型、{} 个有效 API 密钥。完成配置后即可发送真实请求。",
                data.catalog.len(),
                active_keys
            ),
        )
    } else {
        (
            "healthy",
            "API 服务与账户状态正常",
            format!(
                "已连接 {} 个模型，当前有 {} 个有效 API 密钥可用于真实路由。",
                data.catalog.len(),
                active_keys
            ),
        )
    };

    let metrics = format!(
        r#"<section class="metrics" aria-label="今日核心指标">{}{}{}{}</section>"#,
        metric(
            "今日费用",
            &format!("${:.4}", data.billing.total_cost_usd),
            "从当前账户的结算日志汇总",
            None
        ),
        metric(
            "账户余额",
            &data.shell.balance_label,
            "数据库账户钱包实时余额",
            Some(if low_balance { "LOW" } else { "HEALTHY" })
        ),
        metric(
            "API 可用性",
            if has_models { "在线" } else { "待配置" },
            &format!("{} 个模型可由启用渠道提供", data.catalog.len()),
            Some(if has_models { "HEALTHY" } else { "EMPTY" })
        ),
        metric(
            "今日 Token",
            &compact_number(data.billing.total_tokens()),
            &format!(
                "{} 次已结算请求",
                data.billing
                    .models
                    .iter()
                    .map(|model| model.requests)
                    .sum::<i64>()
                    + data.billing.pre_migration_requests
            ),
            None
        ),
    );

    let attention = if low_balance {
        format!(
            r#"<section id="attention" class="attention" role="alert">{}<div class="attention-copy"><h2>余额不足可能导致服务中断</h2><p>当前余额为 {}。请充值或检查预算策略，确保生产请求持续可用。</p></div><a class="button primary" href="/buyer/billing">{}<span>立即充值</span></a></section>"#,
            icon("alert-triangle"),
            escape_html(&data.shell.balance_label),
            icon("card")
        )
    } else if !data.warnings.is_empty() {
        format!(
            r#"<section id="attention" class="attention neutral" role="status">{}<div class="attention-copy"><h2>实时数据连接不完整</h2><p>{}</p></div><a class="button secondary" href="/buyer/overview">重试</a></section>"#,
            icon("info"),
            escape_html(&data.warnings.join("；"))
        )
    } else {
        String::new()
    };

    let content = format!(
        r#"<div class="overview-page"><header class="page-header"><div class="page-heading"><p class="eyebrow">BUYER WORKSPACE</p><h1>概览</h1><p>查看今日用量、账户余额与 API 服务状态。</p></div><div class="page-actions"><a class="button secondary" href="/buyer/playground">{}<span>打开操练场</span></a><a class="button primary" href="/buyer/marketplace">{}<span>浏览模型市场</span></a></div></header><section class="conclusion {}" aria-live="polite">{}<div><strong>{}</strong><p>{}</p></div></section>{metrics}{attention}{}{}</div>"#,
        icon("terminal"),
        icon("store"),
        conclusion.0,
        status_icon(conclusion.0),
        conclusion.1,
        escape_html(&conclusion.2),
        render_models(data),
        render_activity(data),
    );

    render_buyer_shell(
        content,
        &data.shell,
        "/buyer/overview",
        "Buyer Overview - BurnCloud",
        "BurnCloud Buyer API 用量与账户概览",
        "",
    )
}

fn metric(label: &str, value: &str, subtitle: &str, badge: Option<&str>) -> String {
    let badge = badge.map_or_else(String::new, |text| {
        let tone = if text == "HEALTHY" {
            "healthy"
        } else {
            "warning"
        };
        format!(r#"<span class="metric-badge {tone}">{text}</span>"#)
    });
    format!(
        r#"<article class="metric"><div class="metric-label"><span>{}</span>{badge}</div><strong>{}</strong><p>{}</p></article>"#,
        escape_html(label),
        escape_html(value),
        escape_html(subtitle)
    )
}

fn render_models(data: &OverviewData) -> String {
    if data.billing.models.is_empty() {
        return format!(
            r#"<section class="content-section onboarding"><div><p class="section-kicker">开始使用</p><h2>尚无已结算的模型调用</h2><p>选择数据库中已配置的模型，并使用有效 API 密钥执行首个真实请求。</p></div><ol><li><span>1</span><div><strong>确认 API 密钥</strong><p>当前有 {} 个有效密钥。</p></div></li><li><span>2</span><div><strong>选择模型</strong><p>模型目录提供 {} 个可用模型。</p></div></li><li><span>3</span><div><strong>运行请求</strong><p>操练场通过真实 BurnCloud 路由执行。</p></div></li></ol><a class="button primary" href="/buyer/playground">{}<span>开始测试</span></a></section>"#,
            data.tokens
                .iter()
                .filter(|token| token.status == "active")
                .count(),
            data.catalog.len(),
            icon("play")
        );
    }

    let catalog = data
        .catalog
        .iter()
        .map(|model| (model.id.as_str(), model))
        .collect::<HashMap<_, _>>();
    let mut models = data.billing.models.clone();
    models.sort_by(|left, right| {
        right
            .cost_usd
            .partial_cmp(&left.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let rows = models.iter().take(5).map(|usage| {
        let model = catalog.get(usage.model.as_str()).copied();
        let provider = model.and_then(|value| value.providers.first()).map(String::as_str).unwrap_or("BurnCloud Router");
        let latency = model.and_then(|value| value.p95_latency_ms).map_or_else(|| "待采样".to_string(), |value| format!("{value} ms"));
        let available = model.is_some_and(|value| value.available_channels > 0);
        format!(r#"<tr><td data-label="模型"><span class="model-mark">{}</span><span><strong>{}</strong><small>{}</small></span></td><td data-label="今日 Token">{}</td><td data-label="请求数">{}</td><td data-label="P95 延迟">{}</td><td data-label="今日费用">${:.6}</td><td data-label="服务状态"><span class="status {}"><span></span>{}</span></td><td><a class="row-action" href="/buyer/playground?model={}">测试</a></td></tr>"#, escape_html(&usage.model.chars().next().unwrap_or('M').to_string()), escape_html(&usage.model), escape_html(provider), compact_number(usage.total_tokens()), usage.requests, latency, usage.cost_usd, if available { "healthy" } else { "degraded" }, if available { "可用" } else { "当前未暴露" }, escape_html(&usage.model))
    }).collect::<String>();
    format!(
        r#"<section class="content-section"><div class="section-header"><div><p class="section-kicker">当前服务</p><h2>正在使用的模型</h2><p>今日真实结算用量与当前模型目录可用状态。</p></div><a class="text-link" href="/buyer/marketplace">查看全部模型 <span aria-hidden="true">→</span></a></div><div class="table-wrap"><table><thead><tr><th>模型</th><th>今日 Token</th><th>请求数</th><th>P95 延迟</th><th>今日费用</th><th>服务状态</th><th></th></tr></thead><tbody>{rows}</tbody></table></div></section>"#
    )
}

fn render_activity(data: &OverviewData) -> String {
    let mut items = Vec::<String>::new();
    for recharge in data.recharges.iter().take(3) {
        let symbol = if recharge.currency.as_deref() == Some("CNY") {
            "¥"
        } else {
            "$"
        };
        items.push(format!(r#"<li><span class="activity-icon">{}</span><div><strong>账户充值 {}{:.2}</strong><p>{}</p></div><time>{}</time></li>"#, icon("card"), symbol, recharge.amount as f64 / 1_000_000_000.0, escape_html(recharge.description.as_deref().unwrap_or("充值记录已写入账户")), escape_html(recharge.created_at.as_deref().unwrap_or("时间未知"))));
    }
    for token in data.tokens.iter().take(2) {
        items.push(format!(r#"<li><span class="activity-icon">{}</span><div><strong>API 密钥 {}</strong><p>{} · 数据面密钥未向页面暴露</p></div><time>{}</time></li>"#, icon("key"), escape_html(if token.token_hint.is_empty() { "已创建" } else { &token.token_hint }), escape_html(&token.status), if token.created_at > 0 { token.created_at.to_string() } else { "时间未知".to_string() }));
    }
    if items.is_empty() {
        items.push(format!(r#"<li><span class="activity-icon">{}</span><div><strong>尚无账户活动</strong><p>充值或创建 API 密钥后，数据库记录会显示在这里。</p></div><time>--</time></li>"#, icon("logs")));
    }
    format!(
        r#"<section class="content-section"><div class="section-header"><div><p class="section-kicker">账户事件</p><h2>最近活动</h2><p>来自充值与 API 密钥记录的最新账户变化。</p></div><a class="text-link" href="/buyer/logs">查看调用日志 <span aria-hidden="true">→</span></a></div><ul class="activity-list">{}</ul></section>"#,
        items.join("")
    )
}

pub fn render_buyer_shell(
    content: String,
    shell: &ShellContext,
    active_path: &str,
    page_title: &str,
    page_description: &str,
    extra_head: &str,
) -> String {
    let account = &shell.account;
    let username = escape_html(&account.username);
    let initial = account
        .username
        .chars()
        .next()
        .unwrap_or('B')
        .to_uppercase()
        .to_string();
    let admin_item = if account.is_admin() {
        r#"<a href="/admin/overview" data-role="admin"><span class="role-dot admin"></span><span><strong>平台管理员</strong><small>平台治理与运营</small></span></a>"#.to_string()
    } else {
        r#"<span class="role-disabled" title="当前账户没有 admin 权限"><span class="role-dot admin"></span><span><strong>平台管理员</strong><small>需要 admin 权限</small></span></span>"#.to_string()
    };
    let role_menu = format!(
        r#"<div class="role-menu" data-menu-panel hidden><p>切换工作区角色</p><a href="/buyer/overview" class="selected" data-role="buyer"><span class="role-dot buyer"></span><span><strong>API 采购方</strong><small>模型、调用与账单</small></span></a><a href="/supplier/overview" data-role="supplier"><span class="role-dot supplier"></span><span><strong>算力供应方</strong><small>资源、部署与收益</small></span></a>{admin_item}</div>"#
    );
    let nav = [
        ("概览", "/buyer/overview", "layout"),
        ("操练场", "/buyer/playground", "terminal"),
        ("模型市场", "/buyer/marketplace", "store"),
        ("API 密钥", "/buyer/api-keys", "key"),
        ("用量分析", "/buyer/usage", "chart"),
        ("账单与余额", "/buyer/billing", "card"),
        ("调用日志", "/buyer/logs", "logs"),
    ];
    let links = nav
        .iter()
        .map(|(label, href, icon_name)| {
            format!(
                r#"<a href="{}"{}>{}<span>{}</span></a>"#,
                href,
                if *href == active_path {
                    " class=\"active\" aria-current=\"page\""
                } else {
                    ""
                },
                icon(icon_name),
                label
            )
        })
        .collect::<String>();
    let notification = if shell.attention {
        format!(
            r##"<a class="notification" href="/buyer/overview#attention" aria-label="需要处理的通知">{}<span></span></a>"##,
            icon("bell")
        )
    } else {
        format!(
            r#"<a class="notification" href="/buyer/overview" aria-label="通知">{}</a>"#,
            icon("bell")
        )
    };
    let search_items = nav
        .iter()
        .map(|(label, href, _)| {
            format!(r#"<a href="{href}" data-global-result data-search="{label}">{label}</a>"#)
        })
        .collect::<String>();
    let notification = format!(
        "{}{notification}",
        render_language_switcher("topbar-language-switcher")
    );
    let extra_head = format!(r#"<script src="/assets/i18n.js" defer></script>{extra_head}"#);

    format!(
        r##"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="description" content="{}"><title>{}</title><link rel="stylesheet" href="/assets/styles.css"><script src="/assets/app.js" defer></script>{}</head><body><div class="app-shell"><button class="mobile-menu" type="button" aria-label="打开导航" aria-controls="sidebar" aria-expanded="false">{}</button><aside id="sidebar" class="sidebar"><div class="workspace menu-root"><button type="button" class="workspace-switch" data-menu-trigger aria-expanded="false"><span class="brand">{}<span><strong>BurnCloud</strong><small><i class="role-dot buyer"></i>API 采购方 · Workspace</small></span></span>{}</button>{}<button type="button" class="sidebar-close" data-close-sidebar aria-label="关闭导航">{}</button></div><p class="mental-model">模型 → API → 用量 → 账单</p><nav aria-label="Buyer 导航">{}</nav><div class="sidebar-footer"><div><small>账户余额</small><strong>{}</strong><a href="/buyer/billing">充值</a></div><p class="healthy"><span></span>账户数据已连接</p></div></aside><div class="sidebar-backdrop" data-close-sidebar></div><div class="main-column"><header class="topbar"><button class="topbar-menu" type="button" aria-label="打开导航" aria-controls="sidebar" aria-expanded="false">{}</button><div class="global-search"><label for="global-search">{}</label><input id="global-search" type="search" autocomplete="off" placeholder="搜索页面、模型或功能…" aria-controls="global-search-results" aria-expanded="false"><kbd>Ctrl K</kbd><div id="global-search-results" class="global-search-results" hidden>{search_items}</div></div><div class="autopilot"><span></span>Autopilot 已启用</div>{notification}<div class="profile-menu menu-root"><button class="profile" type="button" data-menu-trigger aria-expanded="false"><span>{}</span><div><strong>{username}</strong><small>API 采购方</small></div>{}</button><div class="profile-dropdown" data-menu-panel hidden><div class="profile-account"><strong>{username}</strong><small>{}</small></div><p>切换角色</p><a href="/buyer/overview"><span class="role-dot buyer"></span>API 采购方</a><a href="/supplier/overview"><span class="role-dot supplier"></span>算力供应方</a>{}<form method="post" action="/session/logout"><button type="submit">{}<span>退出登录</span></button></form></div></div></header><main id="main-content" tabindex="-1"><div class="page-wrap">{}</div></main></div></div></body></html>"##,
        escape_html(page_description),
        escape_html(page_title),
        extra_head,
        icon("menu"),
        logo(),
        icon("chevron-down"),
        role_menu,
        icon("x"),
        links,
        escape_html(&shell.balance_label),
        icon("menu"),
        icon("search"),
        escape_html(&initial),
        icon("chevron-down"),
        escape_html(account.email.as_deref().unwrap_or("BurnCloud 账户")),
        if account.is_admin() {
            r#"<a href="/admin/overview"><span class="role-dot admin"></span>平台管理员</a>"#
        } else {
            r#"<span class="profile-role-disabled"><span class="role-dot admin"></span>平台管理员 · 无权限</span>"#
        },
        icon("logout"),
        content
    )
}

pub fn render_role_placeholder(shell: &ShellContext, role: &str, allowed: bool) -> String {
    let (title, copy) = match role {
        "supplier" => (
            "算力供应方工作区尚未迁移",
            "角色切换已生效，但 Supplier 页面不在本次前三页迁移范围内。返回 Buyer 工作区可继续使用完整的前三页功能。",
        ),
        _ if !allowed => (
            "没有管理员权限",
            "当前账户的数据库角色中不包含 admin，因此不能进入平台管理员工作区。",
        ),
        _ => (
            "平台管理员工作区尚未迁移",
            "管理员权限已经验证，但 Admin 页面不在本次前三页迁移范围内。",
        ),
    };
    let content = format!(
        r#"<section class="role-placeholder"><p class="eyebrow">ROLE WORKSPACE</p><h1>{title}</h1><p>{copy}</p><a class="button primary" href="/buyer/overview">返回 Buyer 概览</a></section>"#
    );
    render_buyer_shell(content, shell, "", "角色工作区 - BurnCloud", copy, "")
}

pub fn compact_number(value: i64) -> String {
    let absolute = value.unsigned_abs();
    if absolute >= 1_000_000 {
        format!("{:.2}M", value as f64 / 1_000_000.0)
    } else if absolute >= 1_000 {
        format!("{:.1}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn status_icon(tone: &str) -> &'static str {
    match tone {
        "healthy" => icon("check-circle"),
        "info" => icon("info"),
        "critical" => icon("alert-circle"),
        _ => icon("alert-triangle"),
    }
}

fn logo() -> &'static str {
    r##"<svg class="logo" viewBox="0 0 24 24" aria-hidden="true"><path fill="#ed6a28" d="M17.8 10.1q-.6-.9-1.4-1.9S14.6 6.1 14.9 3c0 0-6.9 2.7-7 8.2 0 0-1-1.6-.8-4.6 0 0-2.2 2.1-2.5 5.5-2.1.7-3.8 2.5-3.8 4.3 0 2.5 2.7 4.6 5.9 4.6-2.4-.4-4.2-2-4.2-4 0-1.4.8-2.5 2-3.3q.1 1.1.5 2.4s1.2 3.8 5.4 4.8c1.2.3 2.5.2 3.7-.3 1.3-.6 2.8-1.8 2.8-4.5 0 0 .1-2.7-1.5-4.1 0 0 2.1 5-1.8 6.5-1.3.5-2.6.5-3.9 0-1.7-.7-3.8-2.5-3.5-7.2 0 0 1 3.4 3.2 4.7 0 0-2-5.8 3.9-9.8 0 0 .5 2.1 1.9 3.3.4.4 4 3.2 3.3 8 .7-.9 1.3-3.1.7-4.8 0 0-.1-.4-.4-.9 1.5.3 2.7 1.5 2.8 4.2.1 2.3-1.6 4.2-3.8 5 3-.4 5.4-2.7 5.4-5.6 0-2.8-2.2-5.1-5.4-5.3z"/></svg>"##
}

pub fn icon(name: &str) -> &'static str {
    match name {
        "menu" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16"/></svg>"#
        }
        "x" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 6 12 12M18 6 6 18"/></svg>"#
        }
        "bell" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4"/></svg>"#
        }
        "search" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m16 16 4 4"/></svg>"#
        }
        "chevron-down" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg>"#
        }
        "logout" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M10 17l5-5-5-5M15 12H3M15 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-4"/></svg>"#
        }
        "layout" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="3" y="3" width="7" height="9" rx="1"/><rect x="14" y="3" width="7" height="5" rx="1"/><rect x="14" y="12" width="7" height="9" rx="1"/><rect x="3" y="16" width="7" height="5" rx="1"/></svg>"#
        }
        "terminal" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m4 17 6-5-6-5M12 19h8"/></svg>"#
        }
        "cpu" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="6" y="6" width="12" height="12" rx="2"/><rect x="9" y="9" width="6" height="6"/><path d="M9 1v3M15 1v3M9 20v3M15 20v3M20 9h3M20 14h3M1 9h3M1 14h3"/></svg>"#
        }
        "code" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m8 9-3 3 3 3M16 9l3 3-3 3M14 5l-4 14"/></svg>"#
        }
        "copy" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></svg>"#
        }
        "shield" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10Z"/><path d="m9 12 2 2 4-4"/></svg>"#
        }
        "rotate" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7L3 8"/><path d="M3 3v5h5"/></svg>"#
        }
        "play" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="m7 4 13 8-13 8V4Z"/></svg>"#
        }
        "store" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 9h18l-2-5H5L3 9Zm1 0v11h16V9M9 20v-6h6v6"/></svg>"#
        }
        "key" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="8" cy="15" r="4"/><path d="m11 12 8-8M16 7l2 2M14 9l2 2"/></svg>"#
        }
        "chart" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20V10M10 20V4M16 20v-7M22 20H2"/></svg>"#
        }
        "card" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><rect x="2" y="5" width="20" height="14" rx="2"/><path d="M2 10h20"/></svg>"#
        }
        "logs" => {
            r#"<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 3h12v18l-3-2-3 2-3-2-3 2V3Zm3 5h6M9 12h6"/></svg>"#
        }
        "check-circle" => {
            r#"<svg class="state-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="m8 12 2.5 2.5L16 9"/></svg>"#
        }
        "info" => {
            r#"<svg class="state-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 11v5M12 8h.01"/></svg>"#
        }
        "alert-circle" => {
            r#"<svg class="state-icon" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="9"/><path d="M12 7v6M12 17h.01"/></svg>"#
        }
        "alert-triangle" => {
            r#"<svg class="state-icon" viewBox="0 0 24 24" aria-hidden="true"><path d="M10.3 3.8 2.2 18a2 2 0 0 0 1.7 3h16.2a2 2 0 0 0 1.7-3L13.7 3.8a2 2 0 0 0-3.4 0ZM12 9v4M12 17h.01"/></svg>"#
        }
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::{OverviewData, ShellContext, compact_number, render_overview};
    use crate::backend::{BillingSummary, CatalogModel, CurrentAccount};

    fn data() -> OverviewData {
        OverviewData {
            shell: ShellContext {
                account: CurrentAccount {
                    username: "buyer@example.com".to_string(),
                    status: 1,
                    balance_usd: 128_500_000_000,
                    roles: vec!["user".to_string()],
                    ..CurrentAccount::default()
                },
                balance_label: "$128.50".to_string(),
                attention: false,
            },
            billing: BillingSummary::default(),
            catalog: vec![CatalogModel {
                id: "model-a".to_string(),
                available_channels: 1,
                ..CatalogModel::default()
            }],
            tokens: Vec::new(),
            recharges: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn renders_authenticated_shell_search_and_role_switchers() {
        let page = render_overview(&data());
        assert!(page.contains("id=\"global-search\""));
        assert!(page.contains("data-role=\"buyer\""));
        assert!(page.contains("data-role=\"supplier\""));
        assert!(page.contains("平台管理员"));
        assert!(page.contains("buyer@example.com"));
        assert!(page.contains("/assets/i18n.js"));
        for language in ["en", "zh", "zh-TW", "ja"] {
            assert!(page.contains(&format!("data-language-option=\"{language}\"")));
        }
    }

    #[test]
    fn formats_metric_values() {
        assert_eq!(compact_number(1_840_000), "1.84M");
        assert_eq!(compact_number(1_200), "1.2K");
        assert_eq!(compact_number(12), "12");
    }
}
