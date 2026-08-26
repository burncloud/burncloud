use chrono::{NaiveDateTime, Utc};
use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{
        billing_summary_for_period, use_auth, BillingSummary, CurrentAccount, TokenDto,
        TokenService, UserRecharge, UserService,
    },
    components::Icon,
};

fn compact(value: i64) -> String {
    let absolute = value.abs();
    if absolute >= 1_000_000_000 {
        format!("{:.2}B", value as f64 / 1_000_000_000.0)
    } else if absolute >= 1_000_000 {
        format!("{:.2}M", value as f64 / 1_000_000.0)
    } else if absolute >= 1_000 {
        format!("{:.2}K", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn metric(
    label: &'static str,
    value: String,
    note: &'static str,
    icon: &'static str,
    tone: &'static str,
) -> Element {
    rsx! {
        div { class: "card metric card-hover",
            div { class: "metric-copy",
                span { class: "metric-label", "{label}" }
                span { class: "metric-value", "{value}" }
                span { class: "metric-note", "{note}" }
            }
            div { class: "metric-icon {tone}", Icon { name: icon } }
        }
    }
}

fn account_balance(account: &CurrentAccount) -> (String, i64) {
    if account.preferred_currency.as_deref() == Some("CNY") {
        ("CNY".to_string(), account.balance_cny)
    } else {
        ("USD".to_string(), account.balance_usd)
    }
}

fn format_money_nano(amount: i64, currency: &str) -> String {
    let value = amount as f64 / 1_000_000_000.0;
    match currency {
        "CNY" => format!("CNY {value:.2}"),
        _ => format!("${value:.2}"),
    }
}

fn daily_tokens(summary: &BillingSummary) -> i64 {
    summary
        .models
        .iter()
        .map(|model| {
            model.prompt_tokens
                + model.cache_read_tokens
                + model.completion_tokens
                + model.reasoning_tokens
        })
        .sum()
}

fn recharge_timestamp(value: Option<&str>) -> i64 {
    let Some(value) = value else {
        return 0;
    };
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date| date.timestamp())
        .or_else(|_| {
            NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S")
                .map(|date| date.and_utc().timestamp())
        })
        .unwrap_or(0)
}

#[derive(Clone)]
struct Activity {
    timestamp: i64,
    title: String,
    detail: String,
    occurred_at: String,
}

fn recent_activity(recharges: &[UserRecharge], tokens: &[TokenDto]) -> Vec<Activity> {
    let mut activity = Vec::new();
    for recharge in recharges {
        let currency = recharge.currency.as_deref().unwrap_or("USD");
        activity.push(Activity {
            timestamp: recharge_timestamp(recharge.created_at.as_deref()),
            title: "Balance funded".to_string(),
            detail: format_money_nano(recharge.amount, currency),
            occurred_at: recharge
                .created_at
                .clone()
                .unwrap_or_else(|| "Timestamp unavailable".to_string()),
        });
    }
    for token in tokens {
        activity.push(Activity {
            timestamp: token.created_at,
            title: "API key created".to_string(),
            detail: format!("Status: {}", token.status),
            occurred_at: chrono::DateTime::from_timestamp(token.created_at, 0)
                .map(|date| date.format("%Y-%m-%d %H:%M UTC").to_string())
                .unwrap_or_else(|| "Timestamp unavailable".to_string()),
        });
    }
    activity.sort_by(|left, right| right.timestamp.cmp(&left.timestamp));
    activity
}

#[component]
pub fn BuyerHome() -> Element {
    let navigator = use_navigator();
    use_effect(move || {
        navigator.replace(Route::BuyerOverview {});
    });
    rsx! {}
}

#[component]
pub fn BuyerOverview() -> Element {
    let auth = use_auth();
    let token = auth.token().unwrap_or_default();
    let account_token = token.clone();
    let recharge_token = token.clone();
    let api_key_token = token.clone();
    let billing_token = token.clone();
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let billing_day = today.clone();

    let mut account_resource = use_resource(move || {
        let token = account_token.clone();
        async move { UserService::current_account(&token).await }
    });
    let mut recharge_resource = use_resource(move || {
        let token = recharge_token.clone();
        async move { UserService::recharges(&token).await }
    });
    let mut token_resource = use_resource(move || {
        let token = api_key_token.clone();
        async move { TokenService::list_with_token(&token).await }
    });
    let mut billing_resource = use_resource(move || {
        let token = billing_token.clone();
        let day = billing_day.clone();
        async move {
            if token.is_empty() {
                Err("No authenticated token".to_string())
            } else {
                billing_summary_for_period(&token, &day, &day).await
            }
        }
    });

    let account_result = account_resource.read().clone();
    let recharge_result = recharge_resource.read().clone();
    let token_result = token_resource.read().clone();
    let billing_result = billing_resource.read().clone();

    let account = account_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let billing = billing_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let api_tokens = token_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let recharges = recharge_result
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    let spend_value = billing
        .map(|summary| format!("${:.2}", summary.total_cost_usd))
        .unwrap_or_else(|| "Unknown".to_string());
    let token_value = billing
        .map(|summary| compact(daily_tokens(summary)))
        .unwrap_or_else(|| "Unknown".to_string());
    let (balance_value, balance_amount) = account
        .map(|account| {
            let (currency, amount) = account_balance(account);
            (format_money_nano(amount, &currency), Some(amount))
        })
        .unwrap_or_else(|| ("Unknown".to_string(), None));

    let active_keys = api_tokens
        .iter()
        .filter(|item| item.status == "active")
        .count();
    let account_or_billing_error = account_result
        .as_ref()
        .is_some_and(|result| result.is_err())
        || billing_result
            .as_ref()
            .is_some_and(|result| result.is_err());
    let is_loading = account_result.is_none() || billing_result.is_none() || token_result.is_none();
    let (status_class, status_title, status_copy) = if is_loading {
        (
            "product-status-card status-attention",
            "Loading account usage",
            "Buyer account, usage, and API key data are being loaded.",
        )
    } else if account_or_billing_error {
        (
            "product-status-card status-blocked",
            "Some account data is unavailable",
            "Balance or billing data could not be loaded. Unknown values are not treated as zero.",
        )
    } else if balance_amount == Some(0) {
        (
            "product-status-card status-blocked",
            "Balance exhausted",
            "This account has no remaining balance. Contact an account administrator before sending more traffic.",
        )
    } else if token_result.as_ref().is_some_and(|result| result.is_ok()) && active_keys == 0 {
        (
            "product-status-card status-attention",
            "Create an API key to start using models",
            "No active API key belongs to this account. Create one before making an inference request.",
        )
    } else {
        (
            "product-status-card status-attention",
            "Usage data is available",
            "Account and daily usage data loaded successfully. Service availability is not currently measured.",
        )
    };

    let errors: Vec<String> = [
        account_result
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .map(|error| format!("Account: {error}")),
        billing_result
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .map(|error| format!("Billing: {error}")),
        token_result
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .map(|error| format!("API keys: {error}")),
        recharge_result
            .as_ref()
            .and_then(|result| result.as_ref().err())
            .map(|error| format!("Funding activity: {error}")),
    ]
    .into_iter()
    .flatten()
    .collect();
    let activities = recent_activity(&recharges, &api_tokens);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Buyer Overview" }
                    p { class: "page-subtitle", "Review today's spend, account balance, usage, and recent account activity." }
                }
                button {
                    class: "button button-secondary",
                    onclick: move |_| {
                        account_resource.restart();
                        recharge_resource.restart();
                        token_resource.restart();
                        billing_resource.restart();
                    },
                    Icon { name: "activity" }
                    "Refresh"
                }
            }

            div { class: "product-hero",
                div { class: "card {status_class}",
                    div { class: "stack",
                        div { class: "row between gap-3",
                            div {
                                h3 { class: "product-status-title", "{status_title}" }
                                p { class: "product-status-copy", "{status_copy}" }
                            }
                            if active_keys == 0 && token_result.as_ref().is_some_and(|result| result.is_ok()) {
                                Link { class: "button button-primary button-sm", to: Route::APIKeys {}, "Create API key" }
                            }
                        }
                        if !errors.is_empty() {
                            div { class: "product-note stack",
                                for error in errors.iter() {
                                    span { class: "small", "{error}" }
                                }
                            }
                        }
                    }
                }
            }

            div { class: "metrics",
                {metric("Today Spend", spend_value, "Today (UTC)", "dollar", "tone-blue")}
                {metric("Balance", balance_value, "Account currency", "billing", "tone-green")}
                {metric("API Availability", "Unknown".to_string(), "Not currently measured", "activity", "tone-gray")}
                {metric("Tokens Today", token_value, "Today (UTC)", "spark", "tone-purple")}
            }

            div { class: "card table-card",
                div { class: "card-pad product-section-head",
                    div {
                        h3 { "Models in Use" }
                        p { "Models billed to this account today. Tier and service status are not currently available." }
                    }
                }
                if let Some(summary) = billing {
                    if summary.models.is_empty() {
                        div { class: "product-empty",
                            div { class: "product-empty-inner",
                                div { class: "product-empty-icon", Icon { name: "models" } }
                                h3 { "No model usage today" }
                                p { "No billed model requests were recorded for this account today (UTC)." }
                            }
                        }
                    } else {
                        div { class: "table-wrap",
                            table { class: "data-table",
                                thead { tr {
                                    th { "Model" }
                                    th { class: "right", "Requests" }
                                    th { class: "right", "Tokens" }
                                    th { class: "right", "Spend" }
                                } }
                                tbody {
                                    for model in summary.models.iter() {
                                        {
                                            let tokens = model.prompt_tokens + model.cache_read_tokens + model.completion_tokens + model.reasoning_tokens;
                                            let requests = compact(model.requests);
                                            let token_count = compact(tokens);
                                            let spend = format!("${:.6}", model.cost_usd);
                                            rsx! { tr {
                                                td { class: "table-primary mono", "{model.model}" }
                                                td { class: "right mono", "{requests}" }
                                                td { class: "right mono", "{token_count}" }
                                                td { class: "right mono", "{spend}" }
                                            } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "product-empty",
                        div { class: "product-empty-inner",
                            div { class: "product-empty-icon", Icon { name: "models" } }
                            h3 { "Model usage unavailable" }
                            p { "Daily billing data must load before model usage can be shown." }
                        }
                    }
                }
            }

            div { class: "card card-pad stack",
                div { class: "product-section-head",
                    div {
                        h3 { "Recent Activity" }
                        p { "Owner-scoped funding and API key events for this account." }
                    }
                }
                if activities.is_empty() {
                    p { class: "small muted", "No funding or API key activity is available yet." }
                } else {
                    div { class: "stack",
                        for activity in activities.iter().take(6) {
                            div { class: "receipt-row",
                                div { class: "two-line",
                                    strong { class: "small", "{activity.title}" }
                                    small { "{activity.occurred_at}" }
                                }
                                span { class: "mono small", "{activity.detail}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
