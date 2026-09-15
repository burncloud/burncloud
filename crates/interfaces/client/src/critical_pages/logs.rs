use dioxus::prelude::*;

use crate::{
    backend::{LogService, RouterLog},
    components::{Badge, Drawer, Icon},
};

fn status_tone(status: &str) -> &'static str {
    match status {
        "Success" => "success",
        "Fallback" => "warning",
        "Timeout" | "Error" => "error",
        _ => "neutral",
    }
}

fn detail_stat(label: &str, value: String, mono: bool) -> Element {
    let class = if mono { "small strong mono" } else { "small strong" };
    rsx! {
        div {
            span { class: "tiny subtle", "{label}" }
            div { class: "{class}", "{value}" }
        }
    }
}

fn timeline_step(tone: &str, text: String) -> Element {
    let dot_color = match tone {
        "red" => "#ef4444",
        "amber" => "#f59e0b",
        "green" => "#22c55e",
        "blue" => "#60a5fa",
        _ => "#d1d5db",
    };
    rsx! {
        div { class: "row gap-2", style: "align-items:flex-start",
            span { style: "width:10px;height:10px;border-radius:50%;background:{dot_color};margin-top:4px;flex:0 0 auto" }
            span { class: "small muted", "{text}" }
        }
    }
}

#[component]
pub fn Logs() -> Element {
    let mut selected = use_signal(|| None::<RouterLog>);
    let mut query = use_signal(String::new);
    let mut filter = use_signal(|| "all".to_string());
    let mut logs = use_resource(move || async move { LogService::list(200).await });

    let resource = logs.read().clone();
    let loading = resource.is_none();
    let error = resource.as_ref().and_then(|result| result.as_ref().err().cloned());
    let log_list = resource.and_then(Result::ok).unwrap_or_default();
    let query_value = query().to_lowercase();
    let filter_value = filter();

    let visible_logs: Vec<RouterLog> = log_list
        .iter()
        .filter(|log| {
            let status = log.status_label();
            let status_match = filter_value == "all" || status.to_lowercase() == filter_value;
            let search_match = query_value.is_empty()
                || log.request_id.to_lowercase().contains(&query_value)
                || log.path.to_lowercase().contains(&query_value)
                || log.user_id.as_deref().unwrap_or("").to_lowercase().contains(&query_value)
                || log.model.as_deref().unwrap_or("").to_lowercase().contains(&query_value)
                || log.upstream_id.as_deref().unwrap_or("").to_lowercase().contains(&query_value)
                || log.layer_decision.as_deref().unwrap_or("").to_lowercase().contains(&query_value);
            status_match && search_match
        })
        .cloned()
        .collect();

    let success_count = log_list.iter().filter(|l| l.status_label() == "Success").count();
    let fallback_count = log_list.iter().filter(|l| l.status_label() == "Fallback").count();
    let error_count = log_list.len().saturating_sub(success_count + fallback_count);
    let avg_latency = if log_list.is_empty() {
        0
    } else {
        log_list.iter().map(|l| l.latency_ms).sum::<i64>() / log_list.len() as i64
    };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Logs" }
                    p { class: "page-subtitle", "Live router observability from /console/api/logs — no seeded rows." }
                }
                div { class: "header-actions",
                    div { class: "search-field", style: "width:288px",
                        Icon { name: "search" }
                        input {
                            class: "input",
                            placeholder: "Request, user, path, model, upstream…",
                            value: "{query}",
                            oninput: move |evt| query.set(evt.value()),
                        }
                    }
                    select {
                        class: "select",
                        value: "{filter}",
                        onchange: move |evt| filter.set(evt.value()),
                        option { value: "all", "All statuses" }
                        option { value: "success", "Success" }
                        option { value: "fallback", "Fallback" }
                        option { value: "timeout", "Timeout" }
                        option { value: "error", "Error" }
                    }
                    button {
                        r#type: "button",
                        class: "button button-secondary",
                        onclick: move |_| logs.restart(),
                        "Refresh"
                    }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Loaded Requests" } span { class: "metric-value", "{log_list.len()}" } }
                    div { class: "metric-icon tone-blue", Icon { name: "logs" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Success" } span { class: "metric-value", "{success_count}" } }
                    div { class: "metric-icon tone-green", Icon { name: "shield" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Fallback / Errors" } span { class: "metric-value", "{fallback_count} / {error_count}" } }
                    div { class: "metric-icon tone-amber", Icon { name: "routes" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Average Latency" } span { class: "metric-value", "{avg_latency}ms" } }
                    div { class: "metric-icon tone-purple", Icon { name: "activity" } }
                }
            }

            if loading {
                div { class: "card card-pad", "Loading router logs…" }
            } else if let Some(message) = error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Unable to load router logs" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| logs.restart(), "Retry" }
                }
            } else {
                div { class: "card table-card",
                    if visible_logs.is_empty() {
                        div { class: "card-pad small muted", "No router-log rows match the current search/filter." }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Timestamp" }
                                    th { "Request ID" }
                                    th { "User" }
                                    th { "Path / Model" }
                                    th { "Route Decision" }
                                    th { "Status" }
                                    th { class: "right", "Latency" }
                                    th { class: "right", "Tokens" }
                                    th { class: "right", "Cost" }
                                } }
                                tbody {
                                    for log in visible_logs {
                                        {
                                            let status = log.status_label();
                                            let timestamp = log.created_at.clone().unwrap_or_else(|| "—".to_string());
                                            let user = log.user_id.clone().unwrap_or_else(|| "anonymous".to_string());
                                            let model = log.model.clone().unwrap_or_else(|| "—".to_string());
                                            let upstream = log.upstream_id.clone().unwrap_or_else(|| "—".to_string());
                                            let decision = log.layer_decision.clone().unwrap_or_else(|| "direct".to_string());
                                            let tokens = log.total_tokens();
                                            let cost = log.cost_usd();
                                            rsx! {
                                                tr {
                                                    key: "{log.id}-{log.request_id}",
                                                    onclick: move |_| selected.set(Some(log.clone())),
                                                    style: "cursor:pointer",
                                                    td { class: "mono muted", "{timestamp}" }
                                                    td { class: "mono table-primary", "{log.request_id}" }
                                                    td { "{user}" }
                                                    td {
                                                        div { class: "two-line",
                                                            span { class: "table-primary mono", "{log.path}" }
                                                            small { "{model} • {upstream}" }
                                                        }
                                                    }
                                                    td { class: "mono muted", "{decision}" }
                                                    td { Badge { text: status, tone: status_tone(status) } }
                                                    td { class: "right muted tabular", "{log.latency_ms}ms" }
                                                    td { class: "right muted tabular", "{tokens}" }
                                                    td { class: "right strong tabular", "${cost:.6}" }
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

            Drawer {
                title: "Request Detail",
                open: selected().is_some(),
                on_close: move |_| selected.set(None),
                if let Some(log) = selected() {
                    {
                        let status = log.status_label();
                        let upstream = log.upstream_id.clone().unwrap_or_else(|| "—".to_string());
                        let model = log.model.clone().unwrap_or_else(|| "—".to_string());
                        let decision = log.layer_decision.clone().unwrap_or_else(|| "direct".to_string());
                        let traffic = log.traffic_color.clone().unwrap_or_else(|| "—".to_string());
                        let error_type = log.error_type.clone().unwrap_or_else(|| "none".to_string());
                        let pricing_region = log.pricing_region.clone().unwrap_or_else(|| "—".to_string());
                        let cost_status = log.cost_status.clone().unwrap_or_else(|| "—".to_string());
                        rsx! {
                            div { class: "stack-lg",
                                div { class: "card card-pad", style: "display:grid;grid-template-columns:repeat(3,1fr);gap:16px",
                                    {detail_stat("Request ID", log.request_id.clone(), true)}
                                    {detail_stat("Status", status.to_string(), false)}
                                    {detail_stat("Total Cost", format!("${:.6}", log.cost_usd()), true)}
                                    {detail_stat("Prompt Tokens", log.prompt_tokens.to_string(), true)}
                                    {detail_stat("Completion Tokens", log.completion_tokens.to_string(), true)}
                                    {detail_stat("Latency", format!("{}ms", log.latency_ms), true)}
                                }

                                div { class: "stack",
                                    h3 { class: "section-label", "Real Routing Timeline Metadata" }
                                    {timeline_step("blue", format!("Path accepted: {}", log.path))}
                                    {timeline_step("blue", format!("Model: {model}"))}
                                    {timeline_step("blue", format!("Upstream: {upstream}"))}
                                    {timeline_step(if decision.contains("failover") { "amber" } else { "green" }, format!("Layer decision: {decision}"))}
                                    {timeline_step(if status == "Success" { "green" } else { "red" }, format!("HTTP {} • error_type={error_type}", log.status_code))}
                                }

                                div { class: "card card-pad stack",
                                    h3 { class: "section-label", "Observability Fields" }
                                    div { class: "grid-2",
                                        {detail_stat("Traffic Color", traffic, true)}
                                        {detail_stat("Pricing Region", pricing_region, true)}
                                        {detail_stat("Cost Status", cost_status, true)}
                                        {detail_stat("Cache Read Tokens", log.cache_read_tokens.to_string(), true)}
                                        {detail_stat("Reasoning Tokens", log.reasoning_tokens.to_string(), true)}
                                        {detail_stat("Created At", log.created_at.clone().unwrap_or_else(|| "—".to_string()), true)}
                                    }
                                }

                                div { class: "terminal",
                                    "Prompt/body content is intentionally not present in router_logs, so this page no longer fabricates a prompt snippet."
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
