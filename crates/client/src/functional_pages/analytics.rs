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

fn currency(value: f64) -> String {
    if value == 0.0 {
        "$0.00".to_string()
    } else if value.abs() < 1.0 {
        format!("${value:.6}")
    } else {
        format!("${value:.2}")
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
    let load_error = snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let data: BillingSummary = snapshot.and_then(Result::ok).unwrap_or_default();

    let request_count =
        data.models.iter().map(|model| model.requests).sum::<i64>() + data.pre_migration_requests;
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
    let cost_text = currency(data.total_cost_usd);
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

    let top_model_name = models
        .first()
        .map(|model| model.model.clone())
        .unwrap_or_else(|| "No model-level spend".to_string());
    let top_model_share = models
        .first()
        .map(|model| {
            if data.total_cost_usd > 0.0 {
                model.cost_usd * 100.0 / data.total_cost_usd
            } else {
                0.0
            }
        })
        .unwrap_or(0.0);
    let top_model_share_text = format!("{top_model_share:.1}%");
    let model_note = if model_count == 1 {
        "1 model with billed usage".to_string()
    } else {
        format!("{model_count} models with billed usage")
    };
    let request_note = if data.pre_migration_requests > 0 {
        format!(
            "includes {} requests without model attribution",
            data.pre_migration_requests
        )
    } else {
        "billed request activity".to_string()
    };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Billing" }
                    p { class: "page-subtitle", "Understand total spend first, then which models drove it and the billed usage behind each model." }
                }
                div { class: "header-actions",
                    if !loading && load_error.is_none() {
                        span { class: "badge badge-neutral mono", "{period_text}" }
                    }
                    button {
                        class: "button button-secondary",
                        disabled: loading,
                        onclick: move |_| summary_resource.restart(),
                        if loading { "Refreshing…" } else { "Refresh" }
                    }
                }
            }

            if loading {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "billing" } }
                        h3 { "Loading billing summary" }
                        p { "Reading the current billing period, total recorded cost, model-level spend, and token usage before showing financial conclusions." }
                    }
                }
            } else if let Some(message) = load_error {
                div { class: "card card-pad stack",
                    strong { class: "danger", "Billing could not be loaded" }
                    code { class: "terminal", "{message}" }
                    button { class: "button button-primary", onclick: move |_| summary_resource.restart(), "Retry" }
                }
            } else {
                div { class: "card card-pad-lg stack",
                    div { class: "row between gap-3",
                        div {
                            div { class: "section-label", "Billing period spend" }
                            h2 { class: "page-title mono", "{cost_text}" }
                            p { class: "page-subtitle", "Recorded billed cost for {period_text}. Spend is the primary billing conclusion; usage and concentration below explain what drove it." }
                        }
                        if let Some(top_model) = models.first() {
                            div { class: "product-note",
                                strong { "Top spend driver" }
                                div { class: "small mono", "{top_model.model}" }
                                div { class: "tiny muted", "{top_model_share_text} of recorded spend" }
                            }
                        }
                    }
                }

                div { class: "metrics",
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Requests" } span { class: "metric-value", "{request_text}" } span { class: "metric-note", "{request_note}" } }
                        div { class: "metric-icon tone-gray", Icon { name: "activity" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Tokens" } span { class: "metric-value", "{token_text}" } span { class: "metric-note", "input + cache read + output + reasoning" } }
                        div { class: "metric-icon tone-gray", Icon { name: "models" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Models Used" } span { class: "metric-value", "{model_count}" } span { class: "metric-note", "{model_note}" } }
                        div { class: "metric-icon tone-gray", Icon { name: "routes" } }
                    }
                    div { class: "card metric",
                        div { class: "metric-copy", span { class: "metric-label", "Top Model Share" } span { class: "metric-value", "{top_model_share_text}" } span { class: "metric-note", "{top_model_name}" } }
                        div { class: "metric-icon tone-gray", Icon { name: "dollar" } }
                    }
                }

                if models.is_empty() && request_count == 0 {
                    div { class: "card product-empty",
                        div { class: "product-empty-inner",
                            div { class: "product-empty-icon", Icon { name: "billing" } }
                            h3 { "No billed usage yet" }
                            p { "Once this account sends routed traffic with billable usage, model-level spend and token usage will appear here." }
                        }
                    }
                } else if models.is_empty() {
                    div { class: "card product-empty",
                        div { class: "product-empty-inner",
                            div { class: "product-empty-icon", Icon { name: "billing" } }
                            h3 { "No model-level billing breakdown is available" }
                            p { "Request activity exists, but it predates or falls outside the current model-attributed billing breakdown. The total request count above remains truthful while model spend stays empty." }
                        }
                    }
                } else {
                    div { class: "card table-card",
                        div { class: "card-pad product-section-head",
                            div {
                                h3 { "Spend by model" }
                                p { "Models are sorted by recorded cost. Spend share explains concentration without treating high spend as an error state." }
                            }
                        }
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Model" }
                                    th { class: "right", "Spend" }
                                    th { class: "right", "Share" }
                                    th { class: "right", "Requests" }
                                    th { class: "right", "Input" }
                                    th { class: "right", "Cache Read" }
                                    th { class: "right", "Output" }
                                    th { class: "right", "Reasoning" }
                                } }
                                tbody {
                                    for model in models {
                                        {
                                            let spend = currency(model.cost_usd);
                                            let share = if data.total_cost_usd > 0.0 {
                                                model.cost_usd * 100.0 / data.total_cost_usd
                                            } else {
                                                0.0
                                            };
                                            let share_text = format!("{share:.1}%");
                                            let requests = compact(model.requests);
                                            let input = compact(model.prompt_tokens);
                                            let cache = compact(model.cache_read_tokens);
                                            let output = compact(model.completion_tokens);
                                            let reasoning = compact(model.reasoning_tokens);
                                            rsx! {
                                                tr { key: "{model.model}",
                                                    td { class: "table-primary mono", title: "{model.model}", "{model.model}" }
                                                    td { class: "right strong tabular", "{spend}" }
                                                    td { class: "right tabular", "{share_text}" }
                                                    td { class: "right tabular", "{requests}" }
                                                    td { class: "right tabular", "{input}" }
                                                    td { class: "right tabular", "{cache}" }
                                                    td { class: "right tabular", "{output}" }
                                                    td { class: "right tabular", "{reasoning}" }
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
