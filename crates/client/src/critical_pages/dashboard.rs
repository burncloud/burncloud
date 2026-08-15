use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{
        billing_summary, system_metrics, user_usage, Channel, ChannelService, LogService,
        RouterLog, TokenService,
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

fn kpi(
    label: &str,
    value: String,
    note: String,
    icon: &'static str,
    tone: &'static str,
) -> Element {
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

fn enabled_channel_model_count(channels: &[Channel]) -> usize {
    let mut models: Vec<String> = channels
        .iter()
        .filter(|channel| channel.status == 1)
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
    let decision = log
        .layer_decision
        .clone()
        .unwrap_or_else(|| "-".to_string());
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
            div { class: "setup-step-dot", if complete { "OK" } else { "-" } }
            div {
                strong { "{title}" }
                small { "{detail}" }
            }
            if complete {
                span { class: "badge badge-success", "Present" }
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
    let user_id = current_user
        .as_ref()
        .map(|u| u.id.clone())
        .unwrap_or_default();

    let token_for_usage = token.clone();
    let user_for_usage = user_id.clone();
    let token_for_billing = token.clone();

    let mut metrics_resource = use_resource(move || async move { system_metrics().await });
    let mut channels_resource =
        use_resource(move || async move { ChannelService::list(100).await });
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

    let channels = channels_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let logs = logs_result.as_ref().and_then(|result| result.as_ref().ok());
    let api_tokens = tokens_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let billing = billing_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let metrics = metrics_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());

    let primary_loading =
        channels_result.is_none() || logs_result.is_none() || tokens_result.is_none();
    let primary_errors: Vec<String> = [
        channels_result
            .as_ref()
            .and_then(|v| v.as_ref().err())
            .map(|e| format!("Provider configuration: {e}")),
        logs_result
            .as_ref()
            .and_then(|v| v.as_ref().err())
            .map(|e| format!("Request observations: {e}")),
        tokens_result
            .as_ref()
            .and_then(|v| v.as_ref().err())
            .map(|e| format!("API access: {e}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let supporting_errors: Vec<String> = [
        metrics_result
            .as_ref()
            .and_then(|v| v.as_ref().err())
            .map(|e| format!("Runtime metrics: {e}")),
        usage_result
            .as_ref()
            .and_then(|v| v.as_ref().err())
            .map(|e| format!("Token usage: {e}")),
        billing_result
            .as_ref()
            .and_then(|v| v.as_ref().err())
            .map(|e| format!("Billing summary: {e}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let primary_unknown = !primary_errors.is_empty();

    let active_channels =
        channels.map(|items| items.iter().filter(|channel| channel.status == 1).count());
    let disabled_channels =
        channels.map(|items| items.iter().filter(|channel| channel.status != 1).count());
    let model_count = channels.map(|items| enabled_channel_model_count(items));
    let active_keys =
        api_tokens.map(|items| items.iter().filter(|key| key.status == "active").count());
    let has_provider = active_channels.is_some_and(|count| count > 0);
    let has_model = model_count.is_some_and(|count| count > 0);
    let has_key = active_keys.is_some_and(|count| count > 0);
    let has_request = logs.is_some_and(|items| !items.is_empty());
    let has_successful_request = logs.is_some_and(|items| {
        items
            .iter()
            .any(|log| (200..300).contains(&log.status_code))
    });
    let routing_configured = has_provider && has_model && has_key;
    let setup_complete =
        !primary_loading && !primary_unknown && routing_configured && has_successful_request;

    let (status_class, status_badge_class, status_badge, status_title, status_copy) =
        if primary_loading {
            (
                "product-status-card status-loading",
                "badge badge-neutral",
                "LOADING",
                "Loading current system state",
                "BurnCloud is reading routing prerequisites and the latest persisted request observations.",
            )
        } else if primary_unknown {
            (
                "product-status-card status-blocked",
                "badge badge-error",
                "UNKNOWN",
                "Current system state is incomplete",
                "One or more primary sources could not be loaded. BurnCloud cannot conclude whether routing is configured from the available evidence.",
            )
        } else if !routing_configured {
            (
                "product-status-card status-attention",
                "badge badge-warning",
                "SETUP NEEDED",
                "Routing setup is incomplete",
                "At least one enabled provider, an exposed model, or an active API key is missing. Complete the readiness checklist before sending traffic.",
            )
        } else if !has_successful_request {
            (
                "product-status-card status-attention",
                "badge badge-warning",
                "CONFIGURED",
                "Routing is configured; observation pending",
                if has_request {
                    "The latest 50 persisted requests contain no HTTP 2xx result. Review Logs or run a controlled request before relying on the route."
                } else {
                    "The routing prerequisites are configured, but no persisted request has been observed. Run a controlled request in Playground."
                },
            )
        } else {
            (
                "product-status-card status-ready",
                "badge badge-success",
                "OBSERVED",
                "Routing is configured and a recent success was observed",
                "Providers, models, and API access are configured. At least one HTTP 2xx result is present in the latest 50 persisted request records.",
            )
        };

    let (request_text, request_note) = match &billing_result {
        None => (
            "Loading".to_string(),
            "Fetching billing summary".to_string(),
        ),
        Some(Err(_)) => (
            "UNKNOWN".to_string(),
            "Billing summary unavailable".to_string(),
        ),
        Some(Ok(summary)) => {
            let total = summary
                .models
                .iter()
                .map(|model| model.requests)
                .sum::<i64>()
                + summary.pre_migration_requests;
            (compact(total), "Current billing summary".to_string())
        }
    };
    let (token_text, usage_note) = match &usage_result {
        None => ("Loading".to_string(), "Fetching user usage".to_string()),
        Some(Err(_)) => ("UNKNOWN".to_string(), "Token usage unavailable".to_string()),
        Some(Ok(stats)) => (
            compact(stats.total_tokens),
            format!(
                "{} prompt / {} completion",
                compact(stats.prompt_tokens),
                compact(stats.completion_tokens)
            ),
        ),
    };
    let (spend_text, spend_note) = match &billing_result {
        None => (
            "Loading".to_string(),
            "Fetching billing summary".to_string(),
        ),
        Some(Err(_)) => (
            "UNKNOWN".to_string(),
            "Billing summary unavailable".to_string(),
        ),
        Some(Ok(summary)) => (
            format!("${:.2}", summary.total_cost_usd),
            format!("{} billed models", summary.models.len()),
        ),
    };
    let (provider_text, provider_note) = match &channels_result {
        None => ("Loading".to_string(), "Fetching configuration".to_string()),
        Some(Err(_)) => (
            "UNKNOWN".to_string(),
            "Provider configuration unavailable".to_string(),
        ),
        Some(Ok(items)) => (
            active_channels.map_or_else(|| "UNKNOWN".to_string(), |count| count.to_string()),
            format!(
                "{} configured / {} disabled",
                items.len(),
                disabled_channels.map_or_else(|| "UNKNOWN".to_string(), |count| count.to_string())
            ),
        ),
    };

    let latest = logs.and_then(|items| items.first()).cloned();
    let latest_for_card = latest.clone();
    let latest_for_drawer = latest.clone();
    let has_latest = latest.is_some();
    let mut receipt_open = use_signal(|| false);

    rsx! {
        div { class: "page overview-page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Overview" }
                    p { class: "page-subtitle", "Current routing readiness, observed traffic, and supporting operational state." }
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
                    div { span { class: "{status_badge_class}", "{status_badge}" } }
                    div {
                        div { class: "product-status-title", "{status_title}" }
                        p { class: "product-status-copy", "{status_copy}" }
                    }
                    div { class: "product-actions",
                        if !primary_loading && !primary_unknown && !has_provider {
                            Link { class: "button button-primary", to: Route::Providers {}, Icon { name: "plus" } "Add provider" }
                        } else if !primary_loading && !primary_unknown && !has_key {
                            Link { class: "button button-primary", to: Route::APIKeys {}, Icon { name: "key" } "Create API key" }
                        } else if !primary_loading && !primary_unknown {
                            Link { class: "button button-primary", to: Route::Playground {}, Icon { name: "play" } "Test a request" }
                        }
                        Link { class: "button button-secondary", to: Route::Logs {}, "View request logs" }
                    }
                }

                div { class: "card setup-card",
                    div { class: "product-section-head",
                        div { h3 { "Routing readiness" } p { "Required configuration plus a scoped traffic observation." } }
                        span { class: if setup_complete { "badge badge-success" } else { "badge badge-neutral" },
                            if primary_loading { "Loading" } else if primary_unknown { "Unknown" } else if setup_complete { "Observed" } else { "Incomplete" }
                        }
                    }
                    if primary_loading {
                        div { class: "overview-state-placeholder", "Loading routing prerequisites..." }
                    } else if primary_unknown {
                        div { class: "overview-state-placeholder", "Readiness is unknown until the primary data sources are available." }
                    } else {
                        div { class: "setup-list",
                            SetupStep { complete: has_provider, title: "Enabled provider", detail: format!("{} enabled providers", active_channels.map_or_else(|| "UNKNOWN".to_string(), |count| count.to_string())), to: Route::Providers {}, action: "Configure" }
                            SetupStep { complete: has_model, title: "Exposed model", detail: format!("{} models on enabled providers", model_count.map_or_else(|| "UNKNOWN".to_string(), |count| count.to_string())), to: Route::Models {}, action: "Review" }
                            SetupStep { complete: has_key, title: "Active API access", detail: format!("{} active API keys", active_keys.map_or_else(|| "UNKNOWN".to_string(), |count| count.to_string())), to: Route::APIKeys {}, action: "Create" }
                            SetupStep {
                                complete: has_successful_request,
                                title: "Successful request observed",
                                detail: if has_successful_request {
                                    "HTTP 2xx present in the latest 50 persisted logs".to_string()
                                } else if has_request {
                                    "No HTTP 2xx present in the latest 50 persisted logs".to_string()
                                } else {
                                    "No persisted requests in the loaded sample".to_string()
                                },
                                to: Route::Playground {},
                                action: "Test"
                            }
                        }
                    }
                }
            }

            if !primary_errors.is_empty() || !supporting_errors.is_empty() {
                div { class: "card card-pad stack overview-attention",
                    div { class: "product-section-head",
                        div {
                            h3 { class: "danger", "Data unavailable" }
                            p { "Failed sources remain unknown; they are not represented as zero or empty." }
                        }
                    }
                    for message in primary_errors.iter().chain(supporting_errors.iter()) {
                        code { class: "overview-error-line", "{message}" }
                    }
                }
            }

            div { class: "metrics overview-metrics",
                {kpi("Requests", request_text, request_note, "activity", "tone-blue")}
                {kpi("Tokens", token_text, usage_note, "models", "tone-purple")}
                {kpi("Spend", spend_text, spend_note, "dollar", "tone-amber")}
                {kpi("Enabled Providers", provider_text, provider_note, "server", "tone-green")}
            }

            div { class: "grid-2",
                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { "Provider configuration" } p { "Enabled state from the configured upstream channel records." } }
                        Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Manage providers" }
                    }
                    match &channels_result {
                        None => rsx! { div { class: "overview-state-placeholder", "Loading provider configuration..." } },
                        Some(Err(_)) => rsx! { div { class: "overview-state-placeholder", strong { "UNKNOWN" } span { " Provider configuration could not be loaded." } } },
                        Some(Ok(items)) if items.is_empty() => rsx! {
                            div { class: "product-empty overview-compact-empty",
                                div { class: "product-empty-inner",
                                    div { class: "product-empty-icon", Icon { name: "providers" } }
                                    h3 { "No providers configured" }
                                    p { "Add an upstream provider before BurnCloud can expose models or route requests." }
                                    Link { class: "button button-primary button-sm", to: Route::Providers {}, "Add provider" }
                                }
                            }
                        },
                        Some(Ok(items)) => rsx! {
                            div { class: "stack",
                                for channel in items.iter().take(6) {
                                    {
                                        let mut model_summary = channel.models.chars().take(52).collect::<String>();
                                        if channel.models.chars().count() > 52 { model_summary.push_str("..."); }
                                        if model_summary.is_empty() { model_summary.push_str("No models configured"); }
                                        rsx! {
                                            div { class: "source-line overview-provider-row",
                                                div { class: "source-meta",
                                                    span { class: "strong", "{channel.name}" }
                                                    span { class: if channel.status == 1 { "badge badge-success" } else { "badge badge-neutral" },
                                                        if channel.status == 1 { "ENABLED" } else { "DISABLED" }
                                                    }
                                                }
                                                div { class: "tiny subtle mono", "{model_summary}" }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }

                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div { h3 { "Latest persisted request" } p { "The first record returned by the latest 50-request log query." } }
                        Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Open logs" }
                    }
                    match (&logs_result, latest_for_card) {
                        (None, _) => rsx! { div { class: "overview-state-placeholder", "Loading request observations..." } },
                        (Some(Err(_)), _) => rsx! { div { class: "overview-state-placeholder", strong { "UNKNOWN" } span { " Request observations could not be loaded." } } },
                        (Some(Ok(_)), Some(log)) => {
                            let model_text = log.model.clone().unwrap_or_else(|| "-".to_string());
                            let upstream_text = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                            let status_text = format!("HTTP {} / {}", log.status_code, log.status_label());
                            rsx! {
                                div { class: "receipt",
                                    div { class: "receipt-row", label { "Request" } strong { class: "mono", "{log.request_id}" } }
                                    div { class: "receipt-row", label { "Model" } strong { "{model_text}" } }
                                    div { class: "receipt-row", label { "Selected upstream" } strong { "{upstream_text}" } }
                                    div { class: "receipt-row", label { "Result" } strong { "{status_text}" } }
                                }
                                button { class: "button button-secondary", style: "width:100%", onclick: move |_| receipt_open.set(true), "Inspect stored metadata" }
                            }
                        },
                        (Some(Ok(_)), None) => rsx! {
                            div { class: "product-empty overview-compact-empty",
                                div { class: "product-empty-inner",
                                    div { class: "product-empty-icon", Icon { name: "logs" } }
                                    h3 { "No request activity observed" }
                                    p { "The loaded 50-request sample is empty. Use Playground to make a controlled request." }
                                    Link { class: "button button-primary button-sm", to: Route::Playground {}, "Open Playground" }
                                }
                            }
                        },
                    }
                }
            }

            div { class: "overview-supporting-section",
                div { class: "overview-supporting-head",
                    h3 { "Supporting state" }
                    p { "Capacity and billing context; these sources do not determine routing readiness." }
                }
                div { class: "grid-2",
                    div { class: "card card-pad stack",
                        div { class: "product-section-head",
                            div { h3 { "Host resources" } p { "Latest monitor API sample." } }
                            Link { class: "button button-ghost button-sm", to: Route::Settings {}, "System details" }
                        }
                        match metrics {
                            None if metrics_result.is_none() => rsx! { div { class: "overview-state-placeholder", "Loading runtime metrics..." } },
                            None => rsx! { div { class: "overview-state-placeholder", strong { "UNKNOWN" } span { " Runtime metrics could not be loaded." } } },
                            Some(data) => {
                                let runtime_cpu = format!("{:.0}%", data.cpu.usage_percent);
                                let runtime_memory = format!("{:.0}%", data.memory.usage_percent);
                                let runtime_detail = format!("{} CPU cores / {} mounted disks", data.cpu.core_count, data.disks.len());
                                rsx! {
                                    div { class: "grid-2 overview-runtime-grid",
                                        div { class: "receipt-row", label { "CPU" } strong { class: "mono", "{runtime_cpu}" } }
                                        div { class: "receipt-row", label { "Memory" } strong { class: "mono", "{runtime_memory}" } }
                                    }
                                    span { class: "tiny subtle mono", "{runtime_detail}" }
                                }
                            },
                        }
                    }

                    div { class: "card card-pad stack",
                        div { class: "product-section-head",
                            div { h3 { "Spend by model" } p { "Current billing summary." } }
                            Link { class: "button button-ghost button-sm", to: Route::Billing {}, "Open billing" }
                        }
                        match billing {
                            None if billing_result.is_none() => rsx! { div { class: "overview-state-placeholder", "Loading billing summary..." } },
                            None => rsx! { div { class: "overview-state-placeholder", strong { "UNKNOWN" } span { " Billing summary could not be loaded." } } },
                            Some(summary) if summary.models.is_empty() => rsx! { p { class: "small muted", "No billed model usage is present in the current summary." } },
                            Some(summary) => rsx! {
                                div { class: "stack",
                                    for model in summary.models.iter().take(5) {
                                        {
                                            let cost = format!("${:.6}", model.cost_usd);
                                            rsx! { div { class: "row between", span { class: "mono small", "{model.model}" } strong { class: "mono small", "{cost}" } } }
                                        }
                                    }
                                }
                            },
                        }
                    }
                }
            }

            Drawer {
                title: "Stored Request Routing Metadata",
                open: receipt_open() && has_latest,
                on_close: move |_| receipt_open.set(false),
                if let Some(log) = latest_for_drawer {
                    {
                        let receipt = route_receipt(&log);
                        rsx! {
                            div { class: "stack-lg",
                                div { class: "product-note",
                                    "This is persisted routing and response metadata. A selected upstream and an HTTP result are operational observations; they do not cryptographically prove which provider executed the request."
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

#[cfg(test)]
mod tests {
    use super::*;

    fn channel(status: i32, models: &str) -> Channel {
        Channel {
            status,
            models: models.to_string(),
            ..Channel::default()
        }
    }

    #[test]
    fn model_readiness_uses_only_enabled_channels_and_deduplicates_models() {
        let channels = vec![
            channel(1, "gpt-4o, claude-3"),
            channel(1, "gpt-4o"),
            channel(0, "disabled-only-model"),
        ];

        assert_eq!(enabled_channel_model_count(&channels), 2);
    }

    #[test]
    fn compact_formats_observed_counts_without_changing_zero() {
        assert_eq!(compact(0), "0");
        assert_eq!(compact(1_250), "1.25K");
        assert_eq!(compact(2_500_000), "2.50M");
    }
}
