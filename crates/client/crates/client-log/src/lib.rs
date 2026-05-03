use burncloud_client_shared::components::{
    PageHeader, LevelBadge, Chip, EmptyState,
    SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::log_service::{LogEntry, LogService};
use burncloud_client_shared::use_toast;
use dioxus::document::eval;
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

fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}

fn generate_csv(entries: &[LogEntry]) -> String {
    let mut rows = vec![
        "时间,级别,渠道,方法,路径,状态码,延迟(ms),模型,Token数".to_string()
    ];
    for e in entries {
        let ts = csv_escape(e.created_at.as_deref().unwrap_or(&e.request_id));
        let level = csv_escape(&status_level(e.status_code));
        let upstream = csv_escape(e.upstream_id.as_deref().unwrap_or("—"));
        let method = csv_escape(e.method.as_deref().unwrap_or("POST"));
        let path = csv_escape(&e.path);
        let model = csv_escape(e.model.as_deref().unwrap_or("—"));
        let tokens = e.total_tokens.map_or(String::new(), |t| t.to_string());
        rows.push(format!("{ts},{level},{upstream},{method},{path},{},{},{model},{tokens}", e.status_code, e.latency_ms));
    }
    rows.join("\n")
}

fn trigger_download(csv: &str, filename: &str) {
    let escaped = csv.replace('\\', "\\\\").replace('`', "\\`").replace('$', "\\$");
    let js = format!(
        r#"(function() {{
            var blob = new Blob([`{escaped}`], {{ type: 'text/csv;charset=utf-8;' }});
            var url = URL.createObjectURL(blob);
            var a = document.createElement('a');
            a.href = url;
            a.download = '{filename}';
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
        }})()"#
    );
    eval(&js);
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

    let filtered_logs: Vec<LogEntry> = log_list.iter().filter(|e| {
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
            e.path.to_lowercase().contains(&q)
                || e.upstream_id.as_deref().unwrap_or("").to_lowercase().contains(&q)
                || e.model.as_deref().unwrap_or("").to_lowercase().contains(&q)
        };
        level_match && text_match
    }).cloned().collect();

    let toast = use_toast();

    rsx! {
        PageHeader {
            title: "Logs",
            subtitle: Some("全量请求与计费日志 · 实时流".to_string()),
            actions: rsx! {
                div { class: "input sm log-search-input",
                    input {
                        placeholder: "搜索消息、渠道、token…",
                        value: "{search_query}",
                        oninput: move |e| search_query.set(e.value()),
                    }
                }
                {
                    let filtered_snapshot = filtered_logs.clone();
                    rsx! {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| {
                                if filtered_snapshot.is_empty() {
                                    toast.error("无数据可导出");
                                    return;
                                }
                                let csv = generate_csv(&filtered_snapshot);
                                trigger_download(&csv, "logs_export.csv");
                            },
                            "导出 CSV"
                        }
                    }
                }
            },
        }

        div { class: "page-content flex flex-col gap-xxxl",
            // KPI strip
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "总条数 · 24H" }
                        div { class: "stat-value",
                            "2.84"
                            span { class: "stat-pill muted", "M" }
                        }
                        span { class: "stat-foot up", "↑ 12.4% vs yesterday" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "错误数" }
                        div { class: "stat-value text-danger",
                            "{format_thousands(error_count)}"
                        }
                        span { class: "stat-foot down", "↑ 18 in last hour" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "告警数" }
                        div { class: "stat-value text-warning",
                            "{format_thousands(warn_count)}"
                        }
                        span { class: "stat-foot", "3 个 channel degraded" }
                    }
                    div { class: "stat-card",
                        span { class: "stat-eyebrow", "平均延迟" }
                        div { class: "stat-value",
                            "{avg_latency(&log_list)}"
                            span { class: "stat-pill muted", "ms" }
                        }
                        span { class: "stat-foot up", "P50 · ↓ 4.2%" }
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
                        icon: rsx! { span { class: "text-xxxl", "🔍" } },
                        title: "没有匹配的日志".to_string(),
                        description: Some("调整搜索关键词或级别筛选".to_string()),
                        cta: None,
                    }
                } else {
                    table { class: "table",
                        thead {
                            tr {
                                th { class: "log-col-time", "时间" }
                                th { class: "log-col-level", "级别" }
                                th { class: "log-col-channel", "渠道" }
                                th { "消息" }
                            }
                        }
                        tbody {
                            for entry in filtered_logs {
                                {
                                    let ts = entry.created_at.as_deref().unwrap_or(&entry.request_id);
                                    let upstream = entry.upstream_id.as_deref().unwrap_or("—");
                                    let http_method = entry.method.as_deref().unwrap_or("POST");
                                    let tok_str = entry.total_tokens.map_or(String::new(), |t| format!(" · {} tok", format_thousands(t)));
                                    let msg = format!("{} {} ({}ms{})", http_method, entry.path, entry.latency_ms, tok_str);
                                    let req_id = entry.request_id.clone();
                                    rsx! {
                                        tr {
                                            key: "{req_id}",
                                            td { class: "mono log-cell-secondary",
                                                "{ts}"
                                            }
                                            td {
                                                LevelBadge {
                                                    value: status_level(entry.status_code)
                                                }
                                            }
                                            td { class: "mono log-cell-secondary",
                                                "{upstream}"
                                            }
                                            td { class: "mono log-cell-primary",
                                                "{msg}"
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
}
