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
    match ch.status {
        1 => "ok".to_string(),
        2 => "throttle".to_string(),
        0 => "down".to_string(),
        _ => "maintenance".to_string(),
    }
}

fn channel_status_label(ch: &Channel) -> String {
    match ch.status {
        1 => "OK".to_string(),
        2 => "Throttled".to_string(),
        0 => "Down".to_string(),
        _ => "Maintenance".to_string(),
    }
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

    let total_tokens = match &u {
        Some(Ok(data)) => data.total_tokens,
        _ => 0,
    };

    let active_channels = ch_list.iter().filter(|c| c.status == 1).count();
    let total_weight: i32 = ch_list.iter().map(|c| c.weight).sum();

    let spark_req = vec![12.0, 18.0, 14.0, 22.0, 30.0, 28.0, 26.0, 34.0, 30.0, 42.0, 38.0, 50.0, 46.0, 58.0, 52.0, 60.0, 64.0];
    let spark_tok: Vec<f64> = spark_req.iter().rev().map(|x| x * 0.9).collect();
    let spark_lat = vec![40.0, 38.0, 34.0, 36.0, 32.0, 30.0, 32.0, 28.0, 30.0, 28.0, 26.0, 28.0, 24.0, 26.0, 22.0, 24.0, 20.0];
    let spark_err = vec![2.0, 1.0, 2.0, 3.0, 2.0, 4.0, 3.0, 5.0, 3.0, 6.0, 4.0, 5.0, 4.0, 6.0, 5.0, 7.0, 5.0];

    let channel_columns = vec![
        ColumnDef { key: "name".to_string(), label: "CHANNEL".to_string(), width: None },
        ColumnDef { key: "weight".to_string(), label: "WEIGHT".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "p50".to_string(), label: "P50".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "rpm".to_string(), label: "RPM".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "STATUS".to_string(), width: Some("120px".to_string()) },
    ];

    let log_list = recent_logs.read().clone().and_then(|r| r.ok()).unwrap_or_default();

    // Error breakdown (mock data matching design)
    let err_breakdown = vec![
        ("azure-uksouth", 1284, 62, "503 timeout"),
        ("openai-eu", 412, 20, "401 invalid key"),
        ("gemini-fallback", 268, 13, "429 throttled"),
        ("qwen-cn", 92, 5, "5xx upstream"),
    ];

    rsx! {
        PageHeader {
            title: "仪表盘",
            subtitle: Some("过去 24 小时 · 网关聚合视图".to_string()),
            actions: rsx! {
                button { class: "btn btn-secondary", "刷新" }
                button { class: "btn btn-black", "创建渠道" }
            },
        }

        div { class: "page-content", style: "display:flex; flex-direction:column; gap:24px",
            if let Some(err) = metrics_error {
                ErrorBanner {
                    message: err,
                    on_retry: None,
                }
            }

            // 4 KPIs
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "REQUESTS · 24H".to_string(),
                        value: "1,284,902".to_string(),
                        delta: rsx! { span { class: "stat-foot up", "▲ 12.4% vs yesterday" } },
                        chart: rsx! { Sparkline { data: spark_req.clone(), tone: None } }
                    }
                    StatKpi {
                        label: "TOKENS · 24H".to_string(),
                        value: format_compact(total_tokens),
                        delta: rsx! { span { class: "stat-foot up", "▲ 8.1% vs yesterday" } },
                        chart: rsx! { Sparkline { data: spark_tok.clone(), tone: None } }
                    }
                    StatKpi {
                        label: "P50 LATENCY".to_string(),
                        value: "312ms".to_string(),
                        delta: rsx! { span { class: "stat-foot up", "▲ −4.2%" } },
                        chart: rsx! { Sparkline { data: spark_lat.clone(), tone: Some("success".to_string()) } }
                    }
                    StatKpi {
                        label: "ERROR RATE".to_string(),
                        value: "0.18%".to_string(),
                        delta: rsx! { span { class: "stat-foot down", "▼ +0.04%" } },
                        chart: rsx! { Sparkline { data: spark_err.clone(), tone: Some("danger".to_string()) } }
                    }
                }
            }

            // Channel health + live logs
            div { style: "display:grid; grid-template-columns:1.45fr 1fr; gap:24px",
                // Channel health
                div {
                    div { class: "section-h",
                        span { class: "lead-title", "渠道健康" }
                        span { class: "section-sub", "{active_channels} 个活跃 · 总权重 {total_weight}" }
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
                                    td { class: "mono", style: "text-align:right; font-size:13px", "{ch.weight}" }
                                    td { class: "mono", style: "text-align:right; font-size:13px", "—" }
                                    td { class: "mono", style: "text-align:right; font-size:13px", "—" }
                                    td {
                                        StatusPill {
                                            value: channel_status(ch)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Live logs
                div {
                    div { class: "section-h",
                        span { class: "lead-title", "实时日志" }
                        span { class: "section-sub", "tail -f gateway.log" }
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
                        div { class: "log-block", style: "height:320px",
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

            // Error breakdown by upstream
            div {
                div { class: "section-h",
                    span { class: "lead-title", "当日错误分布 · by upstream" }
                    span { class: "section-sub", "2,056 errors · ↑ 12% vs yesterday" }
                }
                div { style: "display:flex; flex-direction:column; gap:8px",
                    for (upstream, count, share, kind) in &err_breakdown {
                        div { class: "row-card outlined", style: "padding:14px 16px",
                            div { style: "display:flex; align-items:center; gap:14px; flex:1; min-width:0",
                                span { style: "width:200px; font-size:13px; font-weight:500", "{upstream}" }
                                div { style: "flex:1; height:6px; background:var(--bc-bg-hover); border-radius:99px; overflow:hidden",
                                    div { style: "width:{share}%; height:100%; background:var(--bc-danger); border-radius:99px" }
                                }
                                span { class: "mono", style: "width:56px; text-align:right; font-size:12px; color:var(--bc-text-secondary)", "{share}%" }
                            }
                            div { style: "display:flex; align-items:center; gap:16px; margin-left:16px",
                                span { class: "mono", style: "font-size:12px; color:var(--bc-text-tertiary)", "{kind}" }
                                span { class: "mono", style: "font-size:14px; font-weight:600; min-width:52px; text-align:right", "{count}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
