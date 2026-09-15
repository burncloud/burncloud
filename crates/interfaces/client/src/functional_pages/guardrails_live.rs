use dioxus::prelude::*;

use crate::{
    app::Route,
    components::Icon,
    functional_api::{
        circuit_breaker_status, emergency_circuit_break, risk_events, save_security_filters,
        security_filters, security_summary, SecurityFilters,
    },
};

fn event_signal(event_type: &str) -> &'static str {
    match event_type {
        "server_error" => "5xx server/upstream error",
        "client_error" => "4xx client/request error",
        _ => "HTTP error",
    }
}

fn event_tone(event_type: &str) -> &'static str {
    if event_type == "server_error" {
        "badge-error"
    } else {
        "badge-warning"
    }
}

#[component]
pub fn Guardrails() -> Element {
    let mut summary_resource = use_resource(move || async move { security_summary().await });
    let mut filters_resource = use_resource(move || async move { security_filters().await });
    let mut events_resource = use_resource(move || async move { risk_events().await });
    let mut breaker_resource = use_resource(move || async move { circuit_breaker_status().await });

    let mut filter_state = use_signal(SecurityFilters::default);
    let mut synced = use_signal(|| false);
    let mut new_rule = use_signal(String::new);
    let mut reason = use_signal(String::new);
    let mut confirm_trip = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let filter_snapshot = filters_resource.read().clone();
    if !synced() {
        if let Some(Ok(server_filters)) = filter_snapshot.clone() {
            filter_state.set(server_filters);
            synced.set(true);
        }
    }

    let summary_snapshot = summary_resource.read().clone();
    let event_snapshot = events_resource.read().clone();
    let breaker_snapshot = breaker_resource.read().clone();

    let summary_loading = summary_snapshot.is_none();
    let summary_error = summary_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let summary = summary_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();

    let filters_loading = filter_snapshot.is_none();
    let filters_error = filter_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let server_filters = filter_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();
    let filters_ready = server_filters.is_some() && synced();

    let events_loading = event_snapshot.is_none();
    let events_error = event_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let events = event_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned();

    let breaker_loading = breaker_snapshot.is_none();
    let breaker_connected = breaker_snapshot
        .as_ref()
        .is_some_and(|result| result.is_ok());
    let breaker_text = match breaker_snapshot {
        Some(Ok(value)) => {
            serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string())
        }
        Some(Err(message)) => format!("Unavailable: {message}"),
        None => "Loading circuit breaker state…".to_string(),
    };

    let filters = filter_state();
    let content_enabled = filters.content_filter_enabled;
    let blacklist_enabled = filters.blacklist_enabled;
    let rules = filters.custom_rules.clone();
    let enabled_controls = usize::from(content_enabled) + usize::from(blacklist_enabled) + rules.len();
    let policy_dirty = filters_ready
        && server_filters
            .as_ref()
            .is_some_and(|saved| saved != &filters);

    let request_health_value = summary
        .as_ref()
        .map(|value| format!("{}/100", value.score))
        .unwrap_or_else(|| "—".to_string());
    let error_event_value = summary
        .as_ref()
        .map(|value| value.blocked_count.to_string())
        .unwrap_or_else(|| "—".to_string());
    let affected_value = summary
        .as_ref()
        .map(|value| value.threat_source_count.to_string())
        .unwrap_or_else(|| "—".to_string());
    let policy_value = if filters_ready {
        enabled_controls.to_string()
    } else {
        "—".to_string()
    };
    let error_event_icon = summary
        .as_ref()
        .map(|value| {
            if value.blocked_count > 0 {
                "metric-icon tone-amber"
            } else {
                "metric-icon tone-gray"
            }
        })
        .unwrap_or("metric-icon tone-gray");
    let refreshing = summary_loading || filters_loading || events_loading || breaker_loading;

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Guardrails" }
                    p { class: "page-subtitle", "Configure traffic protections, review HTTP risk signals, and keep emergency shutdown separate from routine policy changes." }
                }
                button {
                    class: "button button-secondary",
                    disabled: refreshing,
                    onclick: move |_| {
                        synced.set(false);
                        summary_resource.restart();
                        filters_resource.restart();
                        events_resource.restart();
                        breaker_resource.restart();
                    },
                    if refreshing { "Refreshing…" } else { "Refresh" }
                }
            }

            if !notice().is_empty() {
                div { class: "terminal auth-status", "{notice}" }
            }
            if !error().is_empty() {
                div { class: "terminal auth-status auth-status-error", "{error}" }
            }

            if summary_loading {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "shield" } }
                        h3 { "Loading protection signals" }
                        p { "Reading recent router HTTP outcomes before showing request-health or error conclusions." }
                    }
                }
            } else if let Some(message) = summary_error.clone() {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Request protection signals are unavailable" }
                    p { class: "small muted", "BurnCloud will not replace a failed security-summary request with zero-valued health metrics." }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| summary_resource.restart(), "Retry" }
                }
            } else {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Request Health" }
                            span { class: "metric-value", "{request_health_value}" }
                            span { class: "metric-note", "derived from recent HTTP success/error ratio" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "HTTP Error Events" }
                            span { class: "metric-value", "{error_event_value}" }
                            span { class: "metric-note", "4xx + 5xx in the server summary sample" }
                        }
                        div { class: "{error_event_icon}", Icon { name: "logs" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Affected IDs / Upstreams" }
                            span { class: "metric-value", "{affected_value}" }
                            span { class: "metric-note", "distinct IDs involved in HTTP errors" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "routes" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy",
                            span { class: "metric-label", "Policy Controls" }
                            span { class: "metric-value", "{policy_value}" }
                            span { class: "metric-note", "enabled built-ins + custom rules" }
                        }
                        div { class: "metric-icon tone-gray", Icon { name: "settings" } }
                    }
                }

                div { class: "product-note",
                    strong { "Evidence boundary: " }
                    "Request Health and HTTP risk signals are derived from recent router 4xx/5xx logs. They are operational traffic indicators, not a threat-intelligence feed, intrusion detector, or cryptographic security score."
                }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack-lg",
                    div { class: "product-section-head",
                        div {
                            h3 { "Traffic protection policy" }
                            p { "Routine safeguards belong here. Changes are staged locally until Save Protection Policy is pressed." }
                        }
                        if filters_ready {
                            if policy_dirty {
                                span { class: "badge badge-warning", "Unsaved changes" }
                            } else {
                                span { class: "badge badge-neutral", "Saved policy" }
                            }
                        }
                    }

                    if filters_loading {
                        div { class: "product-note", "Loading the current protection policy before controls become editable…" }
                    } else if let Some(message) = filters_error.clone() {
                        div { class: "stack",
                            strong { class: "danger", "Protection policy unavailable" }
                            p { class: "small muted", "BurnCloud will not show default-off controls when the real saved policy could not be loaded." }
                            code { class: "terminal", "{message}" }
                            button {
                                class: "button button-secondary button-sm",
                                onclick: move |_| {
                                    synced.set(false);
                                    filters_resource.restart();
                                },
                                "Retry policy load"
                            }
                        }
                    } else {
                        label { class: "row between",
                            span {
                                div { class: "strong", "Content Filter" }
                                small { class: "muted", "Apply the server's persisted content-filter policy to routed traffic." }
                            }
                            input {
                                r#type: "checkbox",
                                checked: content_enabled,
                                disabled: busy(),
                                onchange: move |_| {
                                    let mut next = filter_state();
                                    next.content_filter_enabled = !next.content_filter_enabled;
                                    filter_state.set(next);
                                },
                            }
                        }

                        label { class: "row between",
                            span {
                                div { class: "strong", "Blacklist" }
                                small { class: "muted", "Enable the persisted blacklist protection." }
                            }
                            input {
                                r#type: "checkbox",
                                checked: blacklist_enabled,
                                disabled: busy(),
                                onchange: move |_| {
                                    let mut next = filter_state();
                                    next.blacklist_enabled = !next.blacklist_enabled;
                                    filter_state.set(next);
                                },
                            }
                        }

                        div { class: "field",
                            label { "Custom rules" }
                            if rules.is_empty() {
                                div { class: "product-note", "No custom rules. BurnCloud will rely on the enabled built-in protections." }
                            }
                            for (index, rule) in rules.iter().enumerate() {
                                {
                                    let rule_index = index;
                                    rsx! {
                                        div { class: "row between card card-pad", key: "{rule_index}",
                                            code { "{rule}" }
                                            button {
                                                class: "button button-ghost button-sm danger",
                                                disabled: busy(),
                                                onclick: move |_| {
                                                    let mut next = filter_state();
                                                    if rule_index < next.custom_rules.len() {
                                                        next.custom_rules.remove(rule_index);
                                                    }
                                                    filter_state.set(next);
                                                },
                                                "Remove"
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "row gap-2",
                                input {
                                    class: "input",
                                    value: "{new_rule}",
                                    disabled: busy(),
                                    placeholder: "Add a custom protection rule",
                                    oninput: move |event| new_rule.set(event.value()),
                                }
                                button {
                                    class: "button button-secondary",
                                    disabled: busy() || new_rule().trim().is_empty(),
                                    onclick: move |_| {
                                        let rule = new_rule().trim().to_string();
                                        if !rule.is_empty() {
                                            let mut next = filter_state();
                                            if !next.custom_rules.iter().any(|existing| existing == &rule) {
                                                next.custom_rules.push(rule);
                                                filter_state.set(next);
                                            }
                                            new_rule.set(String::new());
                                        }
                                    },
                                    "Add Rule"
                                }
                            }
                        }

                        button {
                            class: "button button-primary",
                            disabled: busy() || !filters_ready || !policy_dirty,
                            onclick: move |_| {
                                let payload = filter_state();
                                busy.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    match save_security_filters(&payload).await {
                                        Ok(saved) => {
                                            filter_state.set(saved);
                                            notice.set("Protection policy saved.".to_string());
                                            filters_resource.restart();
                                        }
                                        Err(message) => error.set(format!("Save protection policy failed: {message}")),
                                    }
                                    busy.set(false);
                                });
                            },
                            if busy() { "Saving…" } else { "Save Protection Policy" }
                        }
                    }
                }

                div { class: "card card-pad stack-lg",
                    div { class: "product-section-head",
                        div {
                            h3 { "Routing protection state" }
                            p { "Circuit-breaker telemetry is available only when the router returns its current internal health payload." }
                        }
                        button {
                            class: "button button-ghost button-sm",
                            disabled: breaker_loading,
                            onclick: move |_| breaker_resource.restart(),
                            if breaker_loading { "Checking…" } else { "Refresh" }
                        }
                    }
                    if breaker_loading {
                        div { class: "readiness-strip",
                            span { class: "readiness-dot" }
                            strong { "Checking circuit breaker state" }
                            span { class: "muted", "Waiting for the router's internal health response." }
                        }
                    } else if breaker_connected {
                        div { class: "readiness-strip ready",
                            span { class: "readiness-dot" }
                            strong { "Circuit breaker telemetry available" }
                            span { class: "muted", "The router returned its current internal health state." }
                        }
                    } else {
                        div { class: "readiness-strip blocked",
                            span { class: "readiness-dot" }
                            strong { "Circuit breaker state is unavailable" }
                            span { class: "muted", "Status could not be verified. Emergency stop remains available because a failed status check should not block incident response." }
                        }
                    }
                    details {
                        summary { class: "small strong", style: "cursor:pointer", "View raw router protection state" }
                        pre { class: "terminal", style: "margin-top:12px;max-height:260px;overflow:auto;white-space:pre-wrap", "{breaker_text}" }
                    }
                    div { class: "product-note", "The server currently returns circuit-breaker health as an untyped structured payload. BurnCloud keeps the raw payload available for diagnosis instead of inventing unsupported per-circuit labels." }
                }
            }

            div { class: "card table-card",
                div { class: "card-pad product-section-head",
                    div {
                        h3 { "HTTP risk signals" }
                        p { "Recent request failures derived from router logs. Use Logs for full request-level diagnosis." }
                    }
                    div { class: "row gap-2",
                        if let Some(ref page) = events {
                            span { class: "small muted", "{page.total} error events" }
                        }
                        Link { class: "button button-secondary button-sm", to: Route::Logs {}, "Inspect Request Logs" }
                    }
                }
                if events_loading {
                    div { class: "product-empty",
                        div { class: "product-empty-inner",
                            div { class: "product-empty-icon", Icon { name: "logs" } }
                            h3 { "Loading HTTP error signals" }
                            p { "Reading recent 4xx/5xx router events before drawing conclusions." }
                        }
                    }
                } else if let Some(message) = events_error.clone() {
                    div { class: "card-pad stack",
                        strong { class: "danger", "HTTP error signals unavailable" }
                        code { class: "terminal", "{message}" }
                        button { class: "button button-secondary button-sm", onclick: move |_| events_resource.restart(), "Retry" }
                    }
                } else if let Some(page) = events.clone() {
                    if page.data.is_empty() {
                        div { class: "product-empty",
                            div { class: "product-empty-inner",
                                div { class: "product-empty-icon", Icon { name: "shield" } }
                                h3 { "No HTTP error signals in this view" }
                                p { "The current server query returned no 4xx or 5xx router events." }
                            }
                        }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Time" }
                                    th { "Signal" }
                                    th { "Account / Source ID" }
                                    th { "Upstream / Path" }
                                    th { "HTTP" }
                                } }
                                tbody {
                                    for event in page.data {
                                        {
                                            let signal = event_signal(&event.event_type);
                                            let tone = event_tone(&event.event_type);
                                            rsx! {
                                                tr { key: "{event.id}",
                                                    td { class: "mono muted", title: "{event.time}", "{event.time}" }
                                                    td { span { class: "badge {tone}", "{signal}" } }
                                                    td { class: "mono", title: "{event.source}", "{event.source}" }
                                                    td { class: "mono", title: "{event.target}", "{event.target}" }
                                                    td { class: "mono strong", "{event.detail}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "card-pad product-note", "These rows are HTTP error-derived operational signals. A 4xx does not automatically mean a malicious client, and a 5xx does not automatically mean an attack." }
            }

            div { class: "card card-pad stack-lg danger-zone",
                div { class: "product-section-head",
                    div {
                        h3 { class: "danger", "Emergency traffic stop" }
                        p { "Use only when continuing upstream traffic is more dangerous than an immediate outage." }
                    }
                    span { class: "badge badge-error", "DANGER ZONE" }
                }
                p { class: "small muted", "Trip All Circuits is a live operational command. It can stop upstream routing across the environment and is deliberately separated from normal protection-policy editing." }
                div { class: "field",
                    label { "Incident reason" }
                    textarea {
                        class: "textarea",
                        rows: "3",
                        value: "{reason}",
                        disabled: busy(),
                        placeholder: "Describe why all routing must be stopped now",
                        oninput: move |event| reason.set(event.value()),
                    }
                }
                label { class: "row gap-2 small", style: "align-items:flex-start",
                    input {
                        r#type: "checkbox",
                        checked: confirm_trip(),
                        disabled: busy(),
                        onchange: move |_| confirm_trip.set(!confirm_trip()),
                    }
                    span { "I understand this can interrupt all routed traffic in the environment." }
                }
                div { class: "row",
                    button {
                        class: "button button-danger",
                        disabled: busy() || reason().trim().is_empty() || !confirm_trip(),
                        onclick: move |_| {
                            let incident_reason = reason().trim().to_string();
                            busy.set(true);
                            error.set(String::new());
                            spawn(async move {
                                match emergency_circuit_break(&incident_reason).await {
                                    Ok(_) => {
                                        notice.set("Emergency traffic stop accepted by the router.".to_string());
                                        reason.set(String::new());
                                        confirm_trip.set(false);
                                        breaker_resource.restart();
                                    }
                                    Err(message) => error.set(format!("Emergency traffic stop failed: {message}")),
                                }
                                busy.set(false);
                            });
                        },
                        if busy() { "Stopping Traffic…" } else { "Trip All Circuits" }
                    }
                }
            }
        }
    }
}
