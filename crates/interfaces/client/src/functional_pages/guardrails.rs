use dioxus::prelude::*;

use crate::{
    functional_api::{
        circuit_breaker_status, emergency_circuit_break, risk_events, save_security_filters,
        security_filters, security_summary, RiskEventPage, SecurityFilters, SecuritySummary,
    },
    components::Icon,
};

#[component]
pub fn Guardrails() -> Element {
    let mut summary_resource = use_resource(move || async move { security_summary().await });
    let mut filters_resource = use_resource(move || async move { security_filters().await });
    let mut events_resource = use_resource(move || async move { risk_events().await });
    let mut breaker_resource = use_resource(move || async move { circuit_breaker_status().await });
    let mut filter_override = use_signal(|| None::<SecurityFilters>);
    let mut new_rule = use_signal(String::new);
    let mut reason = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let summary_result = summary_resource.read().clone();
    let filters_result = filters_resource.read().clone();
    let events_result = events_resource.read().clone();
    let breaker_result = breaker_resource.read().clone();

    let summary: SecuritySummary = summary_result.clone().and_then(Result::ok).unwrap_or_default();
    let server_filters: SecurityFilters = filters_result.clone().and_then(Result::ok).unwrap_or_default();
    let filters = filter_override().unwrap_or(server_filters);
    let events: RiskEventPage = events_result.clone().and_then(Result::ok).unwrap_or_default();
    let breaker_text = match breaker_result {
        Some(Ok(value)) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
        Some(Err(message)) => format!("Unavailable: {message}"),
        None => "Loading circuit breaker status…".to_string(),
    };

    let load_errors: Vec<String> = [
        summary_result.as_ref().and_then(|r| r.as_ref().err()).map(|e| format!("Security summary: {e}")),
        filters_result.as_ref().and_then(|r| r.as_ref().err()).map(|e| format!("Filters: {e}")),
        events_result.as_ref().and_then(|r| r.as_ref().err()).map(|e| format!("Risk events: {e}")),
    ].into_iter().flatten().collect();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Guardrails" }
                    p { class: "page-subtitle", "Live security score, router-derived risk events, persistent filter settings, and the real emergency circuit breaker." }
                }
                button {
                    class: "button button-secondary",
                    onclick: move |_| {
                        filter_override.set(None);
                        summary_resource.restart();
                        filters_resource.restart();
                        events_resource.restart();
                        breaker_resource.restart();
                    },
                    "Refresh"
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }
            if !load_errors.is_empty() {
                div { class: "card card-pad stack", for message in load_errors { code { class: "terminal", "{message}" } } }
            }

            div { class: "metrics",
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Security Score" } span { class: "metric-value", "{summary.score}/100" } } div { class: "metric-icon tone-green", Icon { name: "shield" } } }
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Blocked / Error Events" } span { class: "metric-value", "{summary.blocked_count}" } } div { class: "metric-icon tone-amber", Icon { name: "logs" } } }
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Threat Sources" } span { class: "metric-value", "{summary.threat_source_count}" } } div { class: "metric-icon tone-red", Icon { name: "users" } } }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack-lg",
                    div { class: "row between", h3 { "Security Filters" } span { class: "badge badge-neutral", "sys_settings" } }
                    label { class: "row between",
                        span { div { class: "strong", "Content Filter" } small { class: "muted", "Persistent security_filters.content_filter_enabled" } }
                        input {
                            r#type: "checkbox",
                            checked: filters.content_filter_enabled,
                            onchange: move |_| {
                                let mut next = filters.clone();
                                next.content_filter_enabled = !next.content_filter_enabled;
                                filter_override.set(Some(next));
                            },
                        }
                    }
                    label { class: "row between",
                        span { div { class: "strong", "Blacklist" } small { class: "muted", "Persistent security_filters.blacklist_enabled" } }
                        input {
                            r#type: "checkbox",
                            checked: filters.blacklist_enabled,
                            onchange: move |_| {
                                let mut next = filters.clone();
                                next.blacklist_enabled = !next.blacklist_enabled;
                                filter_override.set(Some(next));
                            },
                        }
                    }
                    div { class: "field",
                        label { "Custom Rules" }
                        for (index, rule) in filters.custom_rules.iter().enumerate() {
                            div { class: "row between card card-pad", key: "{index}",
                                code { "{rule}" }
                                button {
                                    class: "button button-ghost button-sm danger",
                                    onclick: move |_| {
                                        let mut next = filters.clone();
                                        if index < next.custom_rules.len() { next.custom_rules.remove(index); }
                                        filter_override.set(Some(next));
                                    },
                                    "Remove"
                                }
                            }
                        }
                        div { class: "row gap-2",
                            input { class: "input", value: "{new_rule}", placeholder: "Add custom security rule", oninput: move |evt| new_rule.set(evt.value()) }
                            button {
                                class: "button button-secondary",
                                onclick: move |_| {
                                    let rule = new_rule().trim().to_string();
                                    if !rule.is_empty() {
                                        let mut next = filters.clone();
                                        next.custom_rules.push(rule);
                                        filter_override.set(Some(next));
                                        new_rule.set(String::new());
                                    }
                                },
                                "Add"
                            }
                        }
                    }
                    button {
                        class: "button button-primary",
                        disabled: busy(),
                        onclick: move |_| {
                            let payload = filters.clone();
                            busy.set(true);
                            error.set(String::new());
                            spawn(async move {
                                match save_security_filters(&payload).await {
                                    Ok(saved) => { filter_override.set(Some(saved)); notice.set("Security filters saved to BurnCloud settings.".to_string()); filters_resource.restart(); }
                                    Err(message) => error.set(format!("Save filters failed: {message}")),
                                }
                                busy.set(false);
                            });
                        },
                        "Save Guardrails"
                    }
                }

                div { class: "stack-lg",
                    div { class: "card card-pad stack",
                        div { class: "row between", h3 { "Circuit Breaker Status" } button { class: "button button-ghost button-sm", onclick: move |_| breaker_resource.restart(), "Refresh" } }
                        pre { class: "terminal", style: "max-height:240px;overflow:auto;white-space:pre-wrap", "{breaker_text}" }
                    }
                    div { class: "card card-pad stack",
                        h3 { class: "danger", "Emergency Circuit Break" }
                        p { class: "small muted", "This is a real operational action. It proxies to the router trip-all endpoint and can stop upstream routing." }
                        textarea { class: "textarea", rows: "3", value: "{reason}", placeholder: "Required reason for emergency stop", oninput: move |evt| reason.set(evt.value()) }
                        button {
                            class: "button button-primary",
                            disabled: busy() || reason().trim().is_empty(),
                            onclick: move |_| {
                                let why = reason().trim().to_string();
                                busy.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    match emergency_circuit_break(&why).await {
                                        Ok(value) => { notice.set(format!("Emergency circuit break accepted: {value}")); breaker_resource.restart(); }
                                        Err(message) => error.set(format!("Emergency circuit break failed: {message}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            "Trip All Circuits"
                        }
                    }
                }
            }

            div { class: "card table-card",
                div { class: "card-pad row between", h3 { "Risk Events" } span { class: "small muted", "{events.total} events" } }
                if events.data.is_empty() {
                    div { class: "card-pad small muted", "No HTTP 4xx/5xx risk events were derived from router logs." }
                } else {
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr { th { "Time" } th { "Source" } th { "Target" } th { "Type" } th { "Severity" } th { "Status" } th { "Detail" } } }
                            tbody {
                                for event in events.data {
                                    tr { key: "{event.id}",
                                        td { class: "mono muted", "{event.time}" }
                                        td { class: "mono", "{event.source}" }
                                        td { class: "mono", "{event.target}" }
                                        td { "{event.event_type}" }
                                        td { span { class: if event.severity == "critical" { "badge badge-error" } else { "badge badge-warning" }, "{event.severity}" } }
                                        td { "{event.status}" }
                                        td { "{event.detail}" }
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
