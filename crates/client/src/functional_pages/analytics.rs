use dioxus::prelude::*;

use crate::{
    backend::{billing_summary, use_auth, BillingSummary},
    components::Icon,
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

#[component]
pub fn Billing() -> Element {
    let auth = use_auth();
    let token = auth.token().unwrap_or_default();
    let token_for_resource = token.clone();
    let mut summary_resource = use_resource(move || {
        let token = token_for_resource.clone();
        async move {
            if token.is_empty() {
                Err("No authenticated token".to_string())
            } else {
                billing_summary(&token).await
            }
        }
    });

    let snapshot = summary_resource.read().clone();
    let loading = snapshot.is_none();
    let load_error = snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let data: BillingSummary = snapshot.and_then(Result::ok).unwrap_or_default();

    let request_count = data.models.iter().map(|model| model.requests).sum::<i64>() + data.pre_migration_requests;
    let total_tokens = data
        .models
        .iter()
        .map(|model| {
            model.prompt_tokens
                + model.cache_read_tokens
                + model.completion_tokens
                + model.reasoning_tokens
        })
        .sum::<i64>();
    let model_count = data.models.len();
    let request_text = compact(request_count);
    let token_text = compact(total_tokens);
    let cost_text = format!("${:.4}", data.total_cost_usd);
    let avg_request_cost = if request_count > 0 {
        data.total_cost_usd / request_count as f64
    } else {
        0.0
    };
    let avg_request_cost_text = format!("${avg_request_cost:.6}");
    let period_text = match (&data.period_start, &data.period_end) {
        (Some(start), Some(end)) => format!("{start} → {end}"),
        _ => "Current billing period".to_string(),
    };

    let mut models = data.models.clone();
    models.sort_by(|left, right| {
        right
            .cost_usd
            .partial_cmp(&left.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Billing" }
                    p { class: "page-subtitle", "See total spend first, then identify which models and traffic patterns are driving the bill." }
                }
                div { class: "header-actions",
                    span { class: "badge badge-neutral mono", "{period_text}" }
                    button { class: "button button-secondary", onclick: move |_| summary_resource.restart(), "Refresh" }
                }
            }

            if loading {
                div { class: "card card-pad", "Loading billing summary…" }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Billing could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| summary_resource.restart(), "Retry" }
                }
            } else {
                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Spend" } span { class: "metric-value", "{cost_text}" } span { class: "metric-note", "current billing period" } }
                        div { class: "metric-icon tone-amber", Icon { name: "dollar" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Requests" } span { class: "metric-value", "{request_text}" } span { class: "metric-note", "billed request activity" } }
                        div { class: "metric-icon tone-blue", Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Avg / Request" } span { class: "metric-value", "{avg_request_cost_text}" } span { class: "metric-note", "blended average cost" } }
                        div { class: "metric-icon tone-purple", Icon { name: "billing" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Tokens" } span { class: "metric-value", "{token_text}" } span { class: "metric-note", "all billed token types" } }
                        div { class: "metric-icon tone-green", Icon { name: "models" } }
                    }
                }

                if models.is_empty() {
                    div { class: "card product-empty",
                        div { class: "product-empty-inner",
                            div { class: "product-empty-icon", Icon { name: "billing" } }
                            h3 { "No billed usage yet" }
                            p { "Once this account sends routed traffic, model-level spend and token usage will appear here." }
                        }
                    }
                } else {
                    div { class: "card table-card",
                        div { class: "card-pad product-section-head",
                            div {
                                h3 { "What is driving spend" }
                                p { "Models are sorted by cost. Token composition is shown as secondary context instead of competing with the financial view." }
                            }
                            span { class: "small muted", "{model_count} billed models" }
                        }
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Model" }
                                    th { class: "right", "Spend" }
                                    th { class: "right", "Share" }
                                    th { class: "right", "Requests" }
                                    th { class: "right", "Tokens" }
                                    th { class: "right", "Avg / Request" }
                                } }
                                tbody {
                                    for model in models {
                                        {
                                            let spend = format!("${:.6}", model.cost_usd);
                                            let share = if data.total_cost_usd > 0.0 {
                                                model.cost_usd * 100.0 / data.total_cost_usd
                                            } else {
                                                0.0
                                            };
                                            let share_text = format!("{share:.1}%");
                                            let requests = compact(model.requests);
                                            let model_tokens = model.prompt_tokens
                                                + model.cache_read_tokens
                                                + model.completion_tokens
                                                + model.reasoning_tokens;
                                            let token_total = compact(model_tokens);
                                            let avg_cost = if model.requests > 0 {
                                                model.cost_usd / model.requests as f64
                                            } else {
                                                0.0
                                            };
                                            let avg_cost_text = format!("${avg_cost:.6}");
                                            let mix = format!(
                                                "in {} • cache {} • out {} • reasoning {}",
                                                compact(model.prompt_tokens),
                                                compact(model.cache_read_tokens),
                                                compact(model.completion_tokens),
                                                compact(model.reasoning_tokens)
                                            );
                                            rsx! {
                                                tr { key: "{model.model}",
                                                    td {
                                                        div { class: "two-line",
                                                            strong { class: "table-primary mono", "{model.model}" }
                                                            small { class: "mono muted", "{mix}" }
                                                        }
                                                    }
                                                    td { class: "right strong tabular", "{spend}" }
                                                    td { class: "right tabular", "{share_text}" }
                                                    td { class: "right tabular", "{requests}" }
                                                    td { class: "right tabular", "{token_total}" }
                                                    td { class: "right tabular", "{avg_cost_text}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if data.pre_migration_requests > 0 {
                    div { class: "product-note", "{data.pre_migration_requests} requests predate the current model-level billing breakdown and are included in the total request count without model attribution." }
                }
            }
        }
    }
}
