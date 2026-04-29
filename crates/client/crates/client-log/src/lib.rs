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
        format!("{avg:.0}ms")
    }
}

#[component]
pub fn LogPage() -> Element {
    let mut active_filter = use_signal(|| "all".to_string());

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
        match active_filter().as_str() {
            "error" => level == "ERROR",
            "warn" => level == "WARN",
            "info" => level == "INFO",
            _ => true,
        }
    }).collect();

    let columns = vec![
        ColumnDef { key: "level".to_string(), label: "Level".to_string(), width: Some("90px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "Status".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "path".to_string(), label: "Path".to_string(), width: None },
        ColumnDef { key: "latency".to_string(), label: "Latency".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "request_id".to_string(), label: "Request ID".to_string(), width: Some("140px".to_string()) },
    ];

    rsx! {
        PageHeader {
            title: "日志审查",
            subtitle: Some("系统运行日志与异常追踪".to_string()),
        }

        div { class: "page-content",
            // Stats
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "总请求量",
                        value: "{total}",
                    }
                    StatKpi {
                        label: "5xx 错误",
                        value: "{error_count}",
                    }
                    StatKpi {
                        label: "4xx 警告",
                        value: "{warn_count}",
                    }
                    StatKpi {
                        label: "平均延迟",
                        value: "{avg_latency(&log_list)}",
                    }
                }
            }

            // Level filter chips
            div { class: "chip-row", style: "margin: 20px 0;",
                Chip {
                    label: "ALL".to_string(),
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
            }

            // Table or empty state
            if loading {
                SkeletonCard { variant: Some(SkeletonVariant::Row) }
            } else if filtered_logs.is_empty() {
                EmptyState {
                    icon: rsx! { span { style: "font-size:40px", "🔍" } },
                    title: "暂无日志记录".to_string(),
                    description: Some("当前筛选条件下没有匹配的日志".to_string()),
                    cta: None,
                }
            } else {
                PageTable {
                    columns: columns,
                    for entry in filtered_logs {
                        tr {
                            key: "{entry.request_id}",
                            td {
                                LevelBadge {
                                    value: status_level(entry.status_code)
                                }
                            }
                            td { class: "mono", style: "font-size:13px",
                                "{entry.status_code}"
                            }
                            td { class: "mono", style: "font-size:13px",
                                "{entry.path}"
                            }
                            td { class: "mono", style: "font-size:13px",
                                "{entry.latency_ms}ms"
                            }
                            td { class: "mono", style: "font-size:12px; color:var(--bc-text-secondary)",
                                "{entry.request_id}"
                            }
                        }
                    }
                }
            }
        }
    }
}
