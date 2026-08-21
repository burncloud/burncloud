use crate::api::auth::Claims;
use crate::api::response::{err_status, ok};
use crate::AppState;
use axum::{
    extract::{Extension, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use burncloud_database_user::UserDatabase;
use burncloud_service_channel::{Channel, ChannelService};
use burncloud_service_router_log::{BillingModelSummary, BillingService, RouterLogService};
use burncloud_service_token::TokenService as ApiTokenService;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeSet;

const AVAILABILITY_SAMPLE_LIMIT: i32 = 500;
const DEFAULT_USD_LOW_NANO: i64 = 5_000_000_000;
const DEFAULT_USD_CRITICAL_NANO: i64 = 1_000_000_000;
const DEFAULT_CNY_LOW_NANO: i64 = 35_000_000_000;
const DEFAULT_CNY_CRITICAL_NANO: i64 = 7_000_000_000;
const DEFAULT_HEALTHY_PERCENT: f64 = 99.0;
const DEFAULT_DEGRADED_PERCENT: f64 = 95.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum BalanceState {
    Healthy,
    Low,
    Critical,
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AvailabilityState {
    Healthy,
    Degraded,
    AtRisk,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
struct BuyerModelSummary {
    name: String,
    tier: String,
    tokens_today: i64,
    status: String,
    destination: String,
}

#[derive(Debug, Clone, Serialize)]
struct BuyerActivityEvent {
    kind: String,
    title: String,
    detail: String,
    occurred_at_utc: String,
}

#[derive(Serialize)]
struct BuyerOverviewSnapshot {
    as_of_utc: String,
    balance_nano: i64,
    balance_currency: String,
    balance_state: BalanceState,
    today_spend_usd: Option<f64>,
    tokens_today: Option<i64>,
    api_availability: AvailabilityState,
    availability_percent: Option<f64>,
    availability_sample_requests: i64,
    models_today: Vec<BuyerModelSummary>,
    recent_activity: Vec<BuyerActivityEvent>,
    issues: Vec<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/console/api/buyer/overview", get(buyer_overview))
}

fn env_i64(name: &str, fallback: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value >= 0)
        .unwrap_or(fallback)
}

fn env_f64(name: &str, fallback: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| (0.0..=100.0).contains(value))
        .unwrap_or(fallback)
}

fn classify_balance(balance_nano: i64, low_nano: i64, critical_nano: i64) -> BalanceState {
    if balance_nano <= 0 {
        BalanceState::Exhausted
    } else if balance_nano <= critical_nano {
        BalanceState::Critical
    } else if balance_nano <= low_nano {
        BalanceState::Low
    } else {
        BalanceState::Healthy
    }
}

fn channel_supports_model(channel: &Channel, model: &str) -> bool {
    channel
        .models
        .split(',')
        .map(str::trim)
        .any(|candidate| candidate.eq_ignore_ascii_case(model))
}

fn summarize_models(usage: &[BillingModelSummary], channels: &[Channel]) -> Vec<BuyerModelSummary> {
    usage
        .iter()
        .map(|model| {
            let supporting: Vec<_> = channels
                .iter()
                .filter(|channel| channel_supports_model(channel, &model.model))
                .collect();
            let tiers: BTreeSet<_> = supporting
                .iter()
                .flat_map(|channel| channel.group.split(','))
                .map(str::trim)
                .filter(|group| !group.is_empty())
                .map(str::to_string)
                .collect();
            let tier = if tiers.is_empty() {
                "Historical".to_string()
            } else {
                tiers.into_iter().collect::<Vec<_>>().join(" / ")
            };
            let status = if supporting.iter().any(|channel| channel.status == 1) {
                "available"
            } else {
                "unavailable"
            };
            BuyerModelSummary {
                name: model.model.clone(),
                tier,
                tokens_today: model.prompt_tokens + model.completion_tokens,
                status: status.to_string(),
                destination: "/console/buyer/marketplace".to_string(),
            }
        })
        .collect()
}

fn classify_availability(
    successful: i64,
    total: i64,
    healthy_percent: f64,
    degraded_percent: f64,
) -> (AvailabilityState, Option<f64>) {
    if total <= 0 {
        return (AvailabilityState::Unknown, None);
    }
    let percent = successful.max(0) as f64 * 100.0 / total as f64;
    let state = if successful <= 0 {
        AvailabilityState::Unavailable
    } else if percent >= healthy_percent {
        AvailabilityState::Healthy
    } else if percent >= degraded_percent {
        AvailabilityState::Degraded
    } else {
        AvailabilityState::AtRisk
    };
    (state, Some(percent))
}

async fn buyer_overview(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Response {
    let roles = match state
        .user_service
        .get_user_roles(&state.db, &claims.sub)
        .await
    {
        Ok(roles) => roles,
        Err(error) => {
            tracing::error!(user_id = %claims.sub, error = %error, "Failed to resolve Buyer roles");
            return err_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not verify Buyer access",
            )
            .into_response();
        }
    };
    let has_buyer_access = roles
        .iter()
        .any(|role| role.eq_ignore_ascii_case("buyer") || role.eq_ignore_ascii_case("user"));
    if !has_buyer_access {
        return err_status(StatusCode::FORBIDDEN, "Buyer access is required").into_response();
    }

    let account = match UserDatabase::get_user_by_id(&state.db, &claims.sub).await {
        Ok(Some(account)) => account,
        Ok(None) => {
            return err_status(StatusCode::NOT_FOUND, "Buyer account was not found").into_response()
        }
        Err(error) => {
            tracing::error!(user_id = %claims.sub, error = %error, "Failed to load Buyer account");
            return err_status(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Could not load Buyer account",
            )
            .into_response();
        }
    };

    let currency = account
        .preferred_currency
        .as_deref()
        .unwrap_or("USD")
        .to_ascii_uppercase();
    let (balance_nano, low_nano, critical_nano) = if currency == "CNY" {
        (
            account.balance_cny,
            env_i64("BURNCLOUD_BUYER_CNY_LOW_NANO", DEFAULT_CNY_LOW_NANO),
            env_i64(
                "BURNCLOUD_BUYER_CNY_CRITICAL_NANO",
                DEFAULT_CNY_CRITICAL_NANO,
            ),
        )
    } else {
        (
            account.balance_usd,
            env_i64("BURNCLOUD_BUYER_USD_LOW_NANO", DEFAULT_USD_LOW_NANO),
            env_i64(
                "BURNCLOUD_BUYER_USD_CRITICAL_NANO",
                DEFAULT_USD_CRITICAL_NANO,
            ),
        )
    };

    let today = Utc::now().date_naive().format("%Y-%m-%d").to_string();
    let (billing_result, logs_result, channels_result, recharges_result, tokens_result) = tokio::join!(
        BillingService::get_billing_summary_for_user(
            &state.db,
            &claims.sub,
            Some(&today),
            Some(&today),
        ),
        RouterLogService::get_filtered(
            &state.db,
            Some(&claims.sub),
            None,
            None,
            AVAILABILITY_SAMPLE_LIMIT,
            0,
        ),
        ChannelService::list(&state.db, 1000, 0),
        state.user_service.list_recharges(&state.db, &claims.sub),
        ApiTokenService::list(&state.db),
    );

    let mut issues = Vec::new();
    let mut daily_models = Vec::new();
    let (today_spend_usd, tokens_today) = match billing_result {
        Ok(summary) => {
            let tokens = summary
                .models
                .iter()
                .map(|model| model.prompt_tokens + model.completion_tokens)
                .sum();
            daily_models = summary.models;
            (Some(summary.total_cost_usd), Some(tokens))
        }
        Err(error) => {
            tracing::warn!(user_id = %claims.sub, error = %error, "Buyer daily billing is unavailable");
            issues.push("daily_usage_unavailable".to_string());
            (None, None)
        }
    };

    let (api_availability, availability_percent, availability_sample_requests) = match logs_result {
        Ok(logs) => {
            let today_logs: Vec<_> = logs
                .iter()
                .filter(|log| {
                    log.created_at
                        .as_deref()
                        .is_some_and(|created_at| created_at.starts_with(&today))
                })
                .collect();
            let total = today_logs.len() as i64;
            let successful = today_logs
                .iter()
                .filter(|log| (200..400).contains(&log.status_code))
                .count() as i64;
            let (availability, percent) = classify_availability(
                successful,
                total,
                env_f64(
                    "BURNCLOUD_BUYER_AVAILABILITY_HEALTHY_PERCENT",
                    DEFAULT_HEALTHY_PERCENT,
                ),
                env_f64(
                    "BURNCLOUD_BUYER_AVAILABILITY_DEGRADED_PERCENT",
                    DEFAULT_DEGRADED_PERCENT,
                ),
            );
            (availability, percent, total)
        }
        Err(error) => {
            tracing::warn!(user_id = %claims.sub, error = %error, "Buyer availability sample is unavailable");
            issues.push("availability_unavailable".to_string());
            (AvailabilityState::Unknown, None, 0)
        }
    };

    let channels = match channels_result {
        Ok(channels) => channels,
        Err(error) => {
            tracing::warn!(user_id = %claims.sub, error = %error, "Buyer model catalog is unavailable");
            issues.push("model_catalog_unavailable".to_string());
            Vec::new()
        }
    };
    let models_today = summarize_models(&daily_models, &channels);
    let balance_state = classify_balance(balance_nano, low_nano, critical_nano);
    let now = Utc::now().to_rfc3339();
    let mut recent_activity = Vec::new();

    match recharges_result {
        Ok(recharges) => {
            for recharge in recharges.into_iter().take(3) {
                recent_activity.push(BuyerActivityEvent {
                    kind: "recharge".to_string(),
                    title: "Balance recharged".to_string(),
                    detail: format!(
                        "{} {:.2} was added to this account.",
                        recharge.currency.as_deref().unwrap_or("USD"),
                        recharge.amount as f64 / 1_000_000_000.0
                    ),
                    occurred_at_utc: recharge.created_at.unwrap_or_else(|| now.clone()),
                });
            }
        }
        Err(error) => {
            tracing::warn!(user_id = %claims.sub, error = %error, "Buyer recharge history is unavailable");
            issues.push("recharge_history_unavailable".to_string());
        }
    }

    match tokens_result {
        Ok(tokens) => {
            for token in tokens
                .into_iter()
                .filter(|token| token.user_id == claims.sub)
                .take(3)
            {
                let occurred_at_utc = DateTime::<Utc>::from_timestamp(token.created_at, 0)
                    .map(|value| value.to_rfc3339())
                    .unwrap_or_else(|| now.clone());
                recent_activity.push(BuyerActivityEvent {
                    kind: "api_key_created".to_string(),
                    title: "API key created".to_string(),
                    detail: format!(
                        "Credential version {} was created for this account.",
                        token.key_version
                    ),
                    occurred_at_utc,
                });
            }
        }
        Err(error) => {
            tracing::warn!(user_id = %claims.sub, error = %error, "Buyer credential activity is unavailable");
            issues.push("credential_activity_unavailable".to_string());
        }
    }

    for model in models_today.iter().take(3) {
        recent_activity.push(BuyerActivityEvent {
            kind: "model_usage".to_string(),
            title: format!("{} used today", model.name),
            detail: format!("{} tokens recorded in UTC today.", model.tokens_today),
            occurred_at_utc: format!("{today}T00:00:00Z"),
        });
    }
    if balance_state != BalanceState::Healthy {
        recent_activity.push(BuyerActivityEvent {
            kind: "balance_warning".to_string(),
            title: "Balance needs attention".to_string(),
            detail: format!("The configured balance state is {:?}.", balance_state)
                .to_ascii_lowercase(),
            occurred_at_utc: now.clone(),
        });
    }
    if !matches!(api_availability, AvailabilityState::Unknown) {
        recent_activity.push(BuyerActivityEvent {
            kind: if api_availability == AvailabilityState::Healthy {
                "service_recovery"
            } else {
                "service_incident"
            }
            .to_string(),
            title: if api_availability == AvailabilityState::Healthy {
                "API availability healthy"
            } else {
                "API availability needs attention"
            }
            .to_string(),
            detail: availability_percent
                .map(|percent| {
                    format!("Observed Buyer request availability is {percent:.2}% today.")
                })
                .unwrap_or_else(|| "Observed availability is unavailable.".to_string()),
            occurred_at_utc: now,
        });
    }
    recent_activity.sort_by(|left, right| right.occurred_at_utc.cmp(&left.occurred_at_utc));
    recent_activity.truncate(8);

    ok(BuyerOverviewSnapshot {
        as_of_utc: today,
        balance_nano,
        balance_currency: currency,
        balance_state,
        today_spend_usd,
        tokens_today,
        api_availability,
        availability_percent,
        availability_sample_requests,
        models_today,
        recent_activity,
        issues,
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_state_respects_configured_thresholds() {
        assert_eq!(classify_balance(0, 5_000, 1_000), BalanceState::Exhausted);
        assert_eq!(classify_balance(500, 5_000, 1_000), BalanceState::Critical);
        assert_eq!(classify_balance(2_000, 5_000, 1_000), BalanceState::Low);
        assert_eq!(classify_balance(6_000, 5_000, 1_000), BalanceState::Healthy);
    }

    #[test]
    fn model_summary_exposes_sanitized_tier_and_status() {
        let usage = BillingModelSummary {
            model: "gpt-test".to_string(),
            requests: 2,
            prompt_tokens: 100,
            cache_read_tokens: 0,
            completion_tokens: 25,
            reasoning_tokens: 0,
            cost_usd: 0.5,
        };
        let channel = Channel {
            id: 1,
            type_: 1,
            key: "secret".to_string(),
            status: 1,
            name: "internal-provider".to_string(),
            weight: 1,
            created_time: None,
            test_time: None,
            response_time: None,
            base_url: None,
            models: "gpt-test".to_string(),
            group: "standard".to_string(),
            used_quota: 0,
            model_mapping: None,
            priority: 0,
            auto_ban: 0,
            other_info: None,
            tag: None,
            setting: None,
            param_override: None,
            header_override: None,
            remark: None,
            api_version: None,
            pricing_region: None,
            rpm_cap: None,
            tpm_cap: None,
            reservation_green: None,
            reservation_yellow: None,
            reservation_red: None,
        };

        let models = summarize_models(&[usage], &[channel]);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "gpt-test");
        assert_eq!(models[0].tier, "standard");
        assert_eq!(models[0].tokens_today, 125);
        assert_eq!(models[0].status, "available");
        assert_eq!(models[0].destination, "/console/buyer/marketplace");
    }

    #[test]
    fn availability_uses_only_observed_buyer_requests() {
        assert_eq!(
            classify_availability(0, 0, 99.0, 95.0),
            (AvailabilityState::Unknown, None)
        );
        assert_eq!(
            classify_availability(0, 4, 99.0, 95.0).0,
            AvailabilityState::Unavailable
        );
        assert_eq!(
            classify_availability(95, 100, 99.0, 95.0).0,
            AvailabilityState::Degraded
        );
        assert_eq!(
            classify_availability(99, 100, 99.0, 95.0).0,
            AvailabilityState::Healthy
        );
    }
}
