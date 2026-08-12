use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::*;

use crate::{
    app::Route,
    components::Icon,
    observability::{full_logs, FullRouterLog},
};

fn compact(n: i64) -> String {
    if n.abs() >= 1_000_000_000 {
        format!("{:.2}B", n as f64 / 1_000_000_000.0)
    } else if n.abs() >= 1_000_000 {
        format!("{:.2}M", n as f64 / 1_000_000.0)
    } else if n.abs() >= 1_000 {
        format!("{:.2}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
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
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let logs: Vec<FullRouterLog> = snapshot.and_then(Result::ok).unwrap_or_default();

    let mut models: BTreeMap<String, ModelEval> = BTreeMap::new();
    for log in &logs {
        let model_name = log.model.clone().unwrap_or_else(|| "unattributed".to_string());
        let entry = models.entry(model_name).or_default();
        entry.requests += 1;
        if log.status_code < 400 {
            entry.success += 1;
        }
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

    let total_requests = logs.len() as i64;
    let successful_requests = logs.iter().filter(|log| log.status_code < 400).count() as i64;
    let total_latency = logs.iter().map(|log| log.latency_ms).sum::<i64>();
    let overall_success = if total_requests == 0 {
        0.0
    } else {
        successful_requests as f64 * 100.0 / total_requests as f64
    };
    let overall_success_text = format!("{overall_success:.1}%");
    let avg_latency = if total_requests == 0 { 0 } else { total_latency / total_requests };
    let avg_latency_text = format!("{avg_latency}ms");
    let model_count = models.len();
    let models_with_errors = models.values().filter(|model| model.success < model.requests).count();
    let single_upstream_models = models.values().filter(|model| model.upstreams.len() <= 1).count();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Evaluation" }
                    p { class: "page-subtitle", "Compare observed model reliability and latency from real routed traffic. This is operational evaluation, not synthetic model-quality scoring." }
                }
                div { class: "header-actions",
                    button { class: "button button-secondary", onclick: move |_| resource.restart(), "Refresh" }
                    Link { class: "button button-secondary", to: Route::Logs {}, "Inspect Logs" }
                }
            }

            if loading {
                div { class: "card card-pad", "Loading operational evaluation…" }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Evaluation data could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else if models.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "chart" } }
                        h3 { "No model traffic to evaluate yet" }
                        p { "Evaluation is built from observed router logs. Send representative requests through Playground or production clients first." }
                        Link { class: "button button-primary", to: Route::Playground {}, "Run a Test Request" }
                    }
                }
            } else {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Observed Requests" } span { class: "metric-value", "{total_requests}" } span { class: "metric-note", "latest evaluation sample" } }
                        div { class: "metric-icon tone-blue", Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Success Rate" } span { class: "metric-value", "{overall_success_text}" } span { class: "metric-note", "HTTP success across sample" } }
                        div { class: "metric-icon tone-green", Icon { name: "shield" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Average Latency" } span { class: "metric-value", "{avg_latency_text}" } span { class: "metric-note", "end-to-end observed" } }
                        div { class: "metric-icon tone-purple", Icon { name: "chart" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Models With Errors" } span { class: "metric-value", "{models_with_errors}" } span { class: "metric-note", "of {model_count} observed models" } }
                        div { class: "metric-icon tone-amber", Icon { name: "models" } }
                    }
                }

                if models_with_errors > 0 || single_upstream_models > 0 {
                    div { class: "card card-pad stack",
                        div { class: "product-section-head",
                            div {
                                h3 { "Operational attention" }
                                p { "These signals do not prove model quality, but they indicate where routing or upstream reliability deserves investigation." }
                            }
                        }
                        if models_with_errors > 0 {
                            div { class: "readiness-strip blocked",
                                span { class: "readiness-dot" }
                                strong { "{models_with_errors} models have at least one failed request in the loaded sample." }
                                Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Inspect failures" }
                            }
                        }
                        if single_upstream_models > 0 {
                            div { class: "readiness-strip blocked",
                                span { class: "readiness-dot" }
                                strong { "{single_upstream_models} observed models were served by one or fewer upstreams in this sample." }
                                Link { class: "button button-ghost button-sm", to: Route::Models {}, "Review redundancy" }
                            }
                        }
                    }
                }

                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Model performance" }
                            p { "Use this table to identify failures, slow models, high-cost models, and single-upstream observations." }
                        }
                    }
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr {
                                th { "Model" }
                                th { "Health" }
                                th { class: "right", "Requests" }
                                th { class: "right", "Success" }
                                th { class: "right", "Avg Latency" }
                                th { class: "right", "Tokens" }
                                th { class: "right", "Multimodal" }
                                th { class: "right", "Cost" }
                                th { "Upstreams" }
                            } }
                            tbody {
                                for (model_name, evaluation) in models {
                                    {
                                        let failures = evaluation.requests - evaluation.success;
                                        let success_rate = if evaluation.requests == 0 {
                                            0.0
                                        } else {
                                            evaluation.success as f64 * 100.0 / evaluation.requests as f64
                                        };
                                        let success_text = format!("{success_rate:.1}%");
                                        let avg_latency = if evaluation.requests == 0 { 0 } else { evaluation.latency_sum / evaluation.requests };
                                        let latency_text = format!("{avg_latency}ms");
                                        let token_text = compact(evaluation.tokens);
                                        let cost_text = format!("${:.6}", evaluation.cost);
                                        let upstream_count = evaluation.upstreams.len();
                                        let upstream_text = evaluation.upstreams.into_iter().collect::<Vec<_>>().join(", ");
                                        let (health_class, health_text) = if failures > 0 {
                                            ("badge badge-error", "Errors observed")
                                        } else if upstream_count <= 1 {
                                            ("badge badge-warning", "Single upstream")
                                        } else {
                                            ("badge badge-success", "Stable sample")
                                        };
                                        rsx! {
                                            tr { key: "{model_name}",
                                                td { class: "table-primary mono", "{model_name}" }
                                                td { span { class: "{health_class}", "{health_text}" } }
                                                td { class: "right tabular", "{evaluation.requests}" }
                                                td { class: "right strong tabular", "{success_text}" }
                                                td { class: "right tabular", "{latency_text}" }
                                                td { class: "right tabular", "{token_text}" }
                                                td { class: "right tabular", "{evaluation.multimodal_requests}" }
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

                div { class: "product-note", "Evaluation uses observed production/request-log data. Accuracy, hallucination rate, task quality, and benchmark scores require a dedicated evaluation backend and are intentionally not inferred here." }
            }
        }
    }
}
