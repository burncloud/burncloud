use burncloud_client_shared::components::{
    BCButton, PageHeader, StatKpi, StatusPill, ColumnDef, PageTable,
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

    let recharge_columns = vec![
        ColumnDef { key: "id".to_string(), label: "充值单号".to_string(), width: Some("140px".to_string()) },
        ColumnDef { key: "time".to_string(), label: "时间".to_string(), width: Some("160px".to_string()) },
        ColumnDef { key: "desc".to_string(), label: "说明".to_string(), width: None },
        ColumnDef { key: "amount".to_string(), label: "金额".to_string(), width: Some("120px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "状态".to_string(), width: Some("100px".to_string()) },
    ];

    rsx! {
        PageHeader {
            title: "财务中心",
            subtitle: Some("管理您的账户余额、充值记录与收支统计".to_string()),
            actions: rsx! {
                BCButton {
                    class: "btn-black",
                    onclick: move |_| {},
                    "充值余额"
                }
            },
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:24px",
            // Billing KPIs
            div { class: "stats-grid cols-3",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "本月支出".to_string(),
                        value: format_cents(month_spend),
                        large: Some(true),
                        delta: rsx! { span { class: "stat-pill danger", "+15%" } },
                    }
                    StatKpi {
                        label: "账户余额".to_string(),
                        value: format_cents(balance),
                        large: Some(true),
                    }
                    StatKpi {
                        label: "预估下月".to_string(),
                        value: format_cents(est_next),
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
                    PageTable {
                        columns: recharge_columns,
                        for (id, time, desc, amount, status) in &recharge_records {
                            tr {
                                key: "{id}",
                                td { class: "mono", style: "font-size:13px", "{id}" }
                                td { class: "mono", style: "font-size:13px; color:var(--bc-text-secondary)", "{time}" }
                                td { style: "font-size:13px", "{desc}" }
                                td { class: "mono", style: "text-align:right; font-weight:600; color:if *status == \"refund\" { \"var(--bc-danger)\" } else { \"var(--bc-text-primary)\" }",
                                    "{format_cents(*amount)}"
                                }
                                td {
                                    StatusPill {
                                        value: if *status == "success" { "ok".to_string() } else { "warning".to_string() },
                                        label: if *status == "success" { Some("成功".to_string()) } else { Some("已退款".to_string()) },
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
