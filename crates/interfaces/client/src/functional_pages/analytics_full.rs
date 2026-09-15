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

fn format_latency(latency_ms: i64) -> String {
    if latency_ms >= 1_000 {
        format!("{:.2}s", latency_ms as f64 / 1_000.0)
    } else {
        format!("{latency_ms}ms")
    }
}

#[derive(Default, Clone)]
struct ModelEval {
    requests: i64,
    success: i64,
    fallbacks: i64,
    latency_sum: i64,
    tokens: i64,
    cost: f64,
    upstreams: BTreeSet<String>,
    multimodal_requests: i64,
}

impl ModelEval {
    fn failures(&self) -> i64 {
        self.requests.saturating_sub(self.success)
    }

    fn avg_latency(&self) -> i64 {
        if self.requests == 0 {
            0
        } else {
            self.latency_sum / self.requests
        }
    }
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
        let outcome = log.status_label();
        entry.requests += 1;
        if outcome == "Success" || outcome == "Fallback" {
            entry.success += 1;
        }
        if outcome == "Fallback" {
            entry.fallbacks += 1;
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
    let successful_requests = logs
        .iter()
        .filter(|log| matches!(log.status_label(), "Success" | "Fallback"))
        .count() as i64;
    let fallback_requests = logs.iter().filter(|log| log.status_label() == "Fallback").count() as i64;
    let total_latency = logs.iter().map(|log| log.latency_ms).sum::<i64>();
    let overall_success = if total_requests == 0 {
        0.0
    } else {
        successful_requests as f64 * 100.0 / total_requests as f64
    };
    let overall_success_text = format!("{overall_success:.1}%");
    let avg_latency = if total_requests == 0 { 0 } else { total_latency / total_requests };
    let avg_latency_text = format_latency(avg_latency);
    let model_count = models.len();
    let models_with_errors = models.values().filter(|model| model.failures() > 0).count();
    let models_with_fallbacks = models.values().filter(|model| model.fallbacks > 0).count();
    let one_or_fewer_observed_upstreams = models.values().filter(|model| model.upstreams.len() <= 1).count();

    let sample_class = if models_with_errors > 0 {
        "readiness-strip danger-zone"
    } else if fallback_requests > 0 {
        "readiness-strip blocked"
    } else {
        "readiness-strip ready"
    };
    let (sample_badge_class, sample_badge_text, sample_title, sample_copy) = if models_with_errors > 0 {
        (
            "badge badge-error",
            "FAILURES OBSERVED",
            "The loaded traffic sample contains model request failures",
            format!("{models_with_errors} of {model_count} observed models have at least one failed request. Start with those rows, then inspect the underlying requests in Logs."),
        )
    } else if fallback_requests > 0 {
        (
            "badge badge-warning",
            "FALLBACK OBSERVED",
            "The loaded sample succeeded, but fallback routing was used",
            format!("{fallback_requests} of {total_requests} observed requests completed through fallback routing. This is operational evidence about the sample, not a model-quality score."),
        )
    } else {
        (
            "badge badge-success",
            "STABLE SAMPLE",
            "No request failures or fallbacks were observed in the loaded sample",
            format!("{total_requests} observed requests across {model_count} models completed without a router failure or fallback in this sample."),
        )
    };

    let mut model_rows = models.into_iter().collect::<Vec<_>>();
    model_rows.sort_by(|(left_name, left), (right_name, right)| {
        right
            .failures()
            .cmp(&left.failures())
            .then_with(|| right.fallbacks.cmp(&left.fallbacks))
            .then_with(|| right.avg_latency().cmp(&left.avg_latency()))
            .then_with(|| left_name.cmp(right_name))
    });

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Evaluation" }
                    p { class: "page-subtitle", "Compare observed reliability, fallback behavior, latency, usage, and cost from real routed traffic. This is operational evidence, not synthetic model-quality scoring." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        disabled: loading,
                        onclick: move |_| resource.restart(),
                        if loading { "Refreshing…" } else { "Refresh" }
                    }
                    Link { class: "button button-secondary", to: Route::Logs {}, "Inspect Logs" }
                }
            }

            if loading {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "chart" } }
                        h3 { "Building operational evaluation" }
                        p { "Reading up to 500 recent router-log records before calculating reliability, fallback, latency, usage, and cost observations." }
                    }
                }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Evaluation data could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| resource.restart(), "Retry" }
                }
            } else if model_rows.is_empty() {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "chart" } }
                        h3 { "No model traffic to evaluate yet" }
                        p { "Evaluation is built only from observed router logs. Send representative requests through Playground or production clients first." }
                        Link { class: "button button-primary", to: Route::Playground {}, "Run a Test Request" }
                    }
                }
            } else {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Observed Requests" } span { class: "metric-value", "{total_requests}" } span { class: "metric-note", "up to 500 latest records" } }
                        div { class: "metric-icon tone-gray", Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Success Rate" } span { class: "metric-value", "{overall_success_text}" } span { class: "metric-note", "Success + Fallback outcomes" } }
                        div { class: if models_with_errors == 0 { "metric-icon tone-green" } else { "metric-icon tone-red" }, Icon { name: "shield" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Average Latency" } span { class: "metric-value", "{avg_latency_text}" } span { class: "metric-note", "end-to-end observed" } }
                        div { class: "metric-icon tone-gray", Icon { name: "chart" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Models With Errors" } span { class: "metric-value", "{models_with_errors}" } span { class: "metric-note", "of {model_count} observed models" } }
                        div { class: if models_with_errors > 0 { "metric-icon tone-red" } else { "metric-icon tone-gray" }, Icon { name: "models" } }
                    }
                }

                div { class: "{sample_class}",
                    span { class: "{sample_badge_class}", "{sample_badge_text}" }
                    div { class: "stack", style: "gap:2px;flex:1;min-width:0",
                        strong { "{sample_title}" }
                        span { class: "small muted", "{sample_copy}" }
                    }
                }

                if models_with_errors > 0 || models_with_fallbacks > 0 || one_or_fewer_observed_upstreams > 0 {
                    div { class: "card card-pad stack",
                        div { class: "product-section-head",
                            div {
                                h3 { "Operational attention" }
                                p { "These are facts about the loaded traffic sample. Configuration redundancy must be verified separately in Models and Routes." }
                            }
                        }
                        if models_with_errors > 0 {
                            div { class: "readiness-strip danger-zone",
                                span { class: "badge badge-error", "FAILURES" }
                                strong { "{models_with_errors} models have at least one failed request in the loaded sample." }
                                Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Inspect failures" }
                            }
                        }
                        if models_with_fallbacks > 0 {
                            div { class: "readiness-strip blocked",
                                span { class: "badge badge-warning", "FALLBACK" }
                                strong { "{models_with_fallbacks} models used fallback routing at least once in the loaded sample." }
                                Link { class: "button button-ghost button-sm", to: Route::Logs {}, "Inspect routing" }
                            }
                        }
                        if one_or_fewer_observed_upstreams > 0 {
                            div { class: "readiness-strip",
                                span { class: "badge badge-neutral", "SAMPLE ONLY" }
                                strong { "{one_or_fewer_observed_upstreams} models were observed on one or fewer upstream IDs in this sample; this does not prove configured single-upstream routing." }
                                Link { class: "button button-ghost button-sm", to: Route::Models {}, "Check configured resilience" }
                            }
                        }
                    }
                }

                div { class: "card table-card",
                    div { class: "card-pad product-section-head",
                        div {
                            h3 { "Observed model performance" }
                            p { "Rows are prioritized by failures, then fallback usage, then average latency. Upstream counts describe only what appeared in this sample." }
                        }
                    }
                    div { class: "table-wrap",
                        table { class: "data-table",
                            thead { tr {
                                th { "Model" }
                                th { "Signal" }
                                th { class: "right", "Requests" }
                                th { class: "right", "Success" }
                                th { class: "right", "Fallbacks" }
                                th { class: "right", "Avg Latency" }
                                th { class: "right", "Tokens" }
                                th { class: "right", "Cost" }
                                th { "Observed Upstreams" }
                            } }
                            tbody {
                                for (model_name, evaluation) in model_rows {
                                    {
                                        let failures = evaluation.failures();
                                        let success_rate = if evaluation.requests == 0 {
                                            0.0
                                        } else {
                                            evaluation.success as f64 * 100.0 / evaluation.requests as f64
                                        };
                                        let success_text = format!("{success_rate:.1}%");
                                        let latency_text = format_latency(evaluation.avg_latency());
                                        let token_text = compact(evaluation.tokens);
                                        let cost_text = format!("${:.6}", evaluation.cost);
                                        let upstream_count = evaluation.upstreams.len();
                                        let upstream_text = if upstream_count == 0 {
                                            "No upstream ID recorded".to_string()
                                        } else {
                                            evaluation.upstreams.into_iter().collect::<Vec<_>>().join(", ")
                                        };
                                        let upstream_count_text = if upstream_count == 1 {
                                            "1 observed".to_string()
                                        } else {
                                            format!("{upstream_count} observed")
                                        };
                                        let (signal_class, signal_text) = if failures > 0 {
                                            ("badge badge-error", "Errors observed")
                                        } else if evaluation.fallbacks > 0 {
                                            ("badge badge-warning", "Fallback observed")
                                        } else {
                                            ("badge badge-success", "Stable sample")
                                        };
                                        rsx! {
                                            tr { key: "{model_name}",
                                                td { class: "table-primary mono", title: "{model_name}", "{model_name}" }
                                                td { span { class: "{signal_class}", "{signal_text}" } }
                                                td { class: "right tabular", "{evaluation.requests}" }
                                                td { class: "right strong tabular", "{success_text}" }
                                                td { class: "right tabular", "{evaluation.fallbacks}" }
                                                td { class: "right tabular", "{latency_text}" }
                                                td { class: "right tabular", "{token_text}" }
                                                td { class: "right tabular", "{cost_text}" }
                                                td {
                                                    div { class: "two-line",
                                                        strong { class: "small", "{upstream_count_text}" }
                                                        small { class: "muted", title: "{upstream_text}", "{upstream_text}" }
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

                div { class: "product-note", "Evaluation uses observed request-log data only. Accuracy, hallucination rate, task quality, benchmark scores, and configured failover capacity require different evidence and are intentionally not inferred here." }
            }
        }
    }
}
