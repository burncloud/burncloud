use dioxus::prelude::*;

use crate::{
    components::{Badge, Drawer, Icon},
    observability::{full_logs, FullRouterLog},
};

fn status_tone(status: &str) -> &'static str {
    match status {
        "Success" => "success",
        "Fallback" => "warning",
        "Timeout" | "Error" => "error",
        _ => "neutral",
    }
}

fn detail_stat(label: &str, value: String) -> Element {
    rsx! {
        div {
            span { class: "tiny subtle", "{label}" }
            div { class: "small strong mono", "{value}" }
        }
    }
}

fn token_breakdown(log: &FullRouterLog) -> Vec<(&'static str, i32)> {
    vec![
        ("Prompt", log.prompt_tokens),
        ("Completion", log.completion_tokens),
        ("Cache Read", log.cache_read_tokens),
        ("Cache Write", log.cache_write_tokens),
        ("Reasoning", log.reasoning_tokens),
        ("Video", log.video_tokens),
        ("Audio Input", log.audio_input_tokens),
        ("Audio Output", log.audio_output_tokens),
        ("Image", log.image_tokens),
        ("Embedding", log.embedding_tokens),
    ]
}

fn cost_breakdown(log: &FullRouterLog) -> Vec<(&'static str, i64)> {
    vec![
        ("Input", log.input_cost),
        ("Output", log.output_cost),
        ("Cache Read", log.cache_read_cost),
        ("Cache Write", log.cache_write_cost),
        ("Audio", log.audio_cost),
        ("Image", log.image_cost),
        ("Video", log.video_cost),
        ("Reasoning", log.reasoning_cost),
        ("Embedding", log.embedding_cost),
    ]
}

#[component]
pub fn Logs() -> Element {
    let mut selected = use_signal(|| None::<FullRouterLog>);
    let mut query = use_signal(String::new);
    let mut filter = use_signal(|| "all".to_string());
    let mut resource = use_resource(move || async move { full_logs(200).await });

    let snapshot = resource.read().clone();
    let loading = snapshot.is_none();
    let load_error = snapshot.as_ref().and_then(|r| r.as_ref().err().cloned());
    let rows = snapshot.and_then(Result::ok).unwrap_or_default();
    let q = query().to_lowercase();
    let filter_value = filter();

    let visible: Vec<FullRouterLog> = rows
        .iter()
        .filter(|log| {
            let status = log.status_label();
            let status_match = filter_value == "all" || status.to_lowercase() == filter_value;
            let text_match = q.is_empty()
                || log.request_id.to_lowercase().contains(&q)
                || log.path.to_lowercase().contains(&q)
                || log.user_id.as_deref().unwrap_or("").to_lowercase().contains(&q)
                || log.model.as_deref().unwrap_or("").to_lowercase().contains(&q)
                || log.upstream_id.as_deref().unwrap_or("").to_lowercase().contains(&q)
                || log.error_type.as_deref().unwrap_or("").to_lowercase().contains(&q);
            status_match && text_match
        })
        .cloned()
        .collect();

    let success = rows.iter().filter(|r| r.status_label() == "Success").count();
    let fallback = rows.iter().filter(|r| r.status_label() == "Fallback").count();
    let failures = rows.len().saturating_sub(success + fallback);
    let avg_latency = if rows.is_empty() { 0 } else { rows.iter().map(|r| r.latency_ms).sum::<i64>() / rows.len() as i64 };
    let token_total = rows.iter().map(FullRouterLog::total_tokens).sum::<i64>();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Logs" }
                    p { class: "page-subtitle", "Full router_logs schema including multimodal token and per-type cost fields." }
                }
                div { class: "header-actions",
                    div { class: "search-field", style: "width:300px",
                        Icon { name: "search" }
                        input { class: "input", placeholder: "Request, user, model, upstream, error…", value: "{query}", oninput: move |e| query.set(e.value()) }
                    }
                    select { class: "select", value: "{filter}", onchange: move |e| filter.set(e.value()),
                        option { value: "all", "All statuses" }
                        option { value: "success", "Success" }
                        option { value: "fallback", "Fallback" }
                        option { value: "timeout", "Timeout" }
                        option { value: "error", "Error" }
                    }
                    button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
                }
            }

            div { class: "metrics",
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Loaded Requests" } span { class: "metric-value", "{rows.len()}" } } div { class: "metric-icon tone-blue", Icon { name: "logs" } } }
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Success" } span { class: "metric-value", "{success}" } } div { class: "metric-icon tone-green", Icon { name: "shield" } } }
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Fallback / Failures" } span { class: "metric-value", "{fallback} / {failures}" } } div { class: "metric-icon tone-amber", Icon { name: "routes" } } }
                div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "All Token Types" } span { class: "metric-value", "{token_total}" } span { class: "metric-note mono", "avg latency {avg_latency}ms" } } div { class: "metric-icon tone-purple", Icon { name: "models" } } }
            }

            if loading {
                div { class: "card card-pad", "Loading router logs…" }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack", strong { class: "danger", "Unable to load router logs" } code { class: "terminal", "{message}" } button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" } }
            } else {
                div { class: "card table-card",
                    if visible.is_empty() {
                        div { class: "card-pad small muted", "No log rows match this search/filter." }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr { th { "Timestamp" } th { "Request ID" } th { "User" } th { "Path / Model" } th { "Route" } th { "Status" } th { class: "right", "Latency" } th { class: "right", "All Tokens" } th { class: "right", "Cost" } } }
                                tbody {
                                    for log in visible {
                                        {
                                            let status = log.status_label();
                                            let timestamp = log.created_at.clone().unwrap_or_else(|| "-".to_string());
                                            let user = log.user_id.clone().unwrap_or_else(|| "anonymous".to_string());
                                            let model = log.model.clone().unwrap_or_else(|| "-".to_string());
                                            let upstream = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                                            let decision = log.layer_decision.clone().unwrap_or_else(|| "direct".to_string());
                                            let tokens = log.total_tokens();
                                            let cost = format!("${:.6}", log.cost_usd());
                                            rsx! {
                                                tr { key: "{log.id}-{log.request_id}", style: "cursor:pointer", onclick: move |_| selected.set(Some(log.clone())),
                                                    td { class: "mono muted", "{timestamp}" }
                                                    td { class: "mono table-primary", "{log.request_id}" }
                                                    td { "{user}" }
                                                    td { div { class: "two-line", span { class: "table-primary mono", "{log.path}" } small { "{model} • {upstream}" } } }
                                                    td { class: "mono muted", "{decision}" }
                                                    td { Badge { text: status, tone: status_tone(status) } }
                                                    td { class: "right tabular", "{log.latency_ms}ms" }
                                                    td { class: "right tabular", "{tokens}" }
                                                    td { class: "right strong tabular", "{cost}" }
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

            Drawer { title: "Request Detail", open: selected().is_some(), on_close: move |_| selected.set(None),
                if let Some(log) = selected() {
                    {
                        let status = log.status_label().to_string();
                        let model = log.model.clone().unwrap_or_else(|| "-".to_string());
                        let upstream = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                        let decision = log.layer_decision.clone().unwrap_or_else(|| "-".to_string());
                        let traffic = log.traffic_color.clone().unwrap_or_else(|| "-".to_string());
                        let error_type = log.error_type.clone().unwrap_or_else(|| "none".to_string());
                        let cost_status = log.cost_status.clone().unwrap_or_else(|| "-".to_string());
                        let pricing = log.pricing_region.clone().unwrap_or_else(|| "-".to_string());
                        let total_cost = format!("${:.9}", log.cost_usd());
                        rsx! {
                            div { class: "stack-lg",
                                div { class: "card card-pad", style: "display:grid;grid-template-columns:repeat(3,1fr);gap:16px",
                                    {detail_stat("Request ID", log.request_id.clone())}
                                    {detail_stat("Status", status)}
                                    {detail_stat("Total Cost", total_cost)}
                                    {detail_stat("Latency", format!("{}ms", log.latency_ms))}
                                    {detail_stat("Model", model)}
                                    {detail_stat("Upstream", upstream)}
                                }
                                div { class: "card card-pad stack",
                                    h3 { class: "section-label", "Routing / Billing Metadata" }
                                    div { class: "grid-2",
                                        {detail_stat("Layer Decision", decision)}
                                        {detail_stat("Traffic Color", traffic)}
                                        {detail_stat("Error Type", error_type)}
                                        {detail_stat("Cost Status", cost_status)}
                                        {detail_stat("Pricing Region", pricing)}
                                        {detail_stat("HTTP Status", log.status_code.to_string())}
                                    }
                                }
                                div { class: "card card-pad stack",
                                    h3 { class: "section-label", "Token Breakdown" }
                                    div { class: "grid-2",
                                        for (label, value) in token_breakdown(&log) {
                                            if value != 0 { {detail_stat(label, value.to_string())} }
                                        }
                                    }
                                    {detail_stat("Total Across Token Types", log.total_tokens().to_string())}
                                }
                                div { class: "card card-pad stack",
                                    h3 { class: "section-label", "Cost Breakdown" }
                                    div { class: "grid-2",
                                        for (label, value) in cost_breakdown(&log) {
                                            if value != 0 { {detail_stat(label, format!("${:.9}", FullRouterLog::cost_component_usd(value)))} }
                                        }
                                    }
                                }
                                div { class: "terminal", "Request bodies/prompts are not stored in router_logs, so the UI intentionally does not fabricate them." }
                            }
                        }
                    }
                }
            }
        }
    }
}
