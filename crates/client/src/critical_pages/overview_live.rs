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

fn router_status_label(log: &RouterLog) -> &'static str {
    let is_timeout = log
        .error_type
        .as_deref()
        .is_some_and(|value| value.eq_ignore_ascii_case("timeout"));
    if is_timeout {
        "Timeout"
    } else if log.status_code >= 400 {
        "Error"
    } else if log.layer_decision.as_deref().unwrap_or("").contains("failover") {
        "Fallback"
    } else {
        "Success"
    }
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
    let user_id = current_user.as_ref().map(|user| user.id.clone()).unwrap_or_default();
    let username = current_user
        .as_ref()
        .map(|user| user.username.clone())
        .unwrap_or_else(|| "current account".to_string());

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

    let metrics: Option<SystemMetrics> = metrics_result.clone().and_then(Result::ok);
    let channels: Option<Vec<Channel>> = channels_result.clone().and_then(Result::ok);
    let logs: Option<Vec<RouterLog>> = logs_result.clone().and_then(Result::ok);
    let api_tokens: Option<Vec<TokenDto>> = tokens_result.clone().and_then(Result::ok);
    let usage: Option<UsageStats> = usage_result.clone().and_then(Result::ok);
    let billing: Option<BillingSummary> = billing_result.clone().and_then(Result::ok);

    let errors: Vec<String> = [
        metrics_result.as_ref().and_then(|value| value.as_ref().err()).map(|error| format!("Runtime: {error}")),
        channels_result.as_ref().and_then(|value| value.as_ref().err()).map(|error| format!("Providers: {error}")),
        logs_result.as_ref().and_then(|value| value.as_ref().err()).map(|error| format!("Logs: {error}")),
        tokens_result.as_ref().and_then(|value| value.as_ref().err()).map(|error| format!("API keys: {error}")),
        usage_result.as_ref().and_then(|value| value.as_ref().err()).map(|error| format!("Account usage: {error}")),
        billing_result.as_ref().and_then(|value| value.as_ref().err()).map(|error| format!("Account billing: {error}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let has_errors = !errors.is_empty();
    let errors_for_panel = errors.clone();

    let core_loading = channels_result.is_none() || logs_result.is_none() || tokens_result.is_none();
    let core_failed = channels_result.as_ref().is_some_and(Result::is_err)
        || logs_result.as_ref().is_some_and(Result::is_err)
        || tokens_result.as_ref().is_some_and(Result::is_err);

    let active_channels = channels
        .as_ref()
        .map(|items| items.iter().filter(|channel| channel.status == 1).count())
        .unwrap_or(0);
    let down_channels = channels
        .as_ref()
        .map(|items| items.iter().filter(|channel| channel.status != 1).count())
        .unwrap_or(0);
    let model_count = channels.as_ref().map(|items| channel_model_count(items)).unwrap_or(0);
    let active_keys = api_tokens
        .as_ref()
        .map(|items| items.iter().filter(|key| key.status == "active").count())
        .unwrap_or(0);
    let has_provider = active_channels > 0;
    let has_model = model_count > 0;
    let has_key = active_keys > 0;
    let has_request = logs.as_ref().is_some_and(|items| !items.is_empty());
    let has_successful_request = logs.as_ref().is_some_and(|items| {
        items.iter().any(|log| log.status_code >= 200 && log.status_code < 400)
    });
    let setup_complete = !core_loading && !core_failed && has_provider && has_model && has_key;
    let environment_verified = setup_complete && has_successful_request && !has_errors && down_channels == 0;

    let (status_class, badge_class, status_badge, status_title, status_copy) = if core_loading {
        (
            "product-status-card",
            "badge badge-neutral",
            "CHECKING",
            "Checking this environment",
            "BurnCloud is loading providers, API access, and recent request evidence before deciding whether setup or traffic needs attention.",
        )
    } else if has_errors {
        (
            "product-status-card status-blocked",
            "badge badge-error",
            "CHECK SYSTEM",
            "Some operational data is unavailable",
            "One or more live data sources failed to load. Unknown values stay unknown instead of being displayed as zero; review the errors below.",
        )
    } else if !setup_complete {
        (
            "product-status-card status-attention",
            "badge badge-warning",
            "SETUP REQUIRED",
            "Finish the traffic setup",
            "BurnCloud still needs one or more prerequisites before it can run an end-to-end request. Complete the checklist on the right.",
        )
    } else if down_channels > 0 {
        (
            "product-status-card status-attention",
            "badge badge-warning",
            "PROVIDER ISSUE",
            "A provider needs attention",
            "At least one configured provider is inactive or down. Repair it or confirm that remaining providers give the resilience you expect.",
        )
    } else if !has_successful_request {
        (
            "product-status-card status-attention",
            "badge badge-warning",
            "TEST REQUIRED",
            "Setup is complete — verify one real request",
            "Provider, model and API access are configured, but the recent router log sample contains no successful request. Run a controlled test before relying on production traffic.",
        )
    } else {
        (
            "product-status-card status-ready",
            "badge badge-success",
            "VERIFIED",
            "Traffic path verified",
            "BurnCloud has recent successful routed traffic and no provider or data-source issue is currently visible on this overview.",
        )
    };

    let latest = logs.as_ref().and_then(|items| items.first().cloned());
    let latest_for_card = latest.clone();
    let latest_for_drawer = latest.clone();
    let has_latest = latest.is_some();

    let billing_requests = billing.as_ref().map(|summary| {
        summary.models.iter().map(|model| model.requests).sum::<i64>() + summary.pre_migration_requests
    });
    let request_text = billing_requests.map(compact).unwrap_or_else(|| "—".to_string());
    let request_note = if billing_result.is_none() {
        "loading signed-in account billing".to_string()
    } else if billing_result.as_ref().is_some_and(Result::is_err) {
        "signed-in account billing unavailable".to_string()
    } else {
        format!("billing scope: {username}")
    };

    let token_text = usage.as_ref().map(|value| compact(value.total_tokens)).unwrap_or_else(|| "—".to_string());
    let usage_note = if let Some(value) = usage.as_ref() {
        format!("{} prompt • {} completion", compact(value.prompt_tokens), compact(value.completion_tokens))
    } else if usage_result.is_none() {
        "loading signed-in account usage".to_string()
    } else {
        "signed-in account usage unavailable".to_string()
    };

    let spend_text = billing.as_ref().map(|summary| format!("${:.4}", summary.total_cost_usd)).unwrap_or_else(|| "—".to_string());
    let spend_note = if billing.is_some() { format!("billing scope: {username}") } else { "signed-in account billing unavailable".to_string() };
    let billed_models_text = billing.as_ref().map(|summary| summary.models.len().to_string()).unwrap_or_else(|| "—".to_string());
    let billed_models_note = if billing.is_some() { "models with billed usage".to_string() } else { "signed-in account billing unavailable".to_string() };

    let provider_count_text = channels.as_ref().map(|_| active_channels.to_string()).unwrap_or_else(|| "—".to_string());
    let provider_note = if let Some(items) = channels.as_ref() {
        if down_channels == 0 {
            format!("{} • all active", counted(items.len(), "configured provider", "configured providers"))
        } else {
            format!("{} • {} need attention", counted(items.len(), "configured provider", "configured providers"), down_channels)
        }
    } else if channels_result.is_none() {
        "loading provider inventory".to_string()
    } else {
        "provider inventory unavailable".to_string()
    };

    let runtime_cpu = metrics.as_ref().map(|value| format!("{:.0}%", value.cpu.usage_percent)).unwrap_or_else(|| "—".to_string());
    let runtime_memory = metrics.as_ref().map(|value| format!("{:.0}%", value.memory.usage_percent)).unwrap_or_else(|| "—".to_string());
    let runtime_detail = metrics
        .as_ref()
        .map(|value| format!("{} CPU cores • {} mounted disks", value.cpu.core_count, value.disks.len()))
        .unwrap_or_else(|| {
            if metrics_result.is_none() { "Loading runtime telemetry…".to_string() } else { "Runtime telemetry unavailable".to_string() }
        });

    let mut receipt_open = use_signal(|| false);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Overview" }
                    p { class: "page-subtitle", "Separate your account usage from environment health, verify the traffic path, and act on the next real issue." }
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
                    div { class: "row gap-2", span { class: "{badge_class}", "{status_badge}" } }
                    div {
                        div { class: "product-status-title", "{status_title}" }
                        p { class: "product-status-copy", "{status_copy}" }
                    }
                    div { class: "product-actions",
                        if core_loading {
                            button { class: "button button-secondary", disabled: true, "Checking environment…" }
                        } else if core_failed {
                            button {
                                class: "button button-primary",
                                onclick: move |_| {
                                    channels_resource.restart();
                                    logs_resource.restart();
                                    tokens_resource.restart();
                                },
                                "Retry verification data"
                            }
                        } else if !has_provider {
                            Link { class: "button button-primary", to: Route::Providers {}, Icon { name: "plus" } "Add first provider" }
                        } else if !has_key {
                            Link { class: "button button-primary", to: Route::APIKeys {}, Icon { name: "key" } "Create API key" }
                        } else if !has_successful_request {
                            Link { class: "button button-primary", to: Route::Playground {}, Icon { name: "play" } "Run verification test" }
                        } else {
                            Link { class: "button button-primary", to: Route::Logs {}, Icon { name: "logs" } "Review recent traffic" }
                        }
                        if !core_loading && !core_failed && has_request {
                            Link { class: "button button-secondary", to: Route::Logs {}, "View request logs" }
                        }
                    }
                }

                div { class: "card setup-card",
                    div { class: "product-section-head",
                        div {
                            h3 { "Setup & verification" }
                            p { "Configuration is not considered verified until recent successful traffic is observed." }
                        }
                        span {
                            class: if environment_verified { "badge badge-success" } else if core_loading { "badge badge-neutral" } else if setup_complete { "badge badge-warning" } else { "badge badge-neutral" },
                            if environment_verified { "Verified" } else if core_loading { "Checking" } else if setup_complete { "Needs test" } else { "In progress" }
                        }
                    }
                    if core_loading {
                        div { class: "product-note", "Loading provider, API-key, and request evidence before showing setup conclusions…" }
                    } else if core_failed {
                        div { class: "readiness-strip blocked",
                            span { class: "readiness-dot" }
                            strong { "Setup state cannot be determined" }
                            span { class: "muted", "At least one required verification source failed to load. Retry before treating missing data as missing configuration." }
                        }
                    } else {
                        div { class: "setup-list",
                            SetupStep { complete: has_provider, title: "Provider connected", detail: counted(active_channels, "active provider", "active providers"), to: Route::Providers {}, action: "Configure" }
                            SetupStep { complete: has_model, title: "Model available", detail: counted(model_count, "model exposed", "models exposed"), to: Route::Models {}, action: "Review" }
                            SetupStep { complete: has_key, title: "API access created", detail: counted(active_keys, "active API key", "active API keys"), to: Route::APIKeys {}, action: "Create" }
                            SetupStep {
                                complete: has_successful_request,
                                title: "Recent successful request observed",
                                detail: if has_successful_request {
                                    "Recent traffic is visible in Logs".to_string()
                                } else if has_request {
                                    "Recent requests exist, but none succeeded in the loaded sample".to_string()
                                } else {
                                    "Run a request from Playground".to_string()
                                },
                                to: Route::Playground {},
                                action: "Test"
                            }
                        }
                    }
                }
            }

            if has_errors {
                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { class: "danger", "Needs attention" } p { "These live sources failed on the latest refresh; related values stay unavailable rather than falling back to zero." } }
                    }
                    for message in errors_for_panel { code { class: "terminal", "{message}" } }
                }
            }

            div { class: "product-section-head",
                div {
                    h3 { "Your usage" }
                    p { "Requests, tokens, spend, and billed models below are scoped to the signed-in account: {username}." }
                }
                Link { class: "button button-ghost button-sm", to: Route::Billing {}, "Open billing" }
            }
            div { class: "metrics",
                {kpi("Your Requests", request_text, request_note, "activity", "tone-blue")}
                {kpi("Your Tokens", token_text, usage_note, "models", "tone-purple")}
                {kpi("Your Spend", spend_text, spend_note, "dollar", "tone-amber")}
                {kpi("Your Billed Models", billed_models_text, billed_models_note, "billing", "tone-gray")}
            }

            div { class: "card card-pad stack",
                div { class: "product-section-head",
                    div { h3 { "Your spend by model" } p { "Model-level cost for the signed-in account from the same billing summary as the usage cards above." } }
                }
                if billing_result.is_none() {
                    p { class: "small muted", "Loading account billing…" }
                } else if billing_result.as_ref().is_some_and(Result::is_err) {
                    p { class: "small muted", "Account billing is unavailable. No zero-spend assumption is shown." }
                } else if let Some(summary) = billing.as_ref() {
                    if summary.models.is_empty() {
                        p { class: "small muted", "This account has no billed model usage in the returned billing summary." }
                    } else {
                        div { class: "stack",
                            for model in summary.models.iter().take(6) {
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

            div { class: "product-section-head", style: "margin-top:8px",
                div {
                    h3 { "Environment health" }
                    p { "Provider inventory, recent router logs, and runtime telemetry below describe the BurnCloud environment rather than only your billing account." }
                }
            }
            div { class: "grid-3",
                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { "Providers" } p { "Upstream supply currently visible to the router." } }
                        Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Manage" }
                    }
                    div { class: "receipt-row", label { "Active" } strong { class: "mono", "{provider_count_text}" } }
                    p { class: "tiny subtle", "{provider_note}" }
                    if channels_result.is_none() {
                        p { class: "small muted", "Loading provider inventory…" }
                    } else if channels_result.as_ref().is_some_and(Result::is_err) {
                        div { class: "readiness-strip blocked", span { class: "readiness-dot" } strong { "Provider inventory unavailable" } }
                    } else if let Some(items) = channels.as_ref() {
                        if items.is_empty() {
                            Link { class: "button button-primary button-sm", to: Route::Providers {}, "Add provider" }
                        } else {
                            div { class: "stack",
                                for channel in items.iter().take(4) {
                                    div { class: "row between",
                                        span { class: "small strong", "{channel.name}" }
                                        span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-error" }, if channel.status == 1 { "ACTIVE" } else { "DOWN" } }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { "Latest request" } p { "Latest environment-wide router log returned by the console API." } }
                        Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Open logs" }
                    }
                    if logs_result.is_none() {
                        p { class: "small muted", "Loading recent request activity…" }
                    } else if logs_result.as_ref().is_some_and(Result::is_err) {
                        div { class: "readiness-strip blocked", span { class: "readiness-dot" } strong { "Recent request activity unavailable" } }
                    } else if let Some(log) = latest_for_card {
                        {
                            let model_text = log.model.clone().unwrap_or_else(|| "-".to_string());
                            let upstream_text = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                            let status_text = format!("HTTP {} • {}", log.status_code, router_status_label(&log));
                            rsx! {
                                div { class: "receipt",
                                    div { class: "receipt-row", label { "Request" } strong { class: "mono", "{log.request_id}" } }
                                    div { class: "receipt-row", label { "Model" } strong { "{model_text}" } }
                                    div { class: "receipt-row", label { "Upstream" } strong { "{upstream_text}" } }
                                    div { class: "receipt-row", label { "Result" } strong { "{status_text}" } }
                                }
                                button { class: "button button-secondary button-sm", style: "width:100%", onclick: move |_| receipt_open.set(true), "Inspect routing metadata" }
                            }
                        }
                    } else {
                        div { class: "product-empty", style: "min-height:130px",
                            div { class: "product-empty-inner",
                                h3 { "No request activity returned" }
                                p { "Run a controlled Playground request to create verifiable routing evidence." }
                            }
                        }
                    }
                }

                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { "Runtime" } p { "Host pressure for capacity diagnosis, not a substitute for traffic health." } }
                        Link { class: "button button-ghost button-sm", to: Route::Settings {}, "Details" }
                    }
                    div { class: "grid-2",
                        div { class: "receipt-row", label { "CPU" } strong { class: "mono", "{runtime_cpu}" } }
                        div { class: "receipt-row", label { "Memory" } strong { class: "mono", "{runtime_memory}" } }
                    }
                    p { class: "tiny subtle mono", "{runtime_detail}" }
                    if metrics_result.as_ref().is_some_and(Result::is_err) {
                        div { class: "readiness-strip blocked", span { class: "readiness-dot" } strong { "Runtime telemetry unavailable" } }
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
                                    "This is routing metadata BurnCloud actually persisted for the request. It supports operational traceability; it is not presented as cryptographic attestation."
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
