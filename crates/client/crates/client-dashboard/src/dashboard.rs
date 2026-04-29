use burncloud_client_shared::components::{
    PageHeader, StatKpi, Sparkline, StatusPill, ColumnDef, PageTable, EmptyState,
    SkeletonCard, SkeletonVariant, ErrorBanner,
};
use burncloud_client_shared::services::channel_service::{Channel, ChannelService};
use burncloud_client_shared::services::log_service::LogService;
use burncloud_client_shared::services::monitor_service::MonitorService;
use burncloud_client_shared::services::usage_service::UsageService;
use dioxus::prelude::*;

fn channel_status(ch: &Channel) -> String {
    if ch.status == 1 { "active".to_string() } else { "down".to_string() }
}

#[component]
pub fn Dashboard() -> Element {
    let metrics = use_resource(move || async move {
        MonitorService::get_system_metrics().await
    });

    let channels = use_resource(move || async move {
        ChannelService::list(0, 50).await
    });

    let usage = use_resource(move || async move {
        UsageService::get_user_usage("demo-user").await
    });

    let recent_logs = use_resource(move || async move {
        LogService::list(10).await
    });

    let m = metrics.read().clone();
    let ch_res = channels.read().clone();
    let u = usage.read().clone();

    let loading = m.is_none() && ch_res.is_none();
    let metrics_error = m.as_ref()
        .and_then(|r| r.clone().err().map(|e| e.to_string()))
        .or_else(|| u.as_ref().and_then(|r| r.clone().err().map(|e| e.to_string())));
    let ch_list = ch_res.and_then(|r| r.ok()).unwrap_or_default();

    // System metrics
    let (cpu_pct, mem_pct) = match &m {
        Some(Ok(data)) => (data.cpu.usage_percent as f64, data.memory.usage_percent as f64),
        _ => (0.0, 0.0),
    };

    // Usage data
    let total_tokens = match &u {
        Some(Ok(data)) => data.total_tokens,
        _ => 0,
    };

    // Channel health stats
    let active_channels = ch_list.iter().filter(|c| c.status == 1).count();
    let total_channels = ch_list.len();

    let cpu_data = vec![45.0, 52.0, 48.0, 61.0, 55.0, 43.0, 50.0];
    let req_data = vec![1200.0, 980.0, 1350.0, 1100.0, 1420.0, 1250.0, 1180.0];

    let channel_columns = vec![
        ColumnDef { key: "name".to_string(), label: "Channel".to_string(), width: None },
        ColumnDef { key: "type".to_string(), label: "Provider".to_string(), width: Some("120px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "Status".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "weight".to_string(), label: "Weight".to_string(), width: Some("80px".to_string()) },
    ];

    let log_list = recent_logs.read().clone().and_then(|r| r.ok()).unwrap_or_default();

    rsx! {
        PageHeader {
            title: "仪表盘",
            subtitle: Some("过去 24 小时 · 网关聚合视图".to_string()),
        }

        div { class: "page-content",
            if let Some(err) = metrics_error {
                ErrorBanner {
                    message: err,
                    on_retry: None,
                }
            }

            // KPI row
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "活跃渠道",
                        value: "{active_channels}",
                        chart: rsx! { Sparkline { data: req_data.clone(), tone: None } }
                    }
                    StatKpi {
                        label: "总 Token 消耗",
                        value: "{total_tokens}",
                    }
                    StatKpi {
                        label: "CPU 使用率",
                        value: "{cpu_pct:.0}%",
                        chart: rsx! { Sparkline { data: cpu_data.clone(), tone: Some("danger".to_string()) } }
                    }
                    StatKpi {
                        label: "内存使用率",
                        value: "{mem_pct:.0}%",
                    }
                }
            }

            // 2-column grid: channel health + live logs
            div { style: "display:grid; grid-template-columns:1fr 1fr; gap:20px; margin-top:24px",
                // Channel health
                div {
                    div { class: "section-h",
                        div { class: "lead",
                            span { class: "lead-title", "渠道健康" }
                            span { class: "lead-sub", "{active_channels}/{total_channels} 在线" }
                        }
                    }

                    if loading {
                        SkeletonCard { variant: Some(SkeletonVariant::Row) }
                        SkeletonCard { variant: Some(SkeletonVariant::Row) }
                        SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    } else if ch_list.is_empty() {
                        EmptyState {
                            icon: rsx! { span { style: "font-size:32px", "📡" } },
                            title: "暂无渠道数据".to_string(),
                            description: None,
                            cta: None,
                        }
                    } else {
                        PageTable {
                            columns: channel_columns,
                            for ch in &ch_list {
                                tr {
                                    key: "{ch.id}",
                                    td { style: "font-weight:500", "{ch.name}" }
                                    td { class: "mono", style: "font-size:13px", "{ch.type_}" }
                                    td {
                                        StatusPill {
                                            value: channel_status(ch)
                                        }
                                    }
                                    td { class: "mono", style: "font-size:13px", "{ch.weight}" }
                                }
                            }
                        }
                    }
                }

                // Live logs
                div {
                    div { class: "section-h",
                        div { class: "lead",
                            span { class: "lead-title", "实时日志" }
                            span { class: "lead-sub", "最近请求" }
                        }
                    }

                    if loading {
                        SkeletonCard { variant: Some(SkeletonVariant::Row) }
                        SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    } else if log_list.is_empty() {
                        EmptyState {
                            icon: rsx! { span { style: "font-size:32px", "📋" } },
                            title: "暂无日志".to_string(),
                            description: None,
                            cta: None,
                        }
                    } else {
                        div { class: "log-block",
                            for entry in &log_list {
                                div { class: if entry.status_code >= 500 { "log-err" } else if entry.status_code >= 400 { "log-warn" } else { "log-info" },
                                    span { class: "log-time", "{entry.request_id}" }
                                    " {entry.status_code} {entry.path} {entry.latency_ms}ms"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
