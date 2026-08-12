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
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let rows = snapshot.and_then(Result::ok).unwrap_or_default();
    let search = query().to_lowercase();
    let filter_value = filter();

    let visible: Vec<FullRouterLog> = rows
        .iter()
        .filter(|log| {
            let status = log.status_label();
            let status_match = filter_value == "all" || status.to_lowercase() == filter_value;
            let text_match = search.is_empty()
                || log.request_id.to_lowercase().contains(&search)
                || log.path.to_lowercase().contains(&search)
                || log.user_id.as_deref().unwrap_or("").to_lowercase().contains(&search)
                || log.model.as_deref().unwrap_or("").to_lowercase().contains(&search)
                || log.upstream_id.as_deref().unwrap_or("").to_lowercase().contains(&search)
                || log.error_type.as_deref().unwrap_or("").to_lowercase().contains(&search);
            status_match && text_match
        })
        .cloned()
        .collect();

    let success = rows.iter().filter(|log| log.status_label() == "Success").count();
    let fallback = rows.iter().filter(|log| log.status_label() == "Fallback").count();
    let failures = rows.len().saturating_sub(success + fallback);
    let avg_latency = if rows.is_empty() {
        0
    } else {
        rows.iter().map(|log| log.latency_ms).sum::<i64>() / rows.len() as i64
    };
    let failure_rate = if rows.is_empty() {
        0.0
    } else {
        failures as f64 * 100.0 / rows.len() as f64
    };
    let failure_rate_text = format!("{failure_rate:.1}% of loaded requests");
    let total_cost = rows.iter().map(FullRouterLog::cost_usd).sum::<f64>();
    let total_cost_text = format!("${total_cost:.4}");
    let visible_count = visible.len();
    let row_count = rows.len();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Logs" }
                    p { class: "page-subtitle", "Find failed, slow, or fallback requests first, then inspect the exact routing and usage metadata behind each request." }
                }
                div { class: "header-actions",
                    div { class: "search-field", style: "width:300px",
                        Icon { name: "search" }
                        input { class: "input", placeholder: "Request, model, upstream, user…", value: "{query}", oninput: move |event| query.set(event.value()) }
                    }
                    select { class: "select", value: "{filter}", onchange: move |event| filter.set(event.value()),
                        option { value: "all", "All outcomes" }
                        option { value: "success", "Success" }
                        option { value: "fallback", "Fallback" }
                        option { value: "timeout", "Timeout" }
                        option { value: "error", "Error" }
                    }
                    button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
                }
            }

            div { class: "metrics",
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Requests Loaded" } span { class: "metric-value", "{row_count}" } span { class: "metric-note", "latest router activity" } }
                    div { class: "metric-icon tone-blue", Icon { name: "logs" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Failures" } span { class: "metric-value", "{failures}" } span { class: "metric-note", "{failure_rate_text}" } }
                    div { class: "metric-icon tone-red", Icon { name: "shield" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Fallbacks" } span { class: "metric-value", "{fallback}" } span { class: "metric-note", "alternate route used" } }
                    div { class: "metric-icon tone-amber", Icon { name: "routes" } }
                }
                div { class: "card metric",
                    div { class: "metric-copy", span { class: "metric-label", "Loaded Cost" } span { class: "metric-value", "{total_cost_text}" } span { class: "metric-note mono", "avg latency {avg_latency}ms" } }
                    div { class: "metric-icon tone-purple", Icon { name: "dollar" } }
                }
            }

            if loading {
                div { class: "card card-pad", "Loading request logs…" }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Request logs could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else {
                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Request activity" }
                            p { "Showing {visible_count} of {row_count} loaded requests. Select a row for routing and cost detail." }
                        }
                    }
                    if visible.is_empty() {
                        div { class: "product-empty", style: "min-height:170px",
                            div { class: "product-empty-inner",
                                h3 { "No requests match this view" }
                                p { "Change the outcome filter or search text. If the environment has no traffic yet, use Playground for a controlled test." }
                            }
                        }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Time" }
                                    th { "Request" }
                                    th { "Model / Upstream" }
                                    th { "Outcome" }
                                    th { class: "right", "Latency" }
                                    th { class: "right", "Tokens" }
                                    th { class: "right", "Cost" }
                                } }
                                tbody {
                                    for log in visible {
                                        {
                                            let status = log.status_label();
                                            let timestamp = log.created_at.clone().unwrap_or_else(|| "-".to_string());
                                            let model = log.model.clone().unwrap_or_else(|| "-".to_string());
                                            let upstream = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                                            let user = log.user_id.clone().unwrap_or_else(|| "anonymous".to_string());
                                            let tokens = log.total_tokens();
                                            let cost = format!("${:.6}", log.cost_usd());
                                            rsx! {
                                                tr { key: "{log.id}-{log.request_id}", style: "cursor:pointer", onclick: move |_| selected.set(Some(log.clone())),
                                                    td { class: "mono muted", "{timestamp}" }
                                                    td {
                                                        div { class: "two-line",
                                                            strong { class: "mono table-primary", "{log.request_id}" }
                                                            small { class: "muted", "{user} • {log.path}" }
                                                        }
                                                    }
                                                    td {
                                                        div { class: "two-line",
                                                            strong { class: "mono", "{model}" }
                                                            small { class: "muted", "{upstream}" }
                                                        }
                                                    }
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

            Drawer {
                title: "Request Detail",
                open: selected().is_some(),
                on_close: move |_| selected.set(None),
                if let Some(log) = selected() {
                    {
                        let status = log.status_label().to_string();
                        let model = log.model.clone().unwrap_or_else(|| "-".to_string());
                        let upstream = log.upstream_id.clone().unwrap_or_else(|| "-".to_string());
                        let user = log.user_id.clone().unwrap_or_else(|| "anonymous".to_string());
                        let decision = log.layer_decision.clone().unwrap_or_else(|| "direct".to_string());
                        let traffic = log.traffic_color.clone().unwrap_or_else(|| "-".to_string());
                        let error_type = log.error_type.clone().unwrap_or_else(|| "none".to_string());
                        let cost_status = log.cost_status.clone().unwrap_or_else(|| "-".to_string());
                        let pricing = log.pricing_region.clone().unwrap_or_else(|| "-".to_string());
                        let total_cost = format!("${:.9}", log.cost_usd());
                        let is_problem = log.status_code >= 400 || log.status_label() != "Success";
                        rsx! {
                            div { class: "stack-lg",
                                div { class: if is_problem { "readiness-strip blocked" } else { "readiness-strip ready" },
                                    span { class: "readiness-dot" }
                                    strong { "{status}" }
                                    span { class: "muted", if is_problem { "Review routing and error metadata below." } else { "Request completed without an observed router error." } }
                                }

                                div { class: "card card-pad stack",
                                    div { class: "product-section-head", div { h3 { "Outcome" } p { "The first facts needed to understand this request." } } }
                                    div { class: "grid-2",
                                        {detail_stat("Request ID", log.request_id.clone())}
                                        {detail_stat("HTTP Status", log.status_code.to_string())}
                                        {detail_stat("Model", model)}
                                        {detail_stat("Upstream", upstream)}
                                        {detail_stat("Latency", format!("{}ms", log.latency_ms))}
                                        {detail_stat("Error Type", error_type)}
                                    }
                                }

                                div { class: "card card-pad stack",
                                    div { class: "product-section-head", div { h3 { "Routing" } p { "How BurnCloud classified and routed the request." } } }
                                    div { class: "grid-2",
                                        {detail_stat("User", user)}
                                        {detail_stat("Path", log.path.clone())}
                                        {detail_stat("Layer Decision", decision)}
                                        {detail_stat("Traffic Color", traffic)}
                                        {detail_stat("Pricing Region", pricing)}
                                        {detail_stat("Cost Status", cost_status)}
                                    }
                                }

                                div { class: "card card-pad stack",
                                    div { class: "product-section-head", div { h3 { "Usage" } p { "Only token types that were actually used are shown." } } }
                                    div { class: "grid-2",
                                        for (label, value) in token_breakdown(&log) {
                                            if value != 0 { {detail_stat(label, value.to_string())} }
                                        }
                                    }
                                    {detail_stat("Total Tokens", log.total_tokens().to_string())}
                                }

                                div { class: "card card-pad stack",
                                    div { class: "product-section-head", div { h3 { "Cost" } p { "Available component costs plus the stored request total." } } }
                                    {detail_stat("Total Cost", total_cost)}
                                    div { class: "grid-2",
                                        for (label, value) in cost_breakdown(&log) {
                                            if value != 0 {
                                                {detail_stat(label, format!("${:.9}", FullRouterLog::cost_component_usd(value)))}
                                            }
                                        }
                                    }
                                }

                                div { class: "product-note", "BurnCloud does not store request prompt/body content in router_logs. This detail view shows only persisted operational metadata instead of fabricating request content." }
                            }
                        }
                    }
                }
            }
        }
    }
}
