use burncloud_client_shared::billing_service::BillingService;
use burncloud_client_shared::components::{
    EmptyState, ErrorBanner, PageHeader, SkeletonCard, SkeletonVariant, StatKpi, StatusPill,
};
use burncloud_client_shared::i18n::{t, t_fmt};
use burncloud_client_shared::services::channel_service::{Channel, ChannelService};
use burncloud_client_shared::services::monitor_service::MonitorService;
use burncloud_client_shared::services::usage_service::UsageService;
use burncloud_client_shared::use_auth;
use dioxus::prelude::*;

fn channel_status(ch: &Channel) -> String {
    match ch.status {
        1 => "ok".to_string(),
        2 => "throttle".to_string(),
        0 => "down".to_string(),
        _ => "maintenance".to_string(),
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

fn format_thousands(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn format_usd_short(usd: f64) -> String {
    if usd >= 1.0 {
        format!("$ {usd:.2}")
    } else if usd >= 0.01 {
        format!("$ {usd:.4}")
    } else {
        format!("$ {usd:.6}")
    }
}

#[component]
pub fn Dashboard() -> Element {
    let i18n = burncloud_client_shared::i18n::use_i18n();
    let lang = i18n.language;
    let auth = use_auth();
    let user_id = auth.get_user().map(|u| u.id).unwrap_or_default();
    let token = auth.get_token().unwrap_or_default();
    let token_for_usage = token.clone();
    let token_for_billing = token.clone();

    let metrics = use_resource(move || async move { MonitorService::get_system_metrics().await });

    let channels = use_resource(move || async move { ChannelService::list(0, 50).await });

    let usage = use_resource(move || {
        let uid = user_id.clone();
        let t = token_for_usage.clone();
        async move {
            if uid.is_empty() || t.is_empty() {
                Err("Not authenticated".to_string())
            } else {
                UsageService::get_user_usage(&uid, &t).await
            }
        }
    });

    let billing = use_resource(move || {
        let t = token_for_billing.clone();
        async move {
            if t.is_empty() {
                Err("Not authenticated".to_string())
            } else {
                BillingService::get_billing_summary(&t).await
            }
        }
    });

    // Read resource states
    let m = metrics.read().clone();
    let ch_res = channels.read().clone();
    let u = usage.read().clone();
    let b = billing.read().clone();
    // Loading: any resource still pending
    let loading = m.is_none() || ch_res.is_none() || b.is_none();

    // Collect errors from all API calls
    let metrics_error = m.as_ref().and_then(|r| r.as_ref().err().cloned());
    let channels_error = ch_res.as_ref().and_then(|r| r.as_ref().err().cloned());
    let billing_error = b.as_ref().and_then(|r| r.as_ref().err().cloned());

    // Unwrap successful data
    let system_metrics = m.and_then(|r| r.ok());
    let ch_list = ch_res.and_then(|r| r.ok()).unwrap_or_default();
    let billing_summary = b.and_then(|r| r.ok());
    // Usage stats
    let total_tokens = match &u {
        Some(Ok(data)) => data.total_tokens,
        _ => 0,
    };

    // Billing-derived KPIs
    let total_requests: i64 = billing_summary
        .as_ref()
        .map(|s| s.models.iter().map(|m| m.requests).sum::<i64>() + s.pre_migration_requests)
        .unwrap_or(0);
    let total_cost_usd = billing_summary
        .as_ref()
        .map(|s| s.total_cost_usd)
        .unwrap_or(0.0);
    let model_count = billing_summary
        .as_ref()
        .map(|s| s.models.len())
        .unwrap_or(0);
    let cost_str = format_usd_short(total_cost_usd);
    let token_str = format_compact(total_tokens);
    let req_str = format_thousands(total_requests);

    // Channel health stats
    let active_channels = ch_list.iter().filter(|c| c.status == 1).count();
    let down_channels = ch_list.iter().filter(|c| c.status == 0).count();
    let total_weight: i32 = ch_list.iter().map(|c| c.weight).sum();
    let channel_delta = if down_channels > 0 {
        format!("{down_channels} down")
    } else {
        "all healthy".to_string()
    };
    let channel_count = ch_list.len();

    // System health from metrics
    let cpu_pct = system_metrics
        .as_ref()
        .map(|m| m.cpu.usage_percent)
        .unwrap_or(0.0);
    let mem_pct = system_metrics
        .as_ref()
        .map(|m| m.memory.usage_percent)
        .unwrap_or(0.0);

    rsx! {
        PageHeader {
            title: t(*lang.read(), "dashboard.title"),
            subtitle: Some(t(*lang.read(), "dashboard.subtitle_24h").to_string()),
        }

        div { class: "page-content flex flex-col gap-bc-5",
            // Error banners for all API calls
            if let Some(err) = metrics_error {
                ErrorBanner {
                    message: t_fmt(*lang.read(), "dashboard.error.metrics", &[("error", &err)]),
                    on_retry: None,
                }
            }
            if let Some(err) = billing_error {
                ErrorBanner {
                    message: t_fmt(*lang.read(), "dashboard.error.billing", &[("error", &err)]),
                    on_retry: None,
                }
            }
            if let Some(err) = channels_error {
                ErrorBanner {
                    message: t_fmt(*lang.read(), "dashboard.error.channels", &[("error", &err)]),
                    on_retry: None,
                }
            }
            // 3 KPIs — gateway overview at a glance
            div { class: "stats-grid cols-3",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "REQUESTS · ALL".to_string(),
                        value: req_str,
                        delta: rsx! { span { class: "stat-foot", "{cost_str} total" } },
                        chart: None,
                    }
                    StatKpi {
                        label: "TOKENS · ALL".to_string(),
                        value: token_str,
                        delta: rsx! { span { class: "stat-foot", "{model_count} models" } },
                        chart: None,
                    }
                    StatKpi {
                        label: "CPU / MEM".to_string(),
                        value: format!("{cpu_pct:.0}% / {mem_pct:.0}%"),
                        delta: rsx! { span { class: "stat-foot", "{active_channels}/{channel_count} channels · {channel_delta}" } },
                        chart: None,
                    }
                }
            }

            // Billing model breakdown (real data)
            if let Some(summary) = &billing_summary {
                if !summary.models.is_empty() {
                    div {
                        div { class: "section-h",
                            span { class: "lead-title", {t(*lang.read(), "dashboard.model_breakdown")} }
                            span { class: "section-sub",
                                "{summary.models.len()} models · {format_usd_short(summary.total_cost_usd)}"
                            }
                        }
                        table { class: "table",
                            thead {
                                tr {
                                    th { "MODEL" }
                                    th { class: "text-right", "REQUESTS" }
                                    th { class: "text-right", "PROMPT" }
                                    th { class: "text-right", "COMPLETION" }
                                    th { class: "text-right", "COST" }
                                }
                            }
                            tbody {
                                for m in &summary.models {
                                    tr {
                                        key: "{m.model}",
                                        td { class: "font-medium", "{m.model}" }
                                        td { class: "mono text-right", "{format_thousands(m.requests)}" }
                                        td { class: "mono text-right", "{format_compact(m.prompt_tokens)}" }
                                        td { class: "mono text-right", "{format_compact(m.completion_tokens)}" }
                                        td { class: "mono text-right font-semibold", "{format_usd_short(m.cost_usd)}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Channel health
            div {
                div { class: "section-h",
                    span { class: "lead-title", {t(*lang.read(), "dashboard.channel_health")} }
                    span { class: "section-sub", {t_fmt(*lang.read(), "dashboard.channel_health_sub", &[("active", &active_channels.to_string()), ("weight", &total_weight.to_string())])} }
                }

                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                    SkeletonCard { variant: Some(SkeletonVariant::Row) }
                } else if ch_list.is_empty() {
                    EmptyState {
                        icon: rsx! { span { class: "bc-text-3xl", "📡" } },
                        title: t(*lang.read(), "dashboard.no_channel_title").to_string(),
                        description: Some(t(*lang.read(), "dashboard.no_channel_desc").to_string()),
                        cta: None,
                    }
                } else {
                    table { class: "table",
                        thead {
                            tr {
                                th { "CHANNEL" }
                                th { class: "text-right", "WEIGHT" }
                                th { "TYPE" }
                                th { "MODELS" }
                                th { "STATUS" }
                            }
                        }
                        tbody {
                            for ch in &ch_list {
                                tr {
                                    key: "{ch.id}",
                                    td { class: "font-medium", "{ch.name}" }
                                    td { class: "mono text-right", "{ch.weight}" }
                                    td { class: "mono bc-text-sm-secondary",
                                        "{ch.type_}"
                                    }
                                    td { class: "bc-text-sm-secondary bc-ellipsis-200",
                                        "{ch.models}"
                                    }
                                    td {
                                        StatusPill {
                                            value: channel_status(ch),
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
