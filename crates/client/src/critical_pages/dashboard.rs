use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{
        billing_summary, system_metrics, user_usage, BillingSummary, Channel, ChannelService,
        LogService, RouterLog, SystemMetrics, TokenDto, TokenService, UsageStats,
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

fn counted(count: usize, singular: &str, plural: &str) -> String {
    format!("{} {}", count, if count == 1 { singular } else { plural })
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
        .flat_map(|channel| {
            channel
                .models
                .split(',')
                .map(str::trim)
                .filter(|model| !model.is_empty())
                .map(str::to_string)
        })
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
fn SetupStep(
    complete: bool,
    title: &'static str,
    detail: String,
    to: Route,
    action: &'static str,
) -> Element {
    rsx! {
        div { class: if complete { "setup-step complete" } else { "setup-step" },
            div { class: "setup-step-dot", if complete { "✓" } else { "·" } }
            div {
                strong { "{title}" }
                small { "{detail}" }
            }
            if complete {
                span { class: "badge badge-success", "Done" }
            } else {
                Link { class: "button button-ghost button-sm", to: to, "{action}" }
            }
        }
    }
}

#[component]
pub fn Overview() -> Element {
    let auth = crate::backend::use_auth();
    let current_user = auth.user();
    let token = auth.token().unwrap_or_default();
    let user_id = current_user.as_ref().map(|u| u.id.clone()).unwrap_or_default();

    let token_for_usage = token.clone();
    let user_for_usage = user_id.clone();
    let token_for_billing = token.clone();

    let mut metrics_resource = use_resource(move || async move { system_metrics().await });
    let mut channels_resource = use_resource(move || async move { ChannelService::list(100).await });
    let mut logs_resource = use_resource(move || async move { LogService::list(50).await });
    let mut tokens_resource = use_resource(move || async move { TokenService::list().await });
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
    let tokens_result = tokens_resource.read().clone();
    let usage_result = usage_resource.read().clone();
    let billing_result = billing_resource.read().clone();

    let metrics: SystemMetrics = metrics_result.clone().and_then(Result::ok).unwrap_or_default();
    let channels: Vec<Channel> = channels_result.clone().and_then(Result::ok).unwrap_or_default();
    let logs: Vec<RouterLog> = logs_result.clone().and_then(Result::ok).unwrap_or_default();
    let api_tokens: Vec<TokenDto> = tokens_result.clone().and_then(Result::ok).unwrap_or_default();
    let usage: UsageStats = usage_result.clone().and_then(Result::ok).unwrap_or_default();
    let billing: BillingSummary = billing_result.clone().and_then(Result::ok).unwrap_or_default();

    let errors: Vec<String> = [
        metrics_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Runtime: {e}")),
        channels_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Providers: {e}")),
        logs_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Logs: {e}")),
        tokens_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("API keys: {e}")),
        usage_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Usage: {e}")),
        billing_result.as_ref().and_then(|v| v.as_ref().err()).map(|e| format!("Billing: {e}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let has_errors = !errors.is_empty();
    let errors_for_panel = errors.clone();

    let total_requests = billing.models.iter().map(|m| m.requests).sum::<i64>() + billing.pre_migration_requests;
    let active_channels = channels.iter().filter(|channel| channel.status == 1).count();
    let down_channels = channels.iter().filter(|channel| channel.status != 1).count();
    let model_count = channel_model_count(&channels);
    let active_keys = api_tokens.iter().filter(|key| key.status == "active").count();
    let has_provider = active_channels > 0;
    let has_model = model_count > 0;
    let has_key = active_keys > 0;
    let has_request = !logs.is_empty();
    let has_successful_request = logs.iter().any(|log| log.status_code >= 200 && log.status_code < 400);
    let setup_complete = has_provider && has_model && has_key;
    let environment_verified = setup_complete && has_successful_request && !has_errors && down_channels == 0;

    let (status_class, status_badge, status_title, status_copy) = if has_errors {
        (
            "product-status-card status-blocked",
            "CHECK SYSTEM",
            "Some system data is unavailable",
            "BurnCloud is reachable, but one or more operational data sources could not be loaded. Review the errors below before relying on this environment.",
        )
    } else if !setup_complete {
        (
            "product-status-card status-attention",
            "SETUP REQUIRED",
            "Finish the traffic setup",
            "BurnCloud still needs one or more prerequisites before it can run an end-to-end request. Complete the checklist on the right.",
        )
    } else if down_channels > 0 {
        (
            "product-status-card status-attention",
            "PROVIDER ISSUE",
            "A provider needs attention",
            "At least one configured provider is inactive or down. Review provider health before relying on routing resilience.",
        )
    } else if !has_successful_request {
        (
            "product-status-card status-attention",
            "TEST REQUIRED",
            "Setup is complete — verify one real request",
            "Provider, model and API access are configured, but BurnCloud has not yet observed a successful routed request. Run a controlled test before sending production traffic.",
        )
    } else {
        (
            "product-status-card status-ready",
            "VERIFIED",
            "Traffic path verified",
            "BurnCloud has observed a successful routed request with the current environment. Continue monitoring provider health, request outcomes and spend below.",
        )
    };

    let latest = logs.first().cloned();
    let latest_for_card = latest.clone();
    let latest_for_drawer = latest.clone();
    let has_latest = latest.is_some();
    let request_text = compact(total_requests);
    let token_text = compact(usage.total_tokens);
    let spend_text = format!("${:.4}", billing.total_cost_usd);
    let provider_note = if down_channels == 0 {
        format!("{} • all healthy", counted(channels.len(), "configured provider", "configured providers"))
    } else {
        format!("{} • {} need attention", counted(channels.len(), "configured provider", "configured providers"), down_channels)
    };
    let request_note = if has_successful_request {
        "Successful traffic observed".to_string()
    } else if has_request {
        "Requests seen • none successful yet".to_string()
    } else {
        "No requests observed yet".to_string()
    };
    let usage_note = format!("{} prompt • {} completion", compact(usage.prompt_tokens), compact(usage.completion_tokens));
    let spend_note = format!("{} billed models", billing.models.len());
    let runtime_cpu = format!("{:.0}%", metrics.cpu.usage_percent);
    let runtime_memory = format!("{:.0}%", metrics.memory.usage_percent);
    let runtime_detail = format!("{} CPU cores • {} mounted disks", metrics.cpu.core_count, metrics.disks.len());
    let mut receipt_open = use_signal(|| false);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "System Overview" }
                    p { class: "page-subtitle", "See setup status, real traffic evidence, current risks, and where to act next." }
                }
                button {
                    class: "button button-secondary",
                    onclick: move |_| {
                        metrics_resource.restart();
                        channels_resource.restart();
                        logs_resource.restart();
                        tokens_resource.restart();
                        usage_resource.restart();
                        billing_resource.restart();
                    },
                    Icon { name: "activity" }
                    "Refresh"
                }
            }

            div { class: "product-hero",
                div { class: "card {status_class}",
                    div { class: "row gap-2",
                        span {
                            class: if environment_verified { "badge badge-success" } else if has_errors { "badge badge-error" } else { "badge badge-warning" },
                            "{status_badge}"
                        }
                    }
                    div {
                        div { class: "product-status-title", "{status_title}" }
                        p { class: "product-status-copy", "{status_copy}" }
                    }
                    div { class: "product-actions",
                        if !has_provider {
                            Link { class: "button button-primary", to: Route::Providers {}, Icon { name: "plus" } "Add first provider" }
                        } else if !has_key {
                            Link { class: "button button-primary", to: Route::APIKeys {}, Icon { name: "key" } "Create API key" }
                        } else if !has_successful_request {
                            Link { class: "button button-primary", to: Route::Playground {}, Icon { name: "play" } "Run verification test" }
                        } else {
                            Link { class: "button button-primary", to: Route::Logs {}, Icon { name: "logs" } "Review recent traffic" }
                        }
                        if has_request {
                            Link { class: "button button-secondary", to: Route::Logs {}, "View request logs" }
                        } else if has_provider {
                            Link { class: "button button-secondary", to: Route::Providers {}, "Review providers" }
                        }
                    }
                }

                div { class: "card setup-card",
                    div { class: "product-section-head",
                        div {
                            h3 { "Setup & verification" }
                            p { "Four checks from configuration to proven traffic." }
                        }
                        span {
                            class: if environment_verified { "badge badge-success" } else if setup_complete { "badge badge-warning" } else { "badge badge-neutral" },
                            if environment_verified { "Verified" } else if setup_complete { "Needs test" } else { "In progress" }
                        }
                    }
                    div { class: "setup-list",
                        SetupStep { complete: has_provider, title: "Provider connected", detail: counted(active_channels, "active provider", "active providers"), to: Route::Providers {}, action: "Configure" }
                        SetupStep { complete: has_model, title: "Model available", detail: counted(model_count, "model exposed", "models exposed"), to: Route::Models {}, action: "Review" }
                        SetupStep { complete: has_key, title: "API access created", detail: counted(active_keys, "active API key", "active API keys"), to: Route::APIKeys {}, action: "Create" }
                        SetupStep {
                            complete: has_successful_request,
                            title: "Successful request observed",
                            detail: if has_successful_request {
                                "Traffic is visible in Logs".to_string()
                            } else if has_request {
                                "Requests exist, but none has verified the path yet".to_string()
                            } else {
                                "Run a request from Playground".to_string()
                            },
                            to: Route::Playground {},
                            action: "Test"
                        }
                    }
                }
            }

            if has_errors {
                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { class: "danger", "Needs attention" } p { "These sources failed to load on the latest refresh." } }
                    }
                    for message in errors_for_panel { code { class: "terminal", "{message}" } }
                }
            }

            div { class: "metrics",
                {kpi("Requests", request_text, request_note, "activity", "tone-blue")}
                {kpi("Tokens", token_text, usage_note, "models", "tone-purple")}
                {kpi("Spend", spend_text, spend_note, "dollar", "tone-amber")}
                {kpi("Active Providers", active_channels.to_string(), provider_note, "server", "tone-green")}
            }

            div { class: "grid-2",
                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div {
                            h3 { "Provider health" }
                            p { "The upstream supply currently available to the router." }
                        }
                        Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Manage providers" }
                    }
                    if channels.is_empty() {
                        div { class: "product-empty", style: "min-height:150px",
                            div { class: "product-empty-inner",
                                div { class: "product-empty-icon", Icon { name: "providers" } }
                                h3 { "No providers configured" }
                                p { "Add an upstream provider before BurnCloud can expose models or route requests." }
                                Link { class: "button button-primary button-sm", to: Route::Providers {}, "Add provider" }
                            }
                        }
                    } else {
                        div { class: "stack",
                            for channel in channels.iter().take(6) {
                                {
                                    let mut model_summary = channel.models.chars().take(52).collect::<String>();
                                    if channel.models.chars().count() > 52 {
                                        model_summary.push('…');
                                    }
                                    rsx! {
                                        div { class: "source-line",
                                            div { class: "source-meta",
                                                span { class: "strong", "{channel.name}" }
                                                span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" },
                                                    if channel.status == 1 { "ACTIVE" } else { "DOWN" }
                                                }
                                            }
                                            div { class: "tiny subtle mono", "{model_summary}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div {
                            h3 { "Latest request" }
                            p { "Use the latest real request as a quick routing confidence check." }
                        }
                        Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Open logs" }
                    }
                    if let Some(log) = latest_for_card {
                        {
                            let model_text = log.model.clone().unwrap_or_else(|| "-".to_string());
                            let upstream_text = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                            let status_text = format!("HTTP {} • {}", log.status_code, log.status_label());
                            rsx! {
                                div { class: "receipt",
                                    div { class: "receipt-row", label { "Request" } strong { class: "mono", "{log.request_id}" } }
                                    div { class: "receipt-row", label { "Model" } strong { "{model_text}" } }
                                    div { class: "receipt-row", label { "Upstream" } strong { "{upstream_text}" } }
                                    div { class: "receipt-row", label { "Result" } strong { "{status_text}" } }
                                }
                                button { class: "button button-secondary", style: "width:100%", onclick: move |_| receipt_open.set(true), "Inspect routing metadata" }
                            }
                        }
                    } else {
                        div { class: "product-empty", style: "min-height:150px",
                            div { class: "product-empty-inner",
                                div { class: "product-empty-icon", Icon { name: "logs" } }
                                h3 { "No request activity yet" }
                                p { "Run one controlled request in Playground to verify provider, model, API access and routing together." }
                                Link { class: "button button-primary button-sm", to: Route::Playground {}, "Run verification test" }
                            }
                        }
                    }
                }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { "Runtime" } p { "Host resource pressure is secondary to traffic health, but useful for capacity diagnosis." } }
                        Link { class: "button button-ghost button-sm", to: Route::Settings {}, "System details" }
                    }
                    div { class: "grid-2",
                        div { class: "receipt-row", label { "CPU" } strong { class: "mono", "{runtime_cpu}" } }
                        div { class: "receipt-row", label { "Memory" } strong { class: "mono", "{runtime_memory}" } }
                    }
                    span { class: "tiny subtle mono", "{runtime_detail}" }
                }

                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { "Spend by model" } p { "Top billed models for the current billing period." } }
                        Link { class: "button button-ghost button-sm", to: Route::Billing {}, "Open billing" }
                    }
                    if billing.models.is_empty() {
                        p { class: "small muted", "No billed usage is available yet." }
                    } else {
                        div { class: "stack",
                            for model in billing.models.iter().take(5) {
                                {
                                    let cost = format!("${:.6}", model.cost_usd);
                                    rsx! {
                                        div { class: "row between",
                                            span { class: "mono small", "{model.model}" }
                                            strong { class: "mono small", "{cost}" }
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
                open: receipt_open() && has_latest,
                on_close: move |_| receipt_open.set(false),
                if let Some(log) = latest_for_drawer {
                    {
                        let receipt = route_receipt(&log);
                        rsx! {
                            div { class: "stack-lg",
                                div { class: "product-note",
                                    "This is the routing metadata BurnCloud actually persisted for the request. It is useful for operational traceability; it is not presented as a cryptographic attestation."
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
