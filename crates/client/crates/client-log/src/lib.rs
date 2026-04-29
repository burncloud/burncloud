use burncloud_client_shared::components::{
    PageHeader, StatKpi, LevelBadge, Chip, ColumnDef, PageTable, EmptyState,
    SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::log_service::{LogEntry, LogService};
use dioxus::prelude::*;

fn status_level(code: u16) -> String {
    if code >= 500 { "ERROR".to_string() }
    else if code >= 400 { "WARN".to_string() }
    else { "INFO".to_string() }
}

fn is_error(entries: &[LogEntry]) -> i64 {
    entries.iter().filter(|e| e.status_code >= 500).count() as i64
}

fn is_warn(entries: &[LogEntry]) -> i64 {
    entries.iter().filter(|e| e.status_code >= 400 && e.status_code < 500).count() as i64
}

fn avg_latency(entries: &[LogEntry]) -> String {
    if entries.is_empty() { "—".to_string() } else {
        let avg: f64 = entries.iter().map(|e| e.latency_ms as f64).sum::<f64>() / entries.len() as f64;
        format!("{avg:.0}")
    }
}

fn format_compact(n: i64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}B", n as f64 / 1e9)
    } else if n >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1e6)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1e3)
    } else {
        n.to_string()
    }
}

fn format_thousands(n: i64) -> String {
    let s = n.abs().to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 { result.insert(0, ','); }
        result.insert(0, c);
    }
    if n < 0 { result.insert(0, '-'); }
    result
}

#[component]
pub fn LogPage() -> Element {
    let mut active_filter = use_signal(|| "all".to_string());
    let mut search_query = use_signal(String::new);

    let logs = use_resource(move || async move {
        LogService::list(200).await.unwrap_or_default()
    });

    let log_list = logs.read().clone().unwrap_or_default();
    let loading = logs.read().is_none();
    let total = log_list.len();
    let error_count = is_error(&log_list);
    let warn_count = is_warn(&log_list);

    let filtered_logs: Vec<&LogEntry> = log_list.iter().filter(|e| {
        let level = status_level(e.status_code);
        let level_match = match active_filter().as_str() {
            "error" => level == "ERROR",
            "warn" => level == "WARN",
            "info" => level == "INFO",
            "debug" => level == "DEBUG",
            _ => true,
        };
        let q = search_query().to_lowercase();
        let text_match = if q.is_empty() { true } else {
            e.path.to_lowercase().contains(&q) || e.request_id.to_lowercase().contains(&q)
        };
        level_match && text_match
    }).collect();

    let columns = vec![
        ColumnDef { key: "time".to_string(), label: "时间".to_string(), width: Some("200px".to_string()) },
        ColumnDef { key: "level".to_string(), label: "级别".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "channel".to_string(), label: "渠道".to_string(), width: Some("160px".to_string()) },
        ColumnDef { key: "message".to_string(), label: "消息".to_string(), width: None },
    ];

    rsx! {
        PageHeader {
            title: "Logs",
            subtitle: Some("全量请求与计费日志 · 实时流".to_string()),
            actions: rsx! {
                div { class: "bc-input sm", style: "width:240px; display:flex; align-items:center; gap:8px",
                    span { style: "color:var(--bc-text-tertiary); font-size:14px", "🔍" }
                    input {
                        placeholder: "搜索消息、渠道、token…",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                    }
                }
                button { class: "btn btn-secondary", "Export CSV" }
            },
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
                        label: "总条数 · 24H".to_string(),
                        value: format_compact(total as i64),
                        delta: rsx! { span { class: "stat-foot up", "↑ 12.4% vs yesterday" } },
                    }
                    StatKpi {
                        label: "错误数".to_string(),
                        value: format_thousands(error_count),
                        delta: rsx! { span { class: "stat-foot down", "↑ 18 in last hour" } },
                    }
                    StatKpi {
                        label: "告警数".to_string(),
                        value: format_thousands(warn_count),
                        delta: rsx! { span { class: "stat-foot", "3 个 channel degraded" } },
                    }
                    StatKpi {
                        label: "平均延迟".to_string(),
                        value: format!("{}ms", avg_latency(&log_list)),
                        delta: rsx! { span { class: "stat-foot up", "P50 · ↓ 4.2%" } },
                    }
                }
            }

            // Level filter chips inside section-h
            div {
                div { class: "section-h",
                    span { class: "lead-title", "请求流" }
                    div { class: "chip-row",
                        Chip {
                            label: "全部".to_string(),
                            count: Some(total as i64),
                            active: Some(active_filter() == "all"),
                            onclick: move |_| active_filter.set("all".to_string()),
                        }
                        Chip {
                            label: "INFO".to_string(),
                            count: Some((total as i64) - error_count - warn_count),
                            active: Some(active_filter() == "info"),
                            onclick: move |_| active_filter.set("info".to_string()),
                        }
                        Chip {
                            label: "WARN".to_string(),
                            count: Some(warn_count),
                            active: Some(active_filter() == "warn"),
                            onclick: move |_| active_filter.set("warn".to_string()),
                        }
                        Chip {
                            label: "ERROR".to_string(),
                            count: Some(error_count),
                            active: Some(active_filter() == "error"),
                            onclick: move |_| active_filter.set("error".to_string()),
                        }
                        Chip {
                            label: "DEBUG".to_string(),
                            count: Some(0),
                            active: Some(active_filter() == "debug"),
                            onclick: move |_| active_filter.set("debug".to_string()),
                        }
                    }
                }

                // Table or empty state
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                } else if filtered_logs.is_empty() {
                    EmptyState {
                        icon: rsx! { span { style: "font-size:40px", "🔍" } },
                        title: "没有匹配的日志".to_string(),
                        description: Some("调整搜索关键词或级别筛选".to_string()),
                        cta: None,
                    }
                } else {
                    PageTable {
                        columns: columns,
                        for entry in filtered_logs {
                            tr {
                                key: "{entry.request_id}",
                                td { class: "mono", style: "font-size:12px; color:var(--bc-text-secondary)",
                                    "{entry.request_id}"
                                }
                                td {
                                    LevelBadge {
                                        value: status_level(entry.status_code)
                                    }
                                }
                                td { class: "mono", style: "font-size:12px; color:var(--bc-text-secondary)",
                                    "{entry.path}"
                                }
                                td { class: "mono", style: "font-size:13px; color:var(--bc-text-primary)",
                                    "{entry.status_code} {entry.path} {entry.latency_ms}ms"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
