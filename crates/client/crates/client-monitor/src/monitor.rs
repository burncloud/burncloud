use burncloud_client_shared::components::{
    PageHeader, StatKpi, StatusPill, ColumnDef, PageTable,
    SkeletonCard, SkeletonVariant,
};
use dioxus::prelude::*;

fn risk_pill(level: &str) -> Element {
    match level {
        "high" => rsx! { StatusPill { value: "danger".to_string(), label: Some("高危".to_string()) } },
        "medium" => rsx! { StatusPill { value: "warning".to_string(), label: Some("中危".to_string()) } },
        "low" => rsx! { StatusPill { value: "info".to_string(), label: Some("低危".to_string()) } },
        _ => rsx! { StatusPill { value: "neutral".to_string(), label: Some(level.to_string()) } },
    }
}

#[component]
pub fn ServiceMonitor() -> Element {
    let loading = false;

    // Mock risk data matching design
    let risk_events = vec![
        ("2026-04-29 08:12", "Channel-A", "high", "异常流量激增 — 5分钟内请求量超阈值300%"),
        ("2026-04-29 07:45", "Channel-B", "medium", "API Key 调用频率异常 — 单Key QPS超限"),
        ("2026-04-29 06:30", "Channel-C", "low", "延迟波动 — P99延迟上升15%"),
        ("2026-04-28 22:18", "Channel-A", "high", "疑似凭证泄露 — 同一Key多IP并发调用"),
        ("2026-04-28 19:05", "Channel-D", "medium", "错误率上升 — 429占比超10%"),
        ("2026-04-28 15:22", "Channel-B", "low", "配额接近上限 — 已用92%"),
        ("2026-04-28 11:00", "Channel-A", "medium", "模型降级 — 自动切换至备用模型"),
    ];

    let high_count = risk_events.iter().filter(|(_, _, l, _)| *l == "high").count();
    let medium_count = risk_events.iter().filter(|(_, _, l, _)| *l == "medium").count();
    let low_count = risk_events.iter().filter(|(_, _, l, _)| *l == "low").count();

    let columns = vec![
        ColumnDef { key: "time".to_string(), label: "时间".to_string(), width: Some("180px".to_string()) },
        ColumnDef { key: "channel".to_string(), label: "来源".to_string(), width: Some("120px".to_string()) },
        ColumnDef { key: "level".to_string(), label: "风险等级".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "desc".to_string(), label: "描述".to_string(), width: None },
    ];

    rsx! {
        PageHeader {
            title: "风控中心",
            subtitle: Some("实时风险监控与异常事件追踪".to_string()),
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:24px",
            // KPI strip
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "总请求数 · 24H".to_string(),
                        value: "1.24M".to_string(),
                        delta: rsx! { span { class: "stat-foot up", "↑ 8.3%" } },
                    }
                    StatKpi {
                        label: "拦截率".to_string(),
                        value: "0.12%".to_string(),
                        delta: rsx! { span { class: "stat-foot", "1,488 blocked" } },
                    }
                    StatKpi {
                        label: "风险评分".to_string(),
                        value: "23".to_string(),
                        delta: rsx! { span { class: "stat-foot", style: "color:var(--bc-success)", "Low Risk" } },
                    }
                    StatKpi {
                        label: "异常事件".to_string(),
                        value: format!("{}", risk_events.len()),
                        delta: rsx! { span { class: "stat-foot", style: "color:var(--bc-danger)", "↑ {high_count} high" } },
                    }
                }
            }

            // Risk trend chart placeholder
            div { class: "card", style: "padding:24px",
                div { class: "section-h",
                    span { class: "lead-title", "风险趋势" }
                }
                div { style: "height:200px; display:flex; align-items:center; justify-content:center; color:var(--bc-text-tertiary); font-size:14px",
                    "📈 风险趋势图 (待接入数据源)"
                }
            }

            // Risk events table
            div {
                div { class: "section-h",
                    span { class: "lead-title", "异常事件" }
                    div { style: "display:flex; gap:8px",
                        span { class: "chip active", "全部 {risk_events.len()}" }
                        span { class: "chip", "高危 {high_count}" }
                        span { class: "chip", "中危 {medium_count}" }
                        span { class: "chip", "低危 {low_count}" }
                    }
                }

                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                } else {
                    PageTable {
                        columns: columns,
                        for (time, channel, level, desc) in &risk_events {
                            tr {
                                key: "{time}-{channel}",
                                td { class: "mono", style: "font-size:12px; color:var(--bc-text-secondary)", "{time}" }
                                td { style: "font-weight:600", "{channel}" }
                                td { {risk_pill(level)} }
                                td { style: "font-size:13px", "{desc}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
