use dioxus::prelude::*;

use crate::{
    app::Route,
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
        div { class: "logs-detail-stat",
            span { class: "tiny subtle", "{label}" }
            div { class: "small strong mono", "{value}" }
        }
    }
}

fn format_latency(latency_ms: i64) -> String {
    if latency_ms >= 1_000 {
        format!("{:.2}s", latency_ms as f64 / 1_000.0)
    } else {
        format!("{latency_ms}ms")
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
    let load_error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let rows = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
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
    let avg_latency_text = format_latency(avg_latency);
    let failure_rate = if rows.is_empty() {
        0.0
    } else {
        failures as f64 * 100.0 / rows.len() as f64
    };
    let failure_rate_text = format!("{failure_rate:.1}% of loaded requests");
    let fallback_rate = if rows.is_empty() {
        0.0
    } else {
        fallback as f64 * 100.0 / rows.len() as f64
    };
    let fallback_rate_text = format!("{fallback_rate:.1}% used fallback");
    let total_cost = rows.iter().map(FullRouterLog::cost_usd).sum::<f64>();
    let total_cost_text = format!("${total_cost:.4}");
    let visible_count = visible.len();
    let row_count = rows.len();
    let success_rate = if rows.is_empty() {
        0.0
    } else {
        success as f64 * 100.0 / rows.len() as f64
    };
    let success_rate_text = format!("{success_rate:.1}% success");
    let health_class = if failures > 0 {
        "readiness-strip logs-health-error logs-health-strip"
    } else if fallback > 0 {
        "readiness-strip blocked logs-health-strip"
    } else {
        "readiness-strip ready logs-health-strip"
    };
    let health_title = if failures > 0 {
        "Recent request failures need attention"
    } else if fallback > 0 {
        "Traffic is succeeding, but fallback routes are active"
    } else {
        "Loaded request activity is healthy"
    };
    let health_copy = if failures > 0 {
        format!("{failures} of {row_count} loaded requests ended in Error or Timeout. Filter failures first, then inspect routing and error metadata.")
    } else if fallback > 0 {
        format!("{fallback} of {row_count} loaded requests used fallback routing. Requests completed, but primary-route resilience should be reviewed.")
    } else {
        format!("All {row_count} loaded requests completed successfully without an observed fallback, timeout, or router error.")
    };

    rsx! {
        div { class: "page",
            div { class: "page-header logs-page-header",
                div {
                    h2 { class: "page-title", "Logs" }
                    p { class: "page-subtitle", "Find failed, slow, or fallback requests first, then inspect the exact routing and usage metadata behind each request." }
                }
                div { class: "header-actions logs-toolbar",
                    div { class: "search-field logs-search-field",
                        Icon { name: "search" }
                        input {
                            class: "input",
                            placeholder: "Request, model, upstream, user…",
                            value: "{query}",
                            disabled: loading,
                            oninput: move |event| query.set(event.value())
                        }
                    }
                    select {
                        class: "select logs-outcome-filter",
                        value: "{filter}",
                        disabled: loading,
                        onchange: move |event| filter.set(event.value()),
                        option { value: "all", "All outcomes" }
                        option { value: "success", "Success" }
                        option { value: "fallback", "Fallback" }
                        option { value: "timeout", "Timeout" }
                        option { value: "error", "Error" }
                    }
                    button {
                        class: "button button-secondary",
                        disabled: loading,
                        onclick: move |_| resource.restart(),
                        if loading { "Refreshing…" } else { "Refresh" }
                    }
                }
            }

            if loading {
                div { class: "card product-empty logs-loading-state",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "logs" } }
                        h3 { "Loading request activity" }
                        p { "Reading recent router outcomes, upstream decisions, latency, usage, and cost before showing operational conclusions." }
                    }
                }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Request logs could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Failures" } span { class: "metric-value", "{failures}" } span { class: "metric-note", "{failure_rate_text}" } }
                        div { class: if failures > 0 { "metric-icon tone-red" } else { "metric-icon tone-gray" }, Icon { name: "shield" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Fallbacks" } span { class: "metric-value", "{fallback}" } span { class: "metric-note", "{fallback_rate_text}" } }
                        div { class: if fallback > 0 { "metric-icon tone-amber" } else { "metric-icon tone-gray" }, Icon { name: "routes" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Avg Latency" } span { class: "metric-value", "{avg_latency_text}" } span { class: "metric-note", "across loaded requests" } }
                        div { class: "metric-icon tone-gray", Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Loaded Cost" } span { class: "metric-value", "{total_cost_text}" } span { class: "metric-note", "{row_count} request records" } }
                        div { class: "metric-icon tone-gray", Icon { name: "dollar" } }
                    }
                }

                if !rows.is_empty() {
                    div { class: "{health_class}",
                        span { class: "readiness-dot" }
                        div { class: "logs-health-copy",
                            strong { "{health_title}" }
                            span { class: "small muted", "{health_copy}" }
                        }
                        span { class: "badge badge-neutral logs-health-meta", "{success_rate_text}" }
                    }
                }

                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Request activity" }
                            p { "Showing {visible_count} of {row_count} loaded requests. Select a row for routing, usage, and cost detail." }
                        }
                    }
                    if rows.is_empty() {
                        div { class: "product-empty logs-empty-state",
                            div { class: "product-empty-inner",
                                div { class: "product-empty-icon", Icon { name: "logs" } }
                                h3 { "No request activity yet" }
                                p { "No router request records are loaded for this environment. Run a controlled end-to-end test in Playground to create the first traceable request." }
                                Link { class: "button button-primary", to: Route::Playground {}, "Open Playground" }
                            }
                        }
                    } else if visible.is_empty() {
                        div { class: "product-empty logs-empty-state",
                            div { class: "product-empty-inner",
                                h3 { "No requests match this view" }
                                p { "Change the outcome filter or search text to return to the loaded request activity." }
                                button {
                                    class: "button button-secondary",
                                    onclick: move |_| {
                                        query.set(String::new());
                                        filter.set("all".to_string());
                                    },
                                    "Clear filters"
                                }
                            }
                        }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table logs-table",
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
                                            let latency = format_latency(log.latency_ms);
                                            let selected_log = log.clone();
                                            rsx! {
                                                tr { key: "{log.id}-{log.request_id}", class: "logs-row", onclick: move |_| selected.set(Some(selected_log.clone())),
                                                    td { class: "mono muted logs-time", title: "{timestamp}", "{timestamp}" }
                                                    td {
                                                        div { class: "two-line logs-request-cell",
                                                            strong { class: "mono table-primary", title: "{log.request_id}", "{log.request_id}" }
                                                            small { class: "muted", title: "{user} • {log.path}", "{user} • {log.path}" }
                                                        }
                                                    }
                                                    td {
                                                        div { class: "two-line logs-route-cell",
                                                            strong { class: "mono", title: "{model}", "{model}" }
                                                            small { class: "muted", title: "{upstream}", "{upstream}" }
                                                        }
                                                    }
                                                    td { Badge { text: status, tone: status_tone(status) } }
                                                    td { class: "right tabular", "{latency}" }
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
                        let detail_status_class = if status == "Error" || status == "Timeout" {
                            "readiness-strip logs-health-error logs-detail-status"
                        } else if status == "Fallback" {
                            "readiness-strip blocked logs-detail-status"
                        } else {
                            "readiness-strip ready logs-detail-status"
                        };
                        let detail_status_copy = if status == "Error" || status == "Timeout" {
                            "The request did not complete normally. Start with Outcome, then inspect routing and error metadata."
                        } else if status == "Fallback" {
                            "The request completed through a fallback route. Inspect the routing decision and upstream used."
                        } else {
                            "The request completed without an observed router error or fallback."
                        };
                        rsx! {
                            div { class: "stack-lg",
                                div { class: "{detail_status_class}",
                                    span { class: "readiness-dot" }
                                    div { class: "logs-health-copy",
                                        strong { "{status}" }
                                        span { class: "small muted", "{detail_status_copy}" }
                                    }
                                }

                                div { class: "card card-pad stack",
                                    div { class: "product-section-head", div { h3 { "Outcome" } p { "The first facts needed to understand this request." } } }
                                    div { class: "grid-2 logs-detail-grid",
                                        {detail_stat("Request ID", log.request_id.clone())}
                                        {detail_stat("HTTP Status", log.status_code.to_string())}
                                        {detail_stat("Model", model)}
                                        {detail_stat("Upstream", upstream)}
                                        {detail_stat("Latency", format_latency(log.latency_ms))}
                                        {detail_stat("Error Type", error_type)}
                                    }
                                }

                                div { class: "card card-pad stack",
                                    div { class: "product-section-head", div { h3 { "Routing" } p { "How BurnCloud classified and routed the request." } } }
                                    div { class: "grid-2 logs-detail-grid",
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
                                    div { class: "grid-2 logs-detail-grid",
                                        for (label, value) in token_breakdown(&log) {
                                            if value != 0 { {detail_stat(label, value.to_string())} }
                                        }
                                    }
                                    {detail_stat("Total Tokens", log.total_tokens().to_string())}
                                }

                                div { class: "card card-pad stack",
                                    div { class: "product-section-head", div { h3 { "Cost" } p { "Available component costs plus the stored request total." } } }
                                    {detail_stat("Total Cost", total_cost)}
                                    div { class: "grid-2 logs-detail-grid",
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
