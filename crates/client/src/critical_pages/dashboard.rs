use dioxus::prelude::*;

use crate::{
    backend::{
        billing_summary, system_metrics, user_usage, use_auth, BillingSummary, Channel, ChannelService,
        LogService, RouterLog, SystemMetrics, UsageStats,
    },
    components::{Drawer, Icon},
};

fn compact(n: i64) -> String {
    let abs = n.abs();
    if abs >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if abs >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if abs >= 1_000 {
        format!("{:.2}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn kpi(label: &str, value: String, note: String, icon: &'static str, tone: &'static str) -> Element {
    rsx! {
        div { class: "card metric card-hover",
            div { class: "metric-copy",
                span { class: "metric-label", "{label}" }
                span { class: "metric-value", "{value}" }
                span { class: "metric-note mono", "{note}" }
            }
            div { class: "metric-icon {tone}", Icon { name: icon } }
        }
    }
}

fn channel_model_count(channels: &[Channel]) -> usize {
    let mut models: Vec<String> = channels
        .iter()
        .flat_map(|channel| channel.models.split(',').map(str::trim).filter(|m| !m.is_empty()).map(str::to_string))
        .collect();
    models.sort();
    models.dedup();
    models.len()
}

fn route_receipt(log: &RouterLog) -> String {
    let user_id = log.user_id.clone().unwrap_or_else(|| "-".to_string());
    let model = log.model.clone().unwrap_or_else(|| "-".to_string());
    let upstream = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
    let decision = log.layer_decision.clone().unwrap_or_else(|| "-".to_string());
    let traffic = log.traffic_color.clone().unwrap_or_else(|| "-".to_string());
    let error_type = log.error_type.clone().unwrap_or_else(|| "-".to_string());
    let cost_status = log.cost_status.clone().unwrap_or_else(|| "-".to_string());
    format!(
        "request_id: {}\nuser_id: {}\npath: {}\nmodel: {}\nupstream_id: {}\nstatus_code: {}\nlatency_ms: {}\nlayer_decision: {}\ntraffic_color: {}\nerror_type: {}\ncost_status: {}\ntotal_tokens: {}\ncost_usd: {:.9}",
        log.request_id,
        user_id,
        log.path,
        model,
        upstream,
        log.status_code,
        log.latency_ms,
        decision,
        traffic,
        error_type,
        cost_status,
        log.total_tokens(),
        log.cost_usd()
    )
}

#[component]
pub fn Overview() -> Element {
    let auth = use_auth();
    let current_user = auth.user();
    let token = auth.token().unwrap_or_default();
    let user_id = current_user.as_ref().map(|u| u.id.clone()).unwrap_or_default();
    let username = current_user.as_ref().map(|u| u.username.clone()).unwrap_or_else(|| "User".to_string());

    let token_for_usage = token.clone();
    let user_for_usage = user_id.clone();
    let token_for_billing = token.clone();

    let mut metrics_resource = use_resource(move || async move { system_metrics().await });
    let mut channels_resource = use_resource(move || async move { ChannelService::list(100).await });
    let mut logs_resource = use_resource(move || async move { LogService::list(50).await });
    let mut usage_resource = use_resource(move || {
        let token = token_for_usage.clone();
        let uid = user_for_usage.clone();
        async move {
            if token.is_empty() || uid.is_empty() {
                Err("No authenticated user context".to_string())
            } else {
                user_usage(&uid, &token).await
            }
        }
    });
    let mut billing_resource = use_resource(move || {
        let token = token_for_billing.clone();
        async move {
            if token.is_empty() {
                Err("No authenticated token".to_string())
            } else {
                billing_summary(&token).await
            }
        }
    });

    let metrics_result = metrics_resource.read().clone();
    let channels_result = channels_resource.read().clone();
    let logs_result = logs_resource.read().clone();
    let usage_result = usage_resource.read().clone();
    let billing_result = billing_resource.read().clone();

    let metrics: SystemMetrics = metrics_result.clone().and_then(Result::ok).unwrap_or_default();
    let channels: Vec<Channel> = channels_result.clone().and_then(Result::ok).unwrap_or_default();
    let logs: Vec<RouterLog> = logs_result.clone().and_then(Result::ok).unwrap_or_default();
    let usage: UsageStats = usage_result.clone().and_then(Result::ok).unwrap_or_default();
    let billing: BillingSummary = billing_result.clone().and_then(Result::ok).unwrap_or_default();

    let errors: Vec<String> = [
        metrics_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Monitor: {e}")),
        channels_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Channels: {e}")),
        logs_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Logs: {e}")),
        usage_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Usage: {e}")),
        billing_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Billing: {e}")),
    ]
    .into_iter()
    .flatten()
    .collect();

    let total_requests = billing.models.iter().map(|m| m.requests).sum::<i64>() + billing.pre_migration_requests;
    let active_channels = channels.iter().filter(|channel| channel.status == 1).count();
    let down_channels = channels.iter().filter(|channel| channel.status == 0).count();
    let model_count = channel_model_count(&channels);
    let latest = logs.first().cloned();
    let request_text = compact(total_requests);
    let billing_note = format!("${:.4} total billed", billing.total_cost_usd);
    let token_text = compact(usage.total_tokens);
    let usage_note = format!("{} prompt / {} completion", compact(usage.prompt_tokens), compact(usage.completion_tokens));
    let channel_note = format!("{} total • {} down", channels.len(), down_channels);
    let system_text = format!("{:.0}% / {:.0}%", metrics.cpu.usage_percent, metrics.memory.usage_percent);
    let system_note = format!("{} cores • {} models exposed", metrics.cpu.core_count, model_count);
    let mut receipt_open = use_signal(|| false);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Good morning, {username}." }
                    p { class: "page-subtitle", "Live data from the running BurnCloud server. Dashboard values are no longer seeded UI demo values." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        onclick: move |_| {
                            metrics_resource.restart();
                            channels_resource.restart();
                            logs_resource.restart();
                            usage_resource.restart();
                            billing_resource.restart();
                        },
                        Icon { name: "activity" }
                        "Refresh All"
                    }
                    button {
                        class: "button button-primary",
                        disabled: latest.is_none(),
                        onclick: move |_| receipt_open.set(true),
                        Icon { name: "logs" }
                        "Latest Request Receipt"
                    }
                }
            }

            if !errors.is_empty() {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Some live dashboard sources could not be loaded" }
                    for message in errors { code { class: "terminal", "{message}" } }
                }
            }

            div { class: "metrics",
                {kpi("Requests", request_text, billing_note, "activity", "tone-blue")}
                {kpi("User Tokens", token_text, usage_note, "models", "tone-purple")}
                {kpi("Active Channels", active_channels.to_string(), channel_note, "server", "tone-green")}
                {kpi("CPU / Memory", system_text, system_note, "activity", "tone-amber")}
            }

            div { class: "grid-2",
                div { class: "card card-pad stack",
                    div { class: "row between",
                        span { class: "section-label", "Live Provider / Channel Pool" }
                        span { class: if down_channels == 0 { "badge badge-success" } else { "badge badge-warning" },
                            if down_channels == 0 { "ALL HEALTHY" } else { "DEGRADED" }
                        }
                    }
                    if channels.is_empty() {
                        p { class: "small muted", "No channels were returned. Admin role may be required for the channel API." }
                    } else {
                        div { class: "stack",
                            for channel in channels.iter().take(8) {
                                {
                                    let detail = format!("type={} • weight={} • models={}", channel.type_, channel.weight, channel.models);
                                    rsx! {
                                        div { class: "source-line",
                                            div { class: "source-meta",
                                                span { class: "strong", "{channel.name}" }
                                                span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" },
                                                    if channel.status == 1 { "ACTIVE" } else { "DOWN" }
                                                }
                                            }
                                            div { class: "tiny subtle mono", "{detail}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "card card-pad stack",
                    div { class: "row between",
                        span { class: "section-label", "Latest Real Router Receipt" }
                        span { class: "badge badge-neutral", "router_logs" }
                    }
                    if let Some(log) = latest.clone() {
                        {
                            let model_text = log.model.clone().unwrap_or_else(|| "-".to_string());
                            let upstream_text = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                            let status_text = format!("HTTP {} • {}", log.status_code, log.status_label());
                            rsx! {
                                div { class: "receipt",
                                    div { class: "receipt-row", label { "Request:" } strong { class: "mono", "{log.request_id}" } }
                                    div { class: "receipt-row", label { "Model:" } strong { "{model_text}" } }
                                    div { class: "receipt-row", label { "Upstream:" } strong { "{upstream_text}" } }
                                    div { class: "receipt-row", label { "Status:" } strong { "{status_text}" } }
                                }
                                button { class: "button button-primary", style: "width:100%", onclick: move |_| receipt_open.set(true), "Inspect Stored Route Metadata" }
                            }
                        }
                    } else {
                        p { class: "small muted", "No router log receipt is currently available." }
                    }
                }
            }

            div { class: "card table-card",
                div { class: "card-pad row between",
                    span { class: "section-label", "Billing Model Breakdown" }
                    span { class: "small muted", "{billing.models.len()} billed models" }
                }
                if billing.models.is_empty() {
                    div { class: "card-pad small muted", "No billing model usage returned for the current account/period." }
                } else {
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr {
                                th { "Model" }
                                th { class: "right", "Requests" }
                                th { class: "right", "Prompt" }
                                th { class: "right", "Completion" }
                                th { class: "right", "Cost" }
                            } }
                            tbody {
                                for model in billing.models.iter().take(12) {
                                    {
                                        let request_count = compact(model.requests);
                                        let prompt_count = compact(model.prompt_tokens);
                                        let completion_count = compact(model.completion_tokens);
                                        let cost_text = format!("${:.6}", model.cost_usd);
                                        rsx! {
                                            tr { key: "{model.model}",
                                                td { class: "table-primary", "{model.model}" }
                                                td { class: "right tabular", "{request_count}" }
                                                td { class: "right tabular", "{prompt_count}" }
                                                td { class: "right tabular", "{completion_count}" }
                                                td { class: "right strong tabular", "{cost_text}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            Drawer {
                title: "Stored Request Route Receipt",
                open: receipt_open(),
                on_close: move |_| receipt_open.set(false),
                if let Some(log) = latest {
                    {
                        let receipt = route_receipt(&log);
                        rsx! {
                            div { class: "stack-lg",
                                div { class: "card card-pad stack",
                                    span { class: "section-label", "What this proves" }
                                    p { class: "small muted", "These are the actual observability fields persisted by BurnCloud for the request. This UI does not claim TPM or cryptographic verification because the current router_logs schema does not store a hardware signature." }
                                }
                                pre { class: "terminal", style: "white-space:pre-wrap;line-height:1.65", "{receipt}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    rsx! { Overview {} }
}
