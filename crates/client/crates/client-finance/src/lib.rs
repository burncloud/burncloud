use burncloud_client_shared::auth_context::use_auth;
use burncloud_client_shared::components::{
    BCButton, PageHeader,
    SkeletonCard, SkeletonVariant, StatKpi,
};
use burncloud_client_shared::services::billing_service::BillingService;
use dioxus::prelude::*;

/// Format nanodollars (i64, 10^9 precision) to a readable USD string with 2 decimal places.
fn format_nano_usd(nano: i64) -> String {
    let usd = nano as f64 / 1_000_000_000.0;
    format!("${usd:.2}")
}

/// Format nanodollars (i64, 10^9 precision) to a readable CNY string with 2 decimal places.
fn format_nano_cny(nano: i64) -> String {
    let cny = nano as f64 / 1_000_000_000.0;
    format!("¥{cny:.2}")
}

fn format_cents(cents: i64) -> String {
    let yuan = cents as f64 / 100.0;
    format!("¥ {yuan:.2}")
}

#[component]
pub fn FinancePage() -> Element {
    let auth = use_auth();
    let current_user = auth.get_user();
    let user_id = current_user.map(|u| u.id).unwrap_or_default();

    let billing = use_resource(move || {
        let uid = user_id.clone();
        async move {
            if uid.is_empty() {
                return Err("Not authenticated".to_string());
            }
            BillingService::get_billing_summary(&uid, None, None).await
        }
    });

    let billing_data = billing.read().clone();
    let loading = billing_data.is_none();
    let billing_result = billing_data.as_ref().and_then(|r| r.clone().ok());
    let billing_error = billing_data.and_then(|r| r.err().map(|e| e.to_string()));

    let balance_usd = billing_result.as_ref().map(|b| b.balance_usd).unwrap_or(0);
    let balance_cny = billing_result.as_ref().map(|b| b.balance_cny).unwrap_or(0);
    let total_cost_usd = billing_result.as_ref().map(|b| b.total_cost_usd).unwrap_or(0.0);

    let recharge_records = vec![
        ("RECH-1042", "2026-04-26 14:22", "微信支付 · 充值", 200000, "success"),
        ("RECH-1041", "2026-04-22 09:18", "对公转账 · 充值", 500000, "success"),
        ("RECH-1038", "2026-04-15 16:45", "支付宝 · 自动续充", 50000, "success"),
        ("RECH-1031", "2026-04-08 11:02", "微信支付 · 充值", 100000, "success"),
        ("RECH-1024", "2026-03-30 19:10", "对公转账 · 充值", 300000, "success"),
        ("RECH-1020", "2026-03-22 08:34", "退款 · 误充", -12000, "refund"),
    ];

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
            if let Some(err) = billing_error {
                div { class: "error-banner", style: "padding:12px 16px; background:var(--bc-danger-bg); border:1px solid var(--bc-danger); border-radius:8px; color:var(--bc-danger); font-size:13px",
                    "加载账单数据失败: {err}"
                }
            }

            // Billing KPIs — Balance cards first (most important per design review)
            div { class: "stats-grid",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "USD 余额".to_string(),
                        value: format_nano_usd(balance_usd),
                        large: Some(true),
                    }
                    StatKpi {
                        label: "CNY 余额".to_string(),
                        value: format_nano_cny(balance_cny),
                        large: Some(true),
                    }
                    StatKpi {
                        label: "本月支出".to_string(),
                        value: format!("${total_cost_usd:.2}"),
                        large: Some(true),
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
                } else {
                    table { class: "table",
                        thead {
                            tr {
                                th { style: "width:140px", "充值单号" }
                                th { style: "width:160px", "时间" }
                                th { "说明" }
                                th { style: "width:120px; text-align:right", "金额" }
                                th { style: "width:100px", "状态" }
                            }
                        }
                        tbody {
                            for (id, time, desc, amount, status) in &recharge_records {
                                tr {
                                    key: "{id}",
                                    td { class: "mono", style: "font-size:13px", "{id}" }
                                    td { class: "mono", style: "font-size:13px; color:var(--bc-text-secondary)", "{time}" }
                                    td { style: "font-size:13px", "{desc}" }
                                    td { class: "mono", style: "text-align:right; font-weight:600; font-variant-numeric:tabular-nums; color:if *status == \"refund\" {{ \"var(--bc-danger)\" }} else {{ \"var(--bc-text-primary)\" }}",
                                        "{format_cents(*amount)}"
                                    }
                                    td {
                                        if *status == "success" {
                                            span { class: "pill success", span { class: "dot" } "成功" }
                                        } else {
                                            span { class: "pill warning", span { class: "dot" } "已退款" }
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
}
