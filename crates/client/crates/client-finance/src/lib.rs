use burncloud_client_shared::billing_service::BillingService;
use burncloud_client_shared::components::{
    EmptyState, ErrorBanner, PageHeader, SkeletonCard, SkeletonVariant, StatKpi,
};
use burncloud_client_shared::i18n::{t, t_fmt};
use burncloud_client_shared::services::usage_service::UsageService;
use burncloud_client_shared::use_auth;
use dioxus::prelude::*;

fn format_usd(usd: f64) -> String {
    if usd >= 1.0 {
        format!("$ {usd:.2}")
    } else if usd >= 0.01 {
        format!("$ {usd:.4}")
    } else if usd > 0.0 {
        format!("$ {usd:.6}")
    } else {
        "$ 0.00".to_string()
    }
}

fn format_thousands(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_compact(n: i64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1e9)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1e6)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}

#[component]
pub fn Finance() -> Element {
    let i18n = burncloud_client_shared::i18n::use_i18n();
    let lang = i18n.language;
    let auth = use_auth();
    let token = auth.get_token().unwrap_or_default();
    let token_for_billing = token.clone();
    let token_for_recharges = token.clone();

    let billing = use_resource(move || {
        let tok = token_for_billing.clone();
        async move {
            if tok.is_empty() {
                Err("Not authenticated".to_string())
            } else {
                BillingService::get_billing_summary(&tok).await
            }
        }
    });

    let recharges = use_resource(move || {
        let tok = token_for_recharges.clone();
        async move {
            if tok.is_empty() {
                Err("Not authenticated".to_string())
            } else {
                UsageService::list_recharges(&tok).await
            }
        }
    });

    let b = billing.read().clone();
    let r = recharges.read().clone();

    let billing_loading = b.is_none();
    let billing_error = b.as_ref().and_then(|res| res.as_ref().err().cloned());
    let billing_summary = b.and_then(|res| res.ok());

    let recharges_error = r.as_ref().and_then(|res| res.as_ref().err().cloned());
    let recharge_list = r.and_then(|res| res.ok()).unwrap_or_default();

    // Billing-derived KPIs
    let total_cost = billing_summary
        .as_ref()
        .map(|s| s.total_cost_usd)
        .unwrap_or(0.0);
    let total_requests: i64 = billing_summary
        .as_ref()
        .map(|s| s.models.iter().map(|m| m.requests).sum::<i64>() + s.pre_migration_requests)
        .unwrap_or(0);
    let total_prompt: i64 = billing_summary
        .as_ref()
        .map(|s| s.models.iter().map(|m| m.prompt_tokens).sum())
        .unwrap_or(0);
    let total_completion: i64 = billing_summary
        .as_ref()
        .map(|s| s.models.iter().map(|m| m.completion_tokens).sum())
        .unwrap_or(0);
    let total_tokens = total_prompt + total_completion;
    let model_count = billing_summary
        .as_ref()
        .map(|s| s.models.len())
        .unwrap_or(0);

    let cost_str = format_usd(total_cost);
    let req_str = format_thousands(total_requests);
    let token_str = format_compact(total_tokens);

    rsx! {
        PageHeader {
            title: t(*lang.read(), "finance.title"),
            subtitle: Some(t(*lang.read(), "finance.subtitle").to_string()),
        }

        div { class: "page-content flex flex-col gap-xxxl",
            // Error banners
            if let Some(ref err) = billing_error {
                ErrorBanner {
                    message: t_fmt(*lang.read(), "finance.error.billing_load", &[("error", err)]),
                    on_retry: None,
                }
            }
            if let Some(ref err) = recharges_error {
                ErrorBanner {
                    message: t_fmt(*lang.read(), "finance.error.recharge_load", &[("error", err)]),
                    on_retry: None,
                }
            }

            // 3 KPI cards — real billing data
            div { class: "stats-grid cols-3",
                if billing_loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else if billing_summary.is_none() && billing_error.is_none() {
                    EmptyState {
                        icon: rsx! { span { class: "text-xxl", "💰" } },
                        title: t(*lang.read(), "finance.empty.billing_title").to_string(),
                        description: Some(t(*lang.read(), "finance.empty.billing_desc").to_string()),
                        cta: None,
                    }
                } else {
                    StatKpi {
                        label: "TOTAL COST".to_string(),
                        value: cost_str,
                        delta: rsx! { span { class: "stat-foot", "{req_str} requests" } },
                        chart: None,
                    }
                    StatKpi {
                        label: "TOKEN USAGE".to_string(),
                        value: token_str,
                        delta: rsx! { span { class: "stat-foot", "{model_count} models" } },
                        chart: None,
                    }
                    StatKpi {
                        label: "AVG COST / REQ".to_string(),
                        value: if total_requests > 0 {
                            format_usd(total_cost / total_requests as f64)
                        } else {
                            "$ —".to_string()
                        },
                        delta: rsx! { span { class: "stat-foot", "prompt {format_compact(total_prompt)} · completion {format_compact(total_completion)}" } },
                        chart: None,
                    }
                }
            }

            // Model cost breakdown table
            if let Some(summary) = &billing_summary {
                if !summary.models.is_empty() {
                    div {
                        div { class: "section-h",
                            span { class: "lead-title", {t(*lang.read(), "finance.model_breakdown.title")} }
                            span { class: "section-sub", "{summary.models.len()} models" }
                        }
                        table { class: "table",
                            thead {
                                tr {
                                    th { "MODEL" }
                                    th { class: "text-right", "REQUESTS" }
                                    th { class: "text-right", "PROMPT" }
                                    th { class: "text-right", "COMPLETION" }
                                    th { class: "text-right", "COST" }
                                }
                            }
                            tbody {
                                for m in &summary.models {
                                    tr {
                                        key: "{m.model}",
                                        td { class: "font-medium", "{m.model}" }
                                        td { class: "mono text-right", "{format_thousands(m.requests)}" }
                                        td { class: "mono text-right", "{format_compact(m.prompt_tokens)}" }
                                        td { class: "mono text-right", "{format_compact(m.completion_tokens)}" }
                                        td { class: "mono text-right font-semibold", "{format_usd(m.cost_usd)}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Recharge history
            div {
                div { class: "section-h",
                    span { class: "lead-title", {t(*lang.read(), "finance.recharge_history.title")} }
                    span { class: "section-sub", "{recharge_list.len()} records" }
                }

                if recharge_list.is_empty() {
                    EmptyState {
                        icon: rsx! { span { class: "text-xxl", "💳" } },
                        title: t(*lang.read(), "finance.empty.recharge_title").to_string(),
                        description: Some(t(*lang.read(), "finance.empty.recharge_desc").to_string()),
                        cta: None,
                    }
                } else {
                    table { class: "table",
                        thead {
                            tr {
                                th { "ID" }
                                th { "AMOUNT" }
                                th { "DESCRIPTION" }
                                th { "DATE" }
                            }
                        }
                        tbody {
                            for r in &recharge_list {
                                tr {
                                    key: "{r.id}",
                                    td { class: "mono", "#{r.id}" }
                                    td { class: "mono font-semibold",
                                        // Amount is in nanodollars (9 decimal precision)
                                        "{format_usd(r.amount as f64 / 1e9)}"
                                    }
                                    td { class: "text-secondary",
                                        {r.description.as_deref().unwrap_or("—")}
                                    }
                                    td { class: "mono text-secondary",
                                        {r.created_at.as_deref().unwrap_or("—")}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
