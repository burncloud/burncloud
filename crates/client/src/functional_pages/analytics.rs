use std::collections::BTreeMap;

use dioxus::prelude::*;

use crate::{
    backend::{billing_summary, use_auth, BillingSummary, LogService, RouterLog},
    components::Icon,
};

fn compact(n: i64) -> String {
    if n.abs() >= 1_000_000_000 { format!("{:.2}B", n as f64 / 1_000_000_000.0) }
    else if n.abs() >= 1_000_000 { format!("{:.2}M", n as f64 / 1_000_000.0) }
    else if n.abs() >= 1_000 { format!("{:.2}K", n as f64 / 1_000.0) }
    else { n.to_string() }
}

#[component]
pub fn Billing() -> Element {
    let auth = use_auth();
    let token = auth.token().unwrap_or_default();
    let token_for_resource = token.clone();
    let mut summary = use_resource(move || {
        let token = token_for_resource.clone();
        async move {
            if token.is_empty() { Err("No authenticated token".to_string()) } else { billing_summary(&token).await }
        }
    });
    let result = summary.read().clone();
    let loading = result.is_none();
    let load_error = result.as_ref().and_then(|r| r.as_ref().err().cloned());
    let data: BillingSummary = result.and_then(Result::ok).unwrap_or_default();
    let request_count = data.models.iter().map(|m| m.requests).sum::<i64>() + data.pre_migration_requests;
    let prompt_tokens = data.models.iter().map(|m| m.prompt_tokens).sum::<i64>();
    let completion_tokens = data.models.iter().map(|m| m.completion_tokens).sum::<i64>();
    let request_text = compact(request_count);
    let prompt_text = compact(prompt_tokens);
    let completion_text = compact(completion_tokens);
    let cost_text = format!("${:.6}", data.total_cost_usd);
    let period_text = match (&data.period_start, &data.period_end) {
        (Some(start), Some(end)) => format!("{start} → {end}"),
        _ => "Current server billing period".to_string(),
    };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div { h2 { class: "page-title", "Billing" } p { class: "page-subtitle", "Live per-user billing summary from /api/billing/summary." } }
                button { class: "button button-secondary", onclick: move |_| summary.restart(), "Refresh" }
            }
            if loading {
                div { class: "card card-pad", "Loading billing summary…" }
            } else if let Some(message) = load_error {
                div { class: "terminal auth-status auth-status-error", "{message}" }
            } else {
                div { class: "card card-pad row between", span { class: "section-label", "Billing Period" } strong { class: "mono", "{period_text}" } }
                div { class: "metrics",
                    div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Requests" } span { class: "metric-value", "{request_text}" } } div { class: "metric-icon tone-blue", Icon { name: "activity" } } }
                    div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Prompt Tokens" } span { class: "metric-value", "{prompt_text}" } } div { class: "metric-icon tone-purple", Icon { name: "models" } } }
                    div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Completion Tokens" } span { class: "metric-value", "{completion_text}" } } div { class: "metric-icon tone-green", Icon { name: "models" } } }
                    div { class: "card metric", div { class: "metric-copy", span { class: "metric-label", "Total Cost" } span { class: "metric-value", "{cost_text}" } } div { class: "metric-icon tone-amber", Icon { name: "dollar" } } }
                }
                div { class: "card table-card",
                    if data.models.is_empty() {
                        div { class: "card-pad small muted", "No billed model usage is available for this account and period." }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr { th { "Model" } th { class: "right", "Requests" } th { class: "right", "Prompt" } th { class: "right", "Cache Read" } th { class: "right", "Completion" } th { class: "right", "Reasoning" } th { class: "right", "Cost" } } }
                                tbody {
                                    for model in data.models {
                                        {
                                            let requests = compact(model.requests);
                                            let prompt = compact(model.prompt_tokens);
                                            let cache = compact(model.cache_read_tokens);
                                            let completion = compact(model.completion_tokens);
                                            let reasoning = compact(model.reasoning_tokens);
                                            let cost = format!("${:.6}", model.cost_usd);
                                            rsx! {
                                                tr { key: "{model.model}",
                                                    td { class: "table-primary mono", "{model.model}" }
                                                    td { class: "right tabular", "{requests}" }
                                                    td { class: "right tabular", "{prompt}" }
                                                    td { class: "right tabular", "{cache}" }
                                                    td { class: "right tabular", "{completion}" }
                                                    td { class: "right tabular", "{reasoning}" }
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
        }
    }
}

#[derive(Default, Clone)]
struct ModelEval {
    requests: i64,
    success: i64,
    latency_sum: i64,
    tokens: i64,
    cost: f64,
    upstreams: std::collections::BTreeSet<String>,
}

#[component]
pub fn Evaluation() -> Element {
    let mut logs = use_resource(move || async move { LogService::list(500).await });
    let result = logs.read().clone();
    let loading = result.is_none();
    let load_error = result.as_ref().and_then(|r| r.as_ref().err().cloned());
    let list: Vec<RouterLog> = result.and_then(Result::ok).unwrap_or_default();
    let mut models: BTreeMap<String, ModelEval> = BTreeMap::new();
    for log in &list {
        let model_name = log.model.clone().unwrap_or_else(|| "unattributed".to_string());
        let entry = models.entry(model_name).or_default();
        entry.requests += 1;
        if log.status_code < 400 { entry.success += 1; }
        entry.latency_sum += log.latency_ms;
        entry.tokens += log.total_tokens();
        entry.cost += log.cost_usd();
        if let Some(upstream) = &log.upstream_id { entry.upstreams.insert(upstream.clone()); }
    }

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div { h2 { class: "page-title", "Evaluation" } p { class: "page-subtitle", "Operational model evaluation derived from real router logs — success rate, latency, token volume and cost. No synthetic benchmark scores." } }
                button { class: "button button-secondary", onclick: move |_| logs.restart(), "Refresh" }
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
                                thead { tr { th { "Model" } th { class: "right", "Requests" } th { class: "right", "Success" } th { class: "right", "Avg Latency" } th { class: "right", "Tokens" } th { class: "right", "Cost" } th { "Observed Upstreams" } } }
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
                div { class: "card card-pad tiny subtle", "Evaluation is intentionally based on observed production routing data. A quality/accuracy benchmark needs a dedicated evaluation backend, which BurnCloud does not currently expose." }
            }
        }
    }
}
