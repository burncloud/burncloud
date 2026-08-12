use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::*;

use crate::observability::{full_logs, FullRouterLog};

fn compact(n: i64) -> String {
    if n.abs() >= 1_000_000_000 { format!("{:.2}B", n as f64 / 1_000_000_000.0) }
    else if n.abs() >= 1_000_000 { format!("{:.2}M", n as f64 / 1_000_000.0) }
    else if n.abs() >= 1_000 { format!("{:.2}K", n as f64 / 1_000.0) }
    else { n.to_string() }
}

#[derive(Default, Clone)]
struct ModelEval {
    requests: i64,
    success: i64,
    latency_sum: i64,
    tokens: i64,
    cost: f64,
    upstreams: BTreeSet<String>,
    multimodal_requests: i64,
}

#[component]
pub fn Evaluation() -> Element {
    let mut resource = use_resource(move || async move { full_logs(500).await });
    let snapshot = resource.read().clone();
    let loading = snapshot.is_none();
    let load_error = snapshot.as_ref().and_then(|r| r.as_ref().err().cloned());
    let logs: Vec<FullRouterLog> = snapshot.and_then(Result::ok).unwrap_or_default();

    let mut models: BTreeMap<String, ModelEval> = BTreeMap::new();
    for log in &logs {
        let model_name = log.model.clone().unwrap_or_else(|| "unattributed".to_string());
        let entry = models.entry(model_name).or_default();
        entry.requests += 1;
        if log.status_code < 400 { entry.success += 1; }
        entry.latency_sum += log.latency_ms;
        entry.tokens += log.total_tokens();
        entry.cost += log.cost_usd();
        if log.video_tokens != 0
            || log.audio_input_tokens != 0
            || log.audio_output_tokens != 0
            || log.image_tokens != 0
            || log.embedding_tokens != 0
        {
            entry.multimodal_requests += 1;
        }
        if let Some(upstream) = &log.upstream_id {
            entry.upstreams.insert(upstream.clone());
        }
    }

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Evaluation" }
                    p { class: "page-subtitle", "Operational evaluation derived from the full current router log schema, including multimodal token types." }
                }
                button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
            }

            if loading {
                div { class: "card card-pad", "Loading operational evaluation…" }
            } else if let Some(message) = load_error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else {
                div { class: "card table-card",
                    if models.is_empty() {
                        div { class: "card-pad small muted", "No router-log model data is available yet." }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Model" }
                                    th { class: "right", "Requests" }
                                    th { class: "right", "Success" }
                                    th { class: "right", "Avg Latency" }
                                    th { class: "right", "All Tokens" }
                                    th { class: "right", "Multimodal" }
                                    th { class: "right", "Cost" }
                                    th { "Observed Upstreams" }
                                } }
                                tbody {
                                    for (model, eval) in models {
                                        {
                                            let success_rate = if eval.requests == 0 { 0.0 } else { eval.success as f64 * 100.0 / eval.requests as f64 };
                                            let success_text = format!("{success_rate:.1}%");
                                            let avg_latency = if eval.requests == 0 { 0 } else { eval.latency_sum / eval.requests };
                                            let latency_text = format!("{avg_latency}ms");
                                            let token_text = compact(eval.tokens);
                                            let cost_text = format!("${:.6}", eval.cost);
                                            let upstream_text = eval.upstreams.into_iter().collect::<Vec<_>>().join(", ");
                                            rsx! {
                                                tr { key: "{model}",
                                                    td { class: "table-primary mono", "{model}" }
                                                    td { class: "right tabular", "{eval.requests}" }
                                                    td { class: "right strong tabular", "{success_text}" }
                                                    td { class: "right tabular", "{latency_text}" }
                                                    td { class: "right tabular", "{token_text}" }
                                                    td { class: "right tabular", "{eval.multimodal_requests}" }
                                                    td { class: "right tabular", "{cost_text}" }
                                                    td { class: "muted", "{upstream_text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "card card-pad tiny subtle", "Evaluation is production-observability based. Quality/accuracy benchmarks still require a dedicated evaluation backend, which BurnCloud does not currently expose." }
            }
        }
    }
}
