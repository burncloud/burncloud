use dioxus::prelude::*;

use crate::{
    components::Icon,
    functional_api::{
        circuit_breaker_status, emergency_circuit_break, risk_events, save_security_filters,
        security_filters, security_summary, RiskEventPage, SecurityFilters, SecuritySummary,
    },
};

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
    let summary: SecuritySummary = summary_snapshot.clone().and_then(Result::ok).unwrap_or_default();
    let events: RiskEventPage = event_snapshot.clone().and_then(Result::ok).unwrap_or_default();
    let breaker_connected = breaker_snapshot.as_ref().is_some_and(Result::is_ok);
    let breaker_loading = breaker_snapshot.is_none();
    let breaker_text = match breaker_snapshot.clone() {
        Some(Ok(value)) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
        Some(Err(message)) => format!("Unavailable: {message}"),
        None => "Loading circuit breaker state…".to_string(),
    };

    let filters = filter_state();
    let content_enabled = filters.content_filter_enabled;
    let blacklist_enabled = filters.blacklist_enabled;
    let rules = filters.custom_rules.clone();
    let enabled_controls = usize::from(content_enabled) + usize::from(blacklist_enabled) + rules.len();

    let mut load_errors = Vec::new();
    if let Some(Err(message)) = summary_snapshot {
        load_errors.push(format!("Security summary: {message}"));
    }
    if let Some(Err(message)) = filter_snapshot {
        load_errors.push(format!("Filters: {message}"));
    }
    if let Some(Err(message)) = event_snapshot {
        load_errors.push(format!("Risk events: {message}"));
    }
    if let Some(Err(message)) = breaker_snapshot.clone() {
        load_errors.push(format!("Circuit breaker: {message}"));
    }

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Guardrails" }
                    p { class: "page-subtitle", "Set normal traffic protections, review security events, and keep emergency shutdown separate from routine configuration." }
                }
                button {
                    class: "button button-secondary",
                    onclick: move |_| {
                        synced.set(false);
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
                div { class: "card card-pad stack",
                    strong { class: "danger", "Some security data is unavailable" }
                    for message in load_errors { code { class: "terminal", "{message}" } }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Security Score" } span { class: "metric-value", "{summary.score}/100" } span { class: "metric-note", "server-calculated posture" } }
                    div { class: "metric-icon tone-green", Icon { name: "shield" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Blocked / Error Events" } span { class: "metric-value", "{summary.blocked_count}" } span { class: "metric-note", "observed risk events" } }
                    div { class: "metric-icon tone-amber", Icon { name: "logs" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Threat Sources" } span { class: "metric-value", "{summary.threat_source_count}" } span { class: "metric-note", "distinct sources" } }
                    div { class: "metric-icon tone-red", Icon { name: "users" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Policy Controls" } span { class: "metric-value", "{enabled_controls}" } span { class: "metric-note", "enabled toggles + rules" } }
                    div { class: "metric-icon tone-purple", Icon { name: "settings" } }
                }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack-lg",
                    div { class: "product-section-head",
                        div {
                            h3 { "Traffic protection policy" }
                            p { "Routine safeguards belong here. Changes are staged locally until Save Guardrails is pressed." }
                        }
                    }

                    label { class: "row between",
                        span {
                            div { class: "strong", "Content Filter" }
                            small { class: "muted", "Apply the server's content-filtering policy to routed traffic." }
                        }
                        input {
                            r#type: "checkbox",
                            checked: content_enabled,
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
                            input { class: "input", value: "{new_rule}", placeholder: "Add a custom security rule", oninput: move |event| new_rule.set(event.value()) }
                            button {
                                class: "button button-secondary",
                                onclick: move |_| {
                                    let rule = new_rule().trim().to_string();
                                    if !rule.is_empty() {
                                        let mut next = filter_state();
                                        next.custom_rules.push(rule);
                                        filter_state.set(next);
                                        new_rule.set(String::new());
                                    }
                                },
                                "Add Rule"
                            }
                        }
                    }

                    button {
                        class: "button button-primary",
                        disabled: busy(),
                        onclick: move |_| {
                            let payload = filter_state();
                            busy.set(true);
                            error.set(String::new());
                            spawn(async move {
                                match save_security_filters(&payload).await {
                                    Ok(saved) => {
                                        filter_state.set(saved);
                                        notice.set("Guardrail policy saved.".to_string());
                                        filters_resource.restart();
                                    }
                                    Err(message) => error.set(format!("Save guardrails failed: {message}")),
                                }
                                busy.set(false);
                            });
                        },
                        "Save Guardrails"
                    }
                }

                div { class: "card card-pad stack-lg",
                    div { class: "product-section-head",
                        div {
                            h3 { "Protection state" }
                            p { "Circuit-breaker telemetry is shown as available only when the server actually returns it." }
                        }
                        button { class: "button button-ghost button-sm", onclick: move |_| breaker_resource.restart(), "Refresh" }
                    }
                    if breaker_loading {
                        div { class: "product-note", "Loading circuit breaker state…" }
                    } else if breaker_connected {
                        div { class: "readiness-strip ready",
                            span { class: "readiness-dot" }
                            strong { "Circuit breaker telemetry available" }
                        }
                    } else {
                        div { class: "readiness-strip blocked",
                            span { class: "readiness-dot" }
                            strong { "Circuit breaker state is unavailable" }
                            span { class: "muted", "Refresh or review the error above before using emergency controls." }
                        }
                    }
                    details {
                        summary { class: "small strong", style: "cursor:pointer", "View raw circuit breaker state" }
                        pre { class: "terminal", style: "margin-top:12px;max-height:260px;overflow:auto;white-space:pre-wrap", "{breaker_text}" }
                    }
                    div { class: "product-note", "The server currently returns circuit-breaker state as structured operational data. Raw payload remains available for diagnosis instead of inventing unsupported status fields." }
                }
            }

            div { class: "card table-card",
                div { class: "card-pad product-section-head",
                    div {
                        h3 { "Risk events" }
                        p { "Recent HTTP error and security-related events derived by the server from router activity." }
                    }
                    span { class: "small muted", "{events.total} events" }
                }
                if events.data.is_empty() {
                    div { class: "product-empty", style: "min-height:170px",
                        div { class: "product-empty-inner",
                            div { class: "product-empty-icon", Icon { name: "shield" } }
                            h3 { "No risk events in this view" }
                            p { "No HTTP 4xx/5xx security events were returned by the current server query." }
                        }
                    }
                } else {
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr { th { "Time" } th { "Severity" } th { "Type" } th { "Source" } th { "Target" } th { "Status" } th { "Detail" } } }
                            tbody {
                                for event in events.data {
                                    tr { key: "{event.id}",
                                        td { class: "mono muted", "{event.time}" }
                                        td { span { class: if event.severity == "critical" { "badge badge-error" } else { "badge badge-warning" }, "{event.severity}" } }
                                        td { "{event.event_type}" }
                                        td { class: "mono", "{event.source}" }
                                        td { class: "mono", "{event.target}" }
                                        td { "{event.status}" }
                                        td { "{event.detail}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "card card-pad stack-lg danger-zone",
                div { class: "product-section-head",
                    div {
                        h3 { class: "danger", "Emergency traffic stop" }
                        p { "Use only when continuing upstream traffic is more dangerous than an immediate outage." }
                    }
                    span { class: "badge badge-error", "DANGER ZONE" }
                }
                p { class: "small muted", "Trip All Circuits is a real operational command. It can stop upstream routing across the environment. This action is deliberately separated from normal guardrail editing." }
                div { class: "field",
                    label { "Incident reason" }
                    textarea { class: "textarea", rows: "3", value: "{reason}", placeholder: "Describe why all routing must be stopped now", oninput: move |event| reason.set(event.value()) }
                }
                label { class: "row gap-2 small", style: "align-items:flex-start",
                    input { r#type: "checkbox", checked: confirm_trip(), onchange: move |_| confirm_trip.set(!confirm_trip()) }
                    span { "I understand this can interrupt all routed traffic in the environment." }
                }
                div { class: "row",
                    button {
                        class: "button button-primary",
                        disabled: busy() || reason().trim().is_empty() || !confirm_trip(),
                        onclick: move |_| {
                            let incident_reason = reason().trim().to_string();
                            busy.set(true);
                            error.set(String::new());
                            spawn(async move {
                                match emergency_circuit_break(&incident_reason).await {
                                    Ok(value) => {
                                        notice.set(format!("Emergency circuit break accepted: {value}"));
                                        reason.set(String::new());
                                        confirm_trip.set(false);
                                        breaker_resource.restart();
                                    }
                                    Err(message) => error.set(format!("Emergency circuit break failed: {message}")),
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
