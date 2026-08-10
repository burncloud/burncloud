use dioxus::prelude::*;

use crate::{
    components::{Badge, Drawer, Icon},
    data::{LogRow, LOGS},
};

fn page_header(title: &str, subtitle: &str, actions: Element) -> Element {
    rsx! {
        div { class: "page-header",
            div {
                h2 { class: "page-title", "{title}" }
                p { class: "page-subtitle", "{subtitle}" }
            }
            div { class: "header-actions", {actions} }
        }
    }
}

fn status_tone(status: &str) -> &'static str {
    match status {
        "Success" => "success",
        "Fallback" => "warning",
        "Timeout" => "error",
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
    let mut selected = use_signal(|| None::<LogRow>);
    let mut query = use_signal(String::new);
    let query_value = query().to_lowercase();
    let visible_logs: Vec<LogRow> = LOGS
        .iter()
        .copied()
        .filter(|log| {
            query_value.is_empty()
                || log.request_id.to_lowercase().contains(&query_value)
                || log.customer.to_lowercase().contains(&query_value)
                || log.route.to_lowercase().contains(&query_value)
                || log.model.to_lowercase().contains(&query_value)
        })
        .collect();

    rsx! {
        div { class: "page",
            {page_header(
                "Logs",
                "Detailed observability into every routed request.",
                rsx! {
                    div { class: "search-field", style: "width:288px",
                        Icon { name: "search" }
                        input {
                            class: "input",
                            placeholder: "Search by request ID, customer, route...",
                            value: "{query}",
                            oninput: move |evt| query.set(evt.value()),
                        }
                    }
                    button { class: "button button-secondary", Icon { name: "settings" } "Filter" }
                },
            )}

            div { class: "card table-card",
                div { class: "table-wrap",
                    table { class: "data-table",
                        thead { tr {
                            th { "Timestamp" }
                            th { "Request ID" }
                            th { "Customer" }
                            th { "Route / Model" }
                            th { "Status" }
                            th { class: "right", "Latency" }
                            th { class: "right", "Tokens" }
                            th { class: "right", "Cost" }
                        } }
                        tbody {
                            for log in visible_logs {
                                tr {
                                    onclick: move |_| selected.set(Some(log)),
                                    style: "cursor:pointer",
                                    td { class: "mono muted", "{log.timestamp}" }
                                    td { class: "mono table-primary", "{log.request_id}" }
                                    td { "{log.customer}" }
                                    td {
                                        div { class: "two-line",
                                            span { class: "table-primary", "{log.route}" }
                                            small { "{log.model} • {log.provider}" }
                                        }
                                    }
                                    td { Badge { text: log.status, tone: status_tone(log.status) } }
                                    td { class: "right muted tabular", "{log.latency}ms" }
                                    td { class: "right muted tabular", "{log.tokens}" }
                                    td { class: "right strong tabular", "${log.cost:.3}" }
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
                    div { class: "stack-lg",
                        div { class: "card card-pad", style: "display:grid;grid-template-columns:repeat(3,1fr);gap:16px",
                            {detail_stat("Request ID", log.request_id.to_string(), true)}
                            {detail_stat("Customer", log.customer.to_string(), false)}
                            {detail_stat("Total Cost", format!("${:.3}", log.cost), false)}
                        }

                        div { class: "stack",
                            h3 { class: "section-label", "Routing Timeline" }
                            {timeline_step("neutral", format!("Request received • {}", log.timestamp))}
                            {timeline_step("blue", format!("Matched route: {}", log.route))}
                            {timeline_step(
                                "blue",
                                format!(
                                    "Selected primary model: {}",
                                    if log.status == "Success" { log.model } else { "claude-fable-5" }
                                ),
                            )}
                            if log.status == "Timeout" {
                                {timeline_step("red", "Timeout after 10s".to_string())}
                                {timeline_step("amber", "Triggered fallback condition: Timeout > 8s".to_string())}
                                {timeline_step("blue", format!("Retried through fallback path toward {}", log.model))}
                            }
                            if log.status == "Fallback" {
                                {timeline_step("amber", "Provider error rate exceeded threshold".to_string())}
                                {timeline_step("blue", format!("Falling back to {} via {}", log.model, log.provider))}
                            }
                            {timeline_step(
                                if log.status == "Timeout" { "red" } else { "green" },
                                format!("Response completed in {}ms", log.latency),
                            )}
                        }

                        div { class: "stack",
                            h3 { class: "section-label", "Prompt Snippet" }
                            pre { class: "terminal", style: "white-space:pre-wrap",
                                "\"system\": \"You are a senior legal...\"\n\n\"user\": \"Summarize the following contract and...\""
                            }
                        }
                    }
                }
            }
        }
    }
}
