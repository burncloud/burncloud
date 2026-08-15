use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{billing_summary, user_usage, Channel, ChannelService, LogService, RouterLog, TokenService},
    components::Icon,
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

fn display_count(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_string())
}

fn display_compact(value: Option<i64>) -> String {
    value.map(compact).unwrap_or_else(|| "—".to_string())
}

fn stored_route_line(log: &RouterLog) -> String {
    let model = log.model.as_deref().unwrap_or("Unknown model");
    let upstream = log.upstream_id.as_deref().unwrap_or("Unknown upstream");
    format!("{model}  →  {upstream}")
}

#[component]
pub fn Overview() -> Element {
    let auth = crate::backend::use_auth();
    let current_user = auth.user();
    let token = auth.token().unwrap_or_default();
    let user_id = current_user
        .as_ref()
        .map(|user| user.id.clone())
        .unwrap_or_default();

    let token_for_usage = token.clone();
    let user_for_usage = user_id.clone();
    let token_for_billing = token.clone();

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

    let channels_result = channels_resource.read().clone();
    let logs_result = logs_resource.read().clone();
    let tokens_result = tokens_resource.read().clone();
    let usage_result = usage_resource.read().clone();
    let billing_result = billing_resource.read().clone();

    let channels = channels_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let logs = logs_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let api_tokens = tokens_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let usage = usage_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let billing = billing_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());

    let errors: Vec<String> = [
        channels_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("Providers: {error}")),
        logs_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("Requests: {error}")),
        tokens_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("API keys: {error}")),
        usage_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("Usage: {error}")),
        billing_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("Billing: {error}")),
    ]
    .into_iter()
    .flatten()
    .collect();

    let loading = channels_result.is_none()
        || logs_result.is_none()
        || tokens_result.is_none()
        || usage_result.is_none()
        || billing_result.is_none();

    let active_channels = channels.map(|items| items.iter().filter(|channel| channel.status == 1).count());
    let inactive_channels = channels.map(|items| items.iter().filter(|channel| channel.status != 1).count());
    let total_channels = channels.map(Vec::len);
    let model_count = channels.map(|items| channel_model_count(items));
    let active_keys = api_tokens.map(|items| items.iter().filter(|key| key.status == "active").count());

    let recent_requests = logs.map(Vec::len);
    let recent_successes = logs.map(|items| {
        items
            .iter()
            .filter(|log| (200..300).contains(&log.status_code))
            .count()
    });
    let has_successful_request = recent_successes.map(|count| count > 0);
    let latest_request = logs.and_then(|items| items.first()).cloned();

    let billing_requests = billing.map(|summary| {
        summary.models.iter().map(|model| model.requests).sum::<i64>()
            + summary.pre_migration_requests
    });
    let billed_cost = billing.map(|summary| format!("${:.2}", summary.total_cost_usd));
    let total_tokens = usage.map(|summary| summary.total_tokens);

    let provider_value = match (active_channels, total_channels) {
        (Some(active), Some(total)) => format!("{active} / {total}"),
        _ => "—".to_string(),
    };
    let provider_note = match inactive_channels {
        Some(0) => "Configured provider status: all active".to_string(),
        Some(count) => format!("{count} configured provider(s) inactive"),
        None => "Provider status unavailable".to_string(),
    };

    let traffic_value = display_compact(billing_requests);
    let traffic_note = match (recent_successes, recent_requests) {
        (Some(success), Some(total)) if total > 0 => {
            format!("{success}/{total} recent requests returned HTTP 2xx")
        }
        (Some(_), Some(0)) => "No recent request sample".to_string(),
        _ => "Recent request sample unavailable".to_string(),
    };

    let business_value = billed_cost.unwrap_or_else(|| "—".to_string());
    let business_note = match total_tokens {
        Some(tokens) => format!("{} tokens in the current usage view", compact(tokens)),
        None => "Usage data unavailable".to_string(),
    };

    let trust_value = match logs {
        Some(items) if !items.is_empty() => "Stored routing metadata",
        Some(_) => "No request metadata yet",
        None => "Unknown",
    };
    let trust_note = if logs.is_some() {
        "Operational trace only · cryptographic/runtime attestation is not exposed here"
    } else {
        "Request evidence source unavailable"
    };

    let has_provider = active_channels.map(|count| count > 0);
    let has_model = model_count.map(|count| count > 0);
    let has_key = active_keys.map(|count| count > 0);

    let (status_class, badge_class, badge_text, status_title, status_copy) = if !errors.is_empty() {
        (
            "overview-conclusion status-blocked",
            "badge badge-error",
            "DATA UNAVAILABLE",
            "Some overview data is unavailable.",
            "BurnCloud is reachable, but this page cannot safely summarize every domain until the failed sources recover.",
        )
    } else if loading {
        (
            "overview-conclusion",
            "badge badge-neutral",
            "LOADING",
            "Reading the current BurnCloud state.",
            "The overview keeps unknown values neutral while providers, requests, access, usage, and billing are loading.",
        )
    } else if has_provider == Some(false) {
        (
            "overview-conclusion status-attention",
            "badge badge-warning",
            "ATTENTION",
            "Connect an upstream provider before sending traffic.",
            "No active provider is currently configured, so BurnCloud cannot expose a usable upstream path.",
        )
    } else if has_model == Some(false) {
        (
            "overview-conclusion status-attention",
            "badge badge-warning",
            "ATTENTION",
            "Make at least one model available.",
            "Providers are present, but no configured model is visible from the current provider data.",
        )
    } else if has_key == Some(false) {
        (
            "overview-conclusion status-attention",
            "badge badge-warning",
            "ATTENTION",
            "Create API access before testing traffic.",
            "Upstream supply is configured, but no active API key is currently available to a client.",
        )
    } else if has_successful_request == Some(true)
        && matches!(inactive_channels, Some(count) if count > 0)
    {
        (
            "overview-conclusion status-attention",
            "badge badge-warning",
            "ATTENTION",
            "Traffic has succeeded, but provider capacity needs attention.",
            "Recent HTTP 2xx traffic is visible while at least one configured provider is inactive. Review provider coverage before relying on failover.",
        )
    } else if has_successful_request == Some(true) {
        (
            "overview-conclusion status-ready",
            "badge badge-success",
            "OBSERVED",
            "BurnCloud is serving observed traffic.",
            "The current provider/access prerequisites are present and recent Logs include an HTTP 2xx request. This is an operational observation, not a cryptographic verification claim.",
        )
    } else if matches!(recent_requests, Some(count) if count > 0) {
        (
            "overview-conclusion status-attention",
            "badge badge-warning",
            "ATTENTION",
            "Requests are arriving, but no recent success is visible.",
            "The recent request sample contains activity but no HTTP 2xx response. Inspect Requests or run a controlled Playground test.",
        )
    } else {
        (
            "overview-conclusion status-attention",
            "badge badge-warning",
            "VERIFY",
            "Core access prerequisites are present; verify one request.",
            "Provider, model, and API access data are present, but this page has not observed a successful request yet.",
        )
    };

    let attention_kind = if !errors.is_empty() {
        "error"
    } else if matches!(inactive_channels, Some(count) if count > 0) {
        "provider"
    } else if has_provider == Some(false) {
        "provider-setup"
    } else if has_model == Some(false) {
        "model-setup"
    } else if has_key == Some(false) {
        "key-setup"
    } else if has_successful_request != Some(true) {
        "request"
    } else {
        "clear"
    };

    rsx! {
        div { class: "page overview-page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Overview" }
                    p { class: "page-subtitle", "See what is working, what needs attention, and what happened most recently." }
                }
                button {
                    class: "button button-secondary",
                    onclick: move |_| {
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

            div { class: "card {status_class}",
                span { class: "{badge_class}", "{badge_text}" }
                div { class: "overview-conclusion-copy",
                    h1 { "{status_title}" }
                    p { "{status_copy}" }
                }
                div { class: "product-actions",
                    if has_provider == Some(false) {
                        Link { class: "button button-primary", to: Route::Providers {}, "Open Providers" }
                    } else if has_model == Some(false) {
                        Link { class: "button button-primary", to: Route::Models {}, "Open Models" }
                    } else if has_key == Some(false) {
                        Link { class: "button button-primary", to: Route::APIKeys {}, "Open API Keys" }
                    } else {
                        Link { class: "button button-primary", to: Route::Playground {}, "Test a request" }
                    }
                    Link { class: "button button-secondary", to: Route::Logs {}, "Open Requests" }
                }
            }

            div { class: "overview-domain-grid",
                div { class: "card overview-domain",
                    span { class: "overview-domain-label", "TRUST" }
                    strong { class: "overview-domain-value overview-domain-value-text", "{trust_value}" }
                    p { "{trust_note}" }
                    Link { class: "overview-domain-link", to: Route::Logs {}, "Request evidence →" }
                }

                div { class: "card overview-domain",
                    span { class: "overview-domain-label", "TRAFFIC" }
                    strong { class: "overview-domain-value", "{traffic_value}" }
                    p { "{traffic_note}" }
                    div { class: "overview-domain-meta",
                        span { "Providers" }
                        strong { class: "mono", "{provider_value}" }
                    }
                    small { class: "subtle", "{provider_note}" }
                    Link { class: "overview-domain-link", to: Route::Routes {}, "Traffic routing →" }
                }

                div { class: "card overview-domain",
                    span { class: "overview-domain-label", "BUSINESS" }
                    strong { class: "overview-domain-value", "{business_value}" }
                    p { "Observed cost in the current billing summary" }
                    small { class: "subtle", "{business_note}" }
                    Link { class: "overview-domain-link", to: Route::Billing {}, "Billing →" }
                }
            }

            div { class: "card overview-row-card",
                div { class: "overview-row-heading",
                    span { class: "overview-domain-label", "ATTENTION" }
                    if attention_kind == "clear" {
                        span { class: "badge badge-success", "CLEAR" }
                    } else {
                        span { class: "badge badge-warning", "REVIEW" }
                    }
                }

                if attention_kind == "error" {
                    div { class: "overview-attention-copy",
                        strong { "Some data sources failed to load." }
                        for message in errors.iter() {
                            code { class: "overview-inline-error", "{message}" }
                        }
                    }
                    button {
                        class: "button button-ghost button-sm",
                        onclick: move |_| {
                            channels_resource.restart();
                            logs_resource.restart();
                            tokens_resource.restart();
                            usage_resource.restart();
                            billing_resource.restart();
                        },
                        "Retry"
                    }
                } else if attention_kind == "provider" {
                    div { class: "overview-attention-copy",
                        strong { "One or more configured providers are inactive." }
                        span { class: "muted", "{provider_note}" }
                    }
                    Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Review providers →" }
                } else if attention_kind == "provider-setup" {
                    div { class: "overview-attention-copy",
                        strong { "Provider setup is required." }
                        span { class: "muted", "No active provider is visible in the current provider data." }
                    }
                    Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Configure →" }
                } else if attention_kind == "model-setup" {
                    div { class: "overview-attention-copy",
                        strong { "Model availability is required." }
                        span { class: "muted", "No configured model is visible from the current provider mappings." }
                    }
                    Link { class: "button button-ghost button-sm", to: Route::Models {}, "Review models →" }
                } else if attention_kind == "key-setup" {
                    div { class: "overview-attention-copy",
                        strong { "API access is required." }
                        span { class: "muted", "No active API key is currently visible." }
                    }
                    Link { class: "button button-ghost button-sm", to: Route::APIKeys {}, "Create key →" }
                } else if attention_kind == "request" {
                    div { class: "overview-attention-copy",
                        strong { "A successful request has not been observed in the recent sample." }
                        span { class: "muted", "Use Playground for one controlled end-to-end request, then inspect Requests." }
                    }
                    Link { class: "button button-ghost button-sm", to: Route::Playground {}, "Test request →" }
                } else {
                    div { class: "overview-attention-copy",
                        strong { "No immediate action is indicated by the loaded overview sources." }
                        span { class: "muted", "This does not imply cryptographic verification or guarantee provider health beyond the available data." }
                    }
                    Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Inspect requests →" }
                }
            }

            div { class: "card overview-row-card overview-latest",
                div { class: "overview-row-heading",
                    span { class: "overview-domain-label", "LAST ACTIVITY" }
                }
                if let Some(log) = latest_request {
                    {
                        let route_line = stored_route_line(&log);
                        let status_line = format!("HTTP {} · {}ms · {}", log.status_code, log.latency_ms, log.status_label());
                        let request_id = log.request_id.clone();
                        let created_at = log.created_at.clone().unwrap_or_else(|| "Time unavailable".to_string());
                        rsx! {
                            div { class: "overview-request-main",
                                strong { class: "overview-request-flow", "{route_line}" }
                                span { class: "mono muted", "{status_line}" }
                            }
                            div { class: "overview-request-meta",
                                span { class: "mono", "{request_id}" }
                                span { class: "muted", "{created_at}" }
                            }
                            Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Open request →" }
                        }
                    }
                } else if logs.is_some() {
                    div { class: "overview-attention-copy",
                        strong { "No request activity is stored yet." }
                        span { class: "muted", "Run a Playground request to create the first operational trace." }
                    }
                    Link { class: "button button-ghost button-sm", to: Route::Playground {}, "Open Playground →" }
                } else {
                    div { class: "overview-attention-copy",
                        strong { "Request activity is unavailable." }
                        span { class: "muted", "The page will not substitute an empty state for an unknown request source." }
                    }
                }
            }

            div { class: "overview-footnote mono",
                "Overview summarizes operational evidence only. Detailed provider, routing, request, and billing ownership stays in their respective pages."
            }
        }
    }
}
