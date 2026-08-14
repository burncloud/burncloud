use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{
        billing_summary, user_usage, BillingSummary, Channel, ChannelService, LogService, RouterLog,
        TokenDto, TokenService, UsageStats,
    },
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

fn model_count(channels: &[Channel], active_only: bool) -> usize {
    let mut models = BTreeSet::new();
    for channel in channels {
        if active_only && channel.status != 1 {
            continue;
        }
        for model in channel
            .models
            .split(',')
            .map(str::trim)
            .filter(|model| !model.is_empty())
        {
            models.insert(model.to_string());
        }
    }
    models.len()
}

fn route_group_count(channels: &[Channel]) -> usize {
    let mut groups = BTreeSet::new();
    for channel in channels {
        for group in channel
            .group
            .split(',')
            .map(str::trim)
            .filter(|group| !group.is_empty())
        {
            groups.insert(group.to_string());
        }
    }
    groups.len()
}

fn metric(
    label: &'static str,
    value: String,
    note: String,
    icon: &'static str,
    tone: &'static str,
) -> Element {
    rsx! {
        div { class: "card metric",
            div { class: "metric-copy",
                span { class: "metric-label", "{label}" }
                span { class: "metric-value", "{value}" }
                span { class: "metric-note", "{note}" }
            }
            div { class: "metric-icon {tone}", Icon { name: icon } }
        }
    }
}

#[component]
fn EvidenceStep(
    state: &'static str,
    tone: &'static str,
    title: &'static str,
    detail: String,
    to: Route,
    action: &'static str,
) -> Element {
    rsx! {
        div { class: "setup-step",
            div { class: "setup-step-dot", "·" }
            div {
                div { class: "row gap-2",
                    strong { "{title}" }
                    span { class: "badge {tone}", "{state}" }
                }
                small { "{detail}" }
            }
            Link { class: "button button-ghost button-sm", to: to, "{action}" }
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

    let channels_loading = channels_result.is_none();
    let logs_loading = logs_result.is_none();
    let tokens_loading = tokens_result.is_none();
    let usage_loading = usage_result.is_none();
    let billing_loading = billing_result.is_none();
    let readiness_loading = channels_loading || logs_loading || tokens_loading;
    let refreshing = readiness_loading || usage_loading || billing_loading;

    let channels: Vec<Channel> = channels_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let logs: Vec<RouterLog> = logs_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let api_tokens: Vec<TokenDto> = tokens_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let usage: Option<UsageStats> = usage_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();
    let billing: Option<BillingSummary> = billing_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();

    let readiness_errors: Vec<String> = [
        channels_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("Supply: {error}")),
        logs_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("Request evidence: {error}")),
        tokens_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("API access: {error}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let supporting_errors: Vec<String> = [
        usage_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("Current user usage: {error}")),
        billing_result
            .as_ref()
            .and_then(|value| value.as_ref().err())
            .map(|error| format!("Billing summary: {error}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let has_readiness_errors = !readiness_errors.is_empty();
    let has_any_errors = has_readiness_errors || !supporting_errors.is_empty();
    let all_errors: Vec<String> = readiness_errors
        .iter()
        .chain(supporting_errors.iter())
        .cloned()
        .collect();

    let active_providers = channels.iter().filter(|channel| channel.status == 1).count();
    let providers_needing_review = channels.len().saturating_sub(active_providers);
    let configured_models = model_count(&channels, false);
    let available_models = model_count(&channels, true);
    let unavailable_models = configured_models.saturating_sub(available_models);
    let configured_route_groups = route_group_count(&channels);
    let active_keys = api_tokens.iter().filter(|key| key.status == "active").count();
    let has_request = !logs.is_empty();
    let has_successful_request = logs.iter().any(|log| (200..300).contains(&log.status_code));

    let supply_known = channels_result.as_ref().is_some_and(|result| result.is_ok());
    let access_known = tokens_result.as_ref().is_some_and(|result| result.is_ok());
    let request_evidence_known = logs_result.as_ref().is_some_and(|result| result.is_ok());

    let (status_class, badge_class, status_badge, status_title, status_copy) = if readiness_loading {
        (
            "product-status-card",
            "badge badge-neutral",
            "CHECKING",
            "Building an evidence-backed overview",
            "BurnCloud is loading supply, API access, and persisted request evidence before drawing a readiness conclusion.",
        )
    } else if has_readiness_errors {
        (
            "product-status-card status-blocked",
            "badge badge-error",
            "UNKNOWN",
            "Overview evidence is incomplete",
            "One or more readiness sources could not be read. BurnCloud will not replace missing evidence with zero-valued readiness conclusions.",
        )
    } else if active_providers == 0 {
        (
            "product-status-card status-blocked",
            "badge badge-error",
            "BLOCKED",
            "Supply is not available",
            "No active provider is currently visible to Overview. Start with Providers before testing traffic.",
        )
    } else if available_models == 0 {
        (
            "product-status-card status-blocked",
            "badge badge-error",
            "BLOCKED",
            "No model is currently available",
            "Provider supply exists, but none of the configured model IDs are exposed by an active provider.",
        )
    } else if active_keys == 0 {
        (
            "product-status-card status-attention",
            "badge badge-warning",
            "ATTENTION",
            "Create API access before verification",
            "Supply is available, but Overview cannot find an active API key for an end-to-end request test.",
        )
    } else if !has_successful_request {
        (
            "product-status-card status-attention",
            "badge badge-warning",
            "ATTENTION",
            "Configuration is present; verification is still missing",
            if has_request {
                "Request evidence exists, but no HTTP 2xx result is visible in the recent log sample. Use Playground and Logs to verify the request path."
            } else {
                "Supply and API access are present, but no persisted request evidence is visible yet. Run a controlled request in Playground."
            },
        )
    } else {
        (
            "product-status-card status-ready",
            "badge badge-success",
            "VERIFIED",
            "Verified traffic is observable",
            "Overview can see active supply, API access, and at least one persisted HTTP 2xx request. Detailed diagnosis remains on the owning pages.",
        )
    };

    let supply_state = if !supply_known {
        ("UNKNOWN", "badge-neutral")
    } else if active_providers == 0 || available_models == 0 {
        ("NOT AVAILABLE", "badge-error")
    } else {
        ("AVAILABLE", "badge-success")
    };
    let access_state = if !access_known {
        ("UNKNOWN", "badge-neutral")
    } else if active_keys == 0 {
        ("NOT SET", "badge-warning")
    } else {
        ("CONFIGURED", "badge-success")
    };
    let verify_state = if !request_evidence_known {
        ("UNKNOWN", "badge-neutral")
    } else if has_successful_request {
        ("VERIFIED", "badge-success")
    } else {
        ("NOT VERIFIED", "badge-warning")
    };
    let observe_state = if !request_evidence_known {
        ("UNKNOWN", "badge-neutral")
    } else if has_request {
        ("OBSERVED", "badge-success")
    } else {
        ("NO EVIDENCE", "badge-neutral")
    };

    let supply_detail = if supply_known {
        format!(
            "{active_providers} active providers • {available_models} available models • {configured_route_groups} configured routing groups"
        )
    } else {
        "Supply evidence is not available yet.".to_string()
    };
    let access_detail = if access_known {
        format!("{active_keys} active API keys")
    } else {
        "API access evidence is not available yet.".to_string()
    };
    let verify_detail = if request_evidence_known {
        if has_successful_request {
            "At least one HTTP 2xx request is visible in the recent router-log sample.".to_string()
        } else if has_request {
            "Requests are visible, but the recent sample contains no HTTP 2xx result.".to_string()
        } else {
            "No persisted request is visible in the recent router-log sample.".to_string()
        }
    } else {
        "Request verification evidence is unavailable.".to_string()
    };
    let observe_detail = if request_evidence_known {
        format!("{} recent persisted request records loaded", logs.len())
    } else {
        "Operational request evidence is unavailable.".to_string()
    };

    let total_requests = billing.as_ref().map(|summary| {
        summary.models.iter().map(|model| model.requests).sum::<i64>() + summary.pre_migration_requests
    });
    let request_text = total_requests.map(compact).unwrap_or_else(|| "—".to_string());
    let token_text = usage
        .as_ref()
        .map(|value| compact(value.total_tokens))
        .unwrap_or_else(|| "—".to_string());
    let spend_text = billing
        .as_ref()
        .map(|value| format!("${:.2}", value.total_cost_usd))
        .unwrap_or_else(|| "—".to_string());
    let latest_http = logs
        .first()
        .map(|log| log.status_code.to_string())
        .unwrap_or_else(|| "—".to_string());

    let request_note = if billing.is_some() {
        "billing summary scope".to_string()
    } else {
        "billing evidence unavailable".to_string()
    };
    let token_note = if usage.is_some() {
        "current signed-in user scope".to_string()
    } else {
        "current user usage unavailable".to_string()
    };
    let spend_note = if billing.is_some() {
        "billing summary scope".to_string()
    } else {
        "billing evidence unavailable".to_string()
    };
    let latest_http_note = if has_request {
        "latest persisted router log".to_string()
    } else if request_evidence_known {
        "no request evidence yet".to_string()
    } else {
        "request evidence unavailable".to_string()
    };

    let latest = logs.first().cloned();
    let has_attention = providers_needing_review > 0
        || unavailable_models > 0
        || (has_request && !has_successful_request);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Overview" }
                    p { class: "page-subtitle", "Evidence-backed overview: understand the current conclusion, the next action, and which page owns the detail." }
                }
                button {
                    class: "button button-secondary",
                    disabled: refreshing,
                    onclick: move |_| {
                        channels_resource.restart();
                        logs_resource.restart();
                        tokens_resource.restart();
                        usage_resource.restart();
                        billing_resource.restart();
                    },
                    Icon { name: "activity" }
                    if refreshing { "Refreshing…" } else { "Refresh" }
                }
            }

            div { class: "product-hero",
                div { class: "card {status_class}",
                    div { class: "row gap-2",
                        span { class: "{badge_class}", "{status_badge}" }
                    }
                    div {
                        div { class: "product-status-title", "{status_title}" }
                        p { class: "product-status-copy", "{status_copy}" }
                    }
                    div { class: "product-actions",
                        if !readiness_loading && !has_readiness_errors && active_providers == 0 {
                            Link { class: "button button-primary", to: Route::Providers {}, "Open Providers" }
                        } else if !readiness_loading && !has_readiness_errors && available_models == 0 {
                            Link { class: "button button-primary", to: Route::Models {}, "Open Models" }
                        } else if !readiness_loading && !has_readiness_errors && active_keys == 0 {
                            Link { class: "button button-primary", to: Route::APIKeys {}, "Create API key" }
                        } else if !readiness_loading && !has_readiness_errors && !has_successful_request {
                            Link { class: "button button-primary", to: Route::Playground {}, "Open Playground" }
                        } else if !readiness_loading && !has_readiness_errors {
                            Link { class: "button button-primary", to: Route::Logs {}, "Open Logs" }
                        }
                        Link { class: "button button-secondary", to: Route::Routes {}, "Review Routes" }
                    }
                }

                div { class: "card setup-card",
                    div { class: "product-section-head",
                        div {
                            h3 { "Product flow evidence" }
                            p { "Overview summarizes evidence. Configuration and diagnosis stay on the owning pages." }
                        }
                    }
                    div { class: "setup-list",
                        EvidenceStep {
                            state: supply_state.0,
                            tone: supply_state.1,
                            title: "Supply",
                            detail: supply_detail,
                            to: Route::Routes {},
                            action: "Routes"
                        }
                        EvidenceStep {
                            state: access_state.0,
                            tone: access_state.1,
                            title: "Access",
                            detail: access_detail,
                            to: Route::APIKeys {},
                            action: "API Keys"
                        }
                        EvidenceStep {
                            state: verify_state.0,
                            tone: verify_state.1,
                            title: "Verify",
                            detail: verify_detail,
                            to: Route::Playground {},
                            action: "Playground"
                        }
                        EvidenceStep {
                            state: observe_state.0,
                            tone: observe_state.1,
                            title: "Observe",
                            detail: observe_detail,
                            to: Route::Logs {},
                            action: "Logs"
                        }
                    }
                }
            }

            if has_any_errors {
                div { class: "card card-pad stack",
                    div { class: "product-section-head",
                        div {
                            h3 { class: "danger", "Evidence sources need attention" }
                            p { "Unknown data stays unknown instead of becoming a zero or healthy state." }
                        }
                    }
                    for message in all_errors.iter() {
                        code { class: "terminal", "{message}" }
                    }
                }
            }

            div { class: "card card-pad stack-lg",
                div { class: "product-section-head",
                    div {
                        h3 { "Needs attention" }
                        p { "Only cross-page conclusions belong here. Use the owning page for diagnosis or changes." }
                    }
                }
                if !has_readiness_errors && !readiness_loading && has_attention {
                    div { class: "stack",
                        if providers_needing_review > 0 {
                            div { class: "row between",
                                div { class: "two-line",
                                    strong { "Provider state needs review" }
                                    small { class: "muted", "{providers_needing_review} configured providers are not active. Overview does not diagnose provider cause." }
                                }
                                Link { class: "button button-ghost button-sm", to: Route::Providers {}, "Open Providers" }
                            }
                        }
                        if unavailable_models > 0 {
                            div { class: "row between",
                                div { class: "two-line",
                                    strong { "Model availability is reduced" }
                                    small { class: "muted", "{unavailable_models} configured model IDs have no active upstream in the loaded supply evidence." }
                                }
                                Link { class: "button button-ghost button-sm", to: Route::Models {}, "Open Models" }
                            }
                        }
                        if has_request && !has_successful_request {
                            div { class: "row between",
                                div { class: "two-line",
                                    strong { "Recent requests are not yet verified" }
                                    small { class: "muted", "The loaded router-log sample contains requests but no HTTP 2xx result." }
                                }
                                Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Open Logs" }
                            }
                        }
                    }
                } else if !has_readiness_errors && !readiness_loading {
                    div { class: "product-note", "No Overview-level attention item is derived from the currently loaded readiness evidence." }
                } else {
                    div { class: "product-note", "Attention conclusions wait until readiness evidence is available." }
                }
            }

            div { class: "card card-pad stack-lg",
                div { class: "product-section-head",
                    div {
                        h3 { "Observed activity" }
                        p { "Compact evidence with explicit scope; detailed cost, performance, and request analysis remain in Billing, Evaluation, and Logs." }
                    }
                    div { class: "row gap-2",
                        Link { class: "button button-ghost button-sm", to: Route::Evaluation {}, "Evaluation" }
                        Link { class: "button button-ghost button-sm", to: Route::Billing {}, "Billing" }
                    }
                }
                div { class: "metrics",
                    {metric("Billing Requests", request_text, request_note, "activity", "tone-blue")}
                    {metric("Current User Tokens", token_text, token_note, "models", "tone-purple")}
                    {metric("Billing Spend", spend_text, spend_note, "dollar", "tone-amber")}
                    {metric("Latest HTTP", latest_http, latest_http_note, "logs", "tone-gray")}
                }
            }

            div { class: "card card-pad stack-lg",
                div { class: "product-section-head",
                    div {
                        h3 { "Latest request evidence" }
                        p { "One persisted request is enough for orientation; Logs owns request-level diagnosis and routing metadata." }
                    }
                    Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Open Logs" }
                }
                if let Some(log) = latest {
                    {
                        let model = log.model.clone().unwrap_or_else(|| "-".to_string());
                        let upstream = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                        let result = format!("HTTP {} • {}", log.status_code, log.status_label());
                        rsx! {
                            div { class: "receipt",
                                div { class: "receipt-row", label { "Request" } strong { class: "mono", "{log.request_id}" } }
                                div { class: "receipt-row", label { "Result" } strong { "{result}" } }
                                div { class: "receipt-row", label { "Model" } strong { class: "mono", "{model}" } }
                                div { class: "receipt-row", label { "Upstream" } strong { class: "mono", "{upstream}" } }
                            }
                        }
                    }
                } else if request_evidence_known {
                    div { class: "product-empty", style: "min-height:140px",
                        div { class: "product-empty-inner",
                            div { class: "product-empty-icon", Icon { name: "logs" } }
                            h3 { "No request evidence yet" }
                            p { "Use Playground for a controlled end-to-end request. Once persisted, the request becomes observable here and diagnosable in Logs." }
                            Link { class: "button button-primary button-sm", to: Route::Playground {}, "Open Playground" }
                        }
                    }
                } else {
                    div { class: "product-note", "Latest request evidence is unavailable, so Overview will not infer request health." }
                }
            }

            div { class: "product-note",
                strong { "Responsibility boundary: " }
                "Overview summarizes the highest-confidence evidence and routes the operator to the owning page. Provider diagnosis belongs to Providers, routing configuration to Routes, request metadata to Logs, performance analysis to Evaluation, billing detail to Billing, and host/runtime state to Settings."
            }
        }
    }
}

#[component]
pub fn Dashboard() -> Element {
    rsx! { Overview {} }
}
