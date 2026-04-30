use burncloud_client_shared::components::{
    BCButton, PageHeader,
    SkeletonCard, SkeletonVariant,
};
use dioxus::prelude::*;

fn format_cents(cents: i64) -> String {
    let yuan = cents as f64 / 100.0;
    format!("¥ {yuan:.2}")
}

#[component]
pub fn FinancePage() -> Element {
    let loading = false;

    // Mock billing data matching design
    let balance = 523000; // cents → ¥ 5,230.00
    let month_spend = 1245000; // → ¥ 12,450.00
    let est_next = 1800000; // → ¥ 18,000.00

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
            // Billing KPIs
            div { class: "stats-grid",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    div { class: "stat-card", style: "gap:8px",
                        span { class: "stat-eyebrow", "本月支出" }
                        div { class: "stat-value lg", style: "font-variant-numeric:tabular-nums",
                            "{format_cents(month_spend)} "
                            span { class: "stat-pill danger", "+15%" }
                        }
                    }
                    div { class: "stat-card", style: "gap:8px",
                        span { class: "stat-eyebrow", "账户余额" }
                        div { class: "stat-value lg", style: "font-variant-numeric:tabular-nums", "{format_cents(balance)}" }
                    }
                    div { class: "stat-card", style: "gap:8px",
                        span { class: "stat-eyebrow", "预估下月" }
                        div { class: "stat-value lg", style: "font-variant-numeric:tabular-nums; color:var(--bc-text-secondary)", "{format_cents(est_next)}" }
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
