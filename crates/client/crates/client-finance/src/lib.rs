use burncloud_client_shared::billing_service::BillingService;
use burncloud_client_shared::components::{
    BCButton, PageHeader,
    SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::usage_service::UsageService;
use burncloud_client_shared::use_auth;
use dioxus::prelude::*;

fn format_usd(nanodollars: i64) -> String {
    let usd = nanodollars as f64 / 1_000_000_000.0;
    format!("$ {usd:.4}")
}

fn format_usd_f(usd: f64) -> String {
    format!("$ {usd:.4}")
}

#[component]
pub fn FinancePage() -> Element {
    let auth = use_auth();
    let user_id = auth.get_user().map(|u| u.id).unwrap_or_default();
    let token = auth.get_token().unwrap_or_default();

    let billing = use_resource(move || {
        let t = token.clone();
        async move {
            if t.is_empty() {
                Err("Not authenticated".to_string())
            } else {
                BillingService::get_billing_summary(&t).await
            }
        }
    });

    let recharges = use_resource(move || {
        let uid = user_id.clone();
        async move {
            if uid.is_empty() {
                Err("Not authenticated".to_string())
            } else {
                UsageService::list_recharges(&uid).await
            }
        }
    });

    let b = billing.read().clone();
    let r = recharges.read().clone();

    let loading = b.is_none() && r.is_none();

    let billing_summary = match &b {
        Some(Ok(data)) => Some(data.clone()),
        _ => None,
    };

    let recharge_list = match &r {
        Some(Ok(data)) => data.clone(),
        _ => vec![],
    };

    let total_cost_usd = billing_summary
        .as_ref()
        .map(|s| s.total_cost_usd)
        .unwrap_or(0.0);

    let model_count = billing_summary
        .as_ref()
        .map(|s| s.models.len())
        .unwrap_or(0);

    rsx! {
        PageHeader {
            title: "财务中心",
            subtitle: Some("管理您的账户余额、充值记录与收支统计".to_string()),
            actions: rsx! {
                BCButton {
                    class: "btn-primary",
                    onclick: move |_| {},
                    "充值余额"
                }
            },
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:24px",
            // Billing KPIs
            div { class: "stats-grid",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    div { class: "stat-card", style: "gap:8px",
                        span { class: "stat-eyebrow", "累计支出" }
                        div { class: "stat-value lg", style: "font-variant-numeric:tabular-nums",
                            "{format_usd_f(total_cost_usd)} "
                            span { class: "stat-pill", "{model_count} models" }
                        }
                    }
                    div { class: "stat-card", style: "gap:8px",
                        span { class: "stat-eyebrow", "模型明细" }
                        div { class: "stat-value lg", style: "font-variant-numeric:tabular-nums",
                            if let Some(summary) = &billing_summary {
                                for m in &summary.models {
                                    div { key: "{m.model}", style: "font-size:13px; margin:2px 0",
                                        "{m.model}: {format_usd_f(m.cost_usd)} ({m.requests} req)"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "stat-card", style: "gap:8px",
                        span { class: "stat-eyebrow", "充值记录" }
                        div { class: "stat-value lg", style: "font-variant-numeric:tabular-nums; color:var(--bc-text-secondary)",
                            "{recharge_list.len()} records"
                        }
                    }
                }
            }

            // Recharge records
            div {
                div { class: "section-h",
                    span { class: "lead-title", "充值记录" }
                }

                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                } else if recharge_list.is_empty() {
                    div { class: "empty-state", style: "padding:32px; text-align:center; color:var(--bc-text-secondary)",
                        "暂无充值记录"
                    }
                } else {
                    table { class: "table",
                        thead {
                            tr {
                                th { style: "width:100px", "ID" }
                                th { style: "width:160px", "时间" }
                                th { "说明" }
                                th { style: "width:140px; text-align:right", "金额" }
                            }
                        }
                        tbody {
                            for rec in &recharge_list {
                                tr {
                                    key: "{rec.id}",
                                    td { class: "mono", style: "font-size:13px", "#{rec.id}" }
                                    td { class: "mono", style: "font-size:13px; color:var(--bc-text-secondary)",
                                        "{rec.created_at.as_deref().unwrap_or(\"—\")}"
                                    }
                                    td { style: "font-size:13px", "{rec.description.as_deref().unwrap_or(\"充值\")}" }
                                    td { class: "mono", style: "text-align:right; font-weight:600; font-variant-numeric:tabular-nums",
                                        "{format_usd(rec.amount)}"
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
