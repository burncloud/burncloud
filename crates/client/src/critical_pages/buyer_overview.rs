use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{
        buyer_logs, buyer_overview_snapshot, BuyerLogSummary, BuyerOverviewSnapshot, TokenService,
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

fn format_balance(snapshot: &BuyerOverviewSnapshot) -> String {
    let amount = snapshot.balance_nano as f64 / 1_000_000_000.0;
    if snapshot.balance_currency.eq_ignore_ascii_case("CNY") {
        format!("CNY {amount:.2}")
    } else {
        format!("${amount:.2}")
    }
}

fn display_state(value: &str) -> String {
    value
        .split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn metric(
    label: &'static str,
    value: String,
    note: String,
    icon: &'static str,
    tone: &'static str,
    loading: bool,
) -> Element {
    rsx! {
        div { class: "card metric buyer-overview-metric",
            div { class: "metric-copy",
                span { class: "metric-label", "{label}" }
                if loading {
                    span { class: "buyer-overview-skeleton buyer-overview-skeleton-value" }
                    span { class: "buyer-overview-skeleton buyer-overview-skeleton-note" }
                } else {
                    span { class: "metric-value", "{value}" }
                    span { class: "metric-note", "{note}" }
                }
            }
            div { class: "metric-icon {tone}", Icon { name: icon } }
        }
    }
}

fn source_error<T>(result: &Option<Result<T, String>>) -> bool {
    result.as_ref().is_some_and(Result::is_err)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SnapshotModuleState {
    Loading,
    Unavailable,
    Partial,
    Empty,
    Ready,
}

fn has_issue(snapshot: &BuyerOverviewSnapshot, markers: &[&str]) -> bool {
    snapshot
        .issues
        .iter()
        .any(|issue| markers.iter().any(|marker| issue == marker))
}

fn model_module_state(
    result: Option<&Result<BuyerOverviewSnapshot, String>>,
) -> SnapshotModuleState {
    let Some(result) = result else {
        return SnapshotModuleState::Loading;
    };
    let Ok(snapshot) = result else {
        return SnapshotModuleState::Unavailable;
    };
    if has_issue(
        snapshot,
        &["daily_usage_unavailable", "model_catalog_unavailable"],
    ) {
        return SnapshotModuleState::Partial;
    }
    if snapshot.models_today.is_empty() {
        SnapshotModuleState::Empty
    } else {
        SnapshotModuleState::Ready
    }
}

fn activity_is_supported(kind: &str) -> bool {
    matches!(kind, "recharge" | "api_key_created")
}

fn model_tier_label(_source_tier: &str) -> &'static str {
    "Tier unavailable"
}

fn activity_module_state(
    result: Option<&Result<BuyerOverviewSnapshot, String>>,
) -> SnapshotModuleState {
    let Some(result) = result else {
        return SnapshotModuleState::Loading;
    };
    let Ok(snapshot) = result else {
        return SnapshotModuleState::Unavailable;
    };
    if has_issue(
        snapshot,
        &[
            "daily_usage_unavailable",
            "model_catalog_unavailable",
            "recharge_history_unavailable",
            "credential_activity_unavailable",
        ],
    ) {
        return SnapshotModuleState::Partial;
    }
    if snapshot
        .recent_activity
        .iter()
        .all(|event| !activity_is_supported(&event.kind))
    {
        SnapshotModuleState::Empty
    } else {
        SnapshotModuleState::Ready
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ApiKeyReadiness {
    Loading,
    Unknown,
    Missing,
    NonActive,
    Ready,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestHistoryState {
    Loading,
    Unknown,
    Empty,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverviewConclusion {
    Healthy,
    NeedsAttention,
    Unknown,
    SetupRequired,
}

fn derive_api_key_readiness(
    result: Option<&Result<Vec<crate::backend::TokenDto>, String>>,
) -> ApiKeyReadiness {
    let Some(result) = result else {
        return ApiKeyReadiness::Loading;
    };
    let Ok(keys) = result else {
        return ApiKeyReadiness::Unknown;
    };
    if keys.is_empty() {
        return ApiKeyReadiness::Missing;
    }
    if keys
        .iter()
        .any(|key| key.status.eq_ignore_ascii_case("active"))
    {
        return ApiKeyReadiness::Ready;
    }
    ApiKeyReadiness::NonActive
}

fn derive_request_history(
    result: Option<&Result<Vec<BuyerLogSummary>, String>>,
) -> RequestHistoryState {
    let Some(result) = result else {
        return RequestHistoryState::Loading;
    };
    let Ok(logs) = result else {
        return RequestHistoryState::Unknown;
    };
    if logs.is_empty() {
        RequestHistoryState::Empty
    } else {
        RequestHistoryState::Present
    }
}

fn spend_metric(
    snapshot: Option<&BuyerOverviewSnapshot>,
    no_recorded_usage: bool,
) -> (String, String) {
    snapshot
        .and_then(|snapshot| snapshot.today_spend_usd.map(|value| (snapshot, value)))
        .map(|(snapshot, value)| {
            if no_recorded_usage && value == 0.0 {
                (
                    "No spend yet".to_string(),
                    "No requests recorded today".to_string(),
                )
            } else {
                (
                    format!("${value:.4}"),
                    format!("Today in UTC · {}", snapshot.as_of_utc),
                )
            }
        })
        .unwrap_or_else(|| {
            (
                "Unavailable".to_string(),
                "Daily spend could not be confirmed".to_string(),
            )
        })
}

fn tokens_metric(
    snapshot: Option<&BuyerOverviewSnapshot>,
    no_recorded_usage: bool,
) -> (String, String) {
    snapshot
        .and_then(|snapshot| snapshot.tokens_today.map(|tokens| (snapshot, tokens)))
        .map(|(snapshot, tokens)| {
            if no_recorded_usage && tokens == 0 {
                (
                    "No usage yet".to_string(),
                    "No requests recorded today".to_string(),
                )
            } else {
                (
                    compact(tokens),
                    format!("Today in UTC · {}", snapshot.as_of_utc),
                )
            }
        })
        .unwrap_or_else(|| {
            (
                "Unavailable".to_string(),
                "Daily token usage could not be confirmed".to_string(),
            )
        })
}

fn derive_overview_conclusion(
    api_keys: ApiKeyReadiness,
    snapshot: Option<&BuyerOverviewSnapshot>,
) -> OverviewConclusion {
    if snapshot.is_some_and(|snapshot| {
        matches!(
            snapshot.balance_state.as_str(),
            "low" | "critical" | "exhausted"
        )
    }) {
        return OverviewConclusion::NeedsAttention;
    }
    if matches!(
        api_keys,
        ApiKeyReadiness::Missing | ApiKeyReadiness::NonActive
    ) {
        return OverviewConclusion::SetupRequired;
    }
    if api_keys != ApiKeyReadiness::Ready {
        return OverviewConclusion::Unknown;
    }
    let Some(snapshot) = snapshot else {
        return OverviewConclusion::Unknown;
    };
    if matches!(
        snapshot.api_availability.as_str(),
        "degraded" | "at_risk" | "unavailable"
    ) {
        return OverviewConclusion::NeedsAttention;
    }
    if snapshot.today_spend_usd.is_none()
        || snapshot.tokens_today.is_none()
        || !snapshot.issues.is_empty()
    {
        return OverviewConclusion::Unknown;
    }
    if snapshot.balance_state == "healthy" && snapshot.api_availability == "healthy" {
        OverviewConclusion::Healthy
    } else {
        OverviewConclusion::Unknown
    }
}

fn conclusion_class(conclusion: OverviewConclusion) -> &'static str {
    match conclusion {
        OverviewConclusion::Healthy => "status-healthy",
        OverviewConclusion::NeedsAttention | OverviewConclusion::SetupRequired => "status-warning",
        OverviewConclusion::Unknown => "status-unknown",
    }
}

fn conclusion_badge(conclusion: OverviewConclusion) -> &'static str {
    match conclusion {
        OverviewConclusion::Healthy => "HEALTHY",
        OverviewConclusion::NeedsAttention => "NEEDS ATTENTION",
        OverviewConclusion::SetupRequired => "SETUP REQUIRED",
        OverviewConclusion::Unknown => "STATUS UNKNOWN",
    }
}

fn conclusion_title(conclusion: OverviewConclusion) -> &'static str {
    match conclusion {
        OverviewConclusion::Healthy => "Your API usage is healthy.",
        OverviewConclusion::NeedsAttention => "Your account needs attention.",
        OverviewConclusion::SetupRequired => "Buyer setup is incomplete.",
        OverviewConclusion::Unknown => "Account health is unknown.",
    }
}

fn conclusion_copy(conclusion: OverviewConclusion) -> &'static str {
    match conclusion {
        OverviewConclusion::Healthy => {
            "Today's observed usage, balance, and API requests are within configured thresholds."
        }
        OverviewConclusion::NeedsAttention => {
            "BurnCloud confirmed a balance or service condition that may require action."
        }
        OverviewConclusion::SetupRequired => {
            "BurnCloud confirmed an API-key setup gap. Open API Keys to create or review a credential."
        }
        OverviewConclusion::Unknown => {
            "A required account or service signal is unavailable. No healthy state has been inferred."
        }
    }
}

fn balance_attention_copy(state: &str) -> &'static str {
    match state {
        "exhausted" => "New API requests may be rejected because the confirmed balance is exhausted. BurnCloud has not confirmed an automatic recharge or recovery action. Contact your account administrator or support to restore funding; recorded spend remains visible on this page.",
        "critical" => "API requests may soon be rejected because the confirmed balance is critical. BurnCloud has not initiated a recharge. Contact your account administrator or support to arrange funding and monitor recorded spend here.",
        _ => "Requests can continue, but the confirmed balance has low headroom. BurnCloud has not initiated a recharge. Contact your account administrator or support to arrange funding and monitor recorded spend here.",
    }
}

fn availability_attention_copy(state: &str) -> &'static str {
    match state {
        "unavailable" => "Observed Buyer requests were unsuccessful today. No automatic BurnCloud remediation is confirmed. Retry only when safe, and contact support with the request time if service remains unavailable.",
        "at_risk" => "A significant share of observed Buyer requests failed today. No automatic BurnCloud remediation is confirmed. Retry affected requests when safe and contact support if failures continue.",
        _ => "Some observed Buyer requests failed today. No automatic BurnCloud remediation is confirmed. Retry affected requests when safe and contact support if failures continue.",
    }
}

#[component]
pub fn BuyerOverview() -> Element {
    let auth = crate::backend::use_auth();
    let token = auth.token().unwrap_or_default();
    let history_token = token.clone();
    let overview_token = token.clone();

    let mut overview_resource = use_resource(move || {
        let token = overview_token.clone();
        async move {
            if token.is_empty() {
                Err("No authenticated token".to_string())
            } else {
                buyer_overview_snapshot(&token).await
            }
        }
    });
    let mut history_resource = use_resource(move || {
        let token = history_token.clone();
        async move {
            if token.is_empty() {
                Err("No authenticated token".to_string())
            } else {
                buyer_logs(&token).await
            }
        }
    });
    let mut tokens_resource = use_resource(move || async move { TokenService::list().await });

    let overview_result = overview_resource.read().clone();
    let history_result = history_resource.read().clone();
    let tokens_result = tokens_resource.read().clone();
    let overview: Option<BuyerOverviewSnapshot> = overview_result.clone().and_then(Result::ok);
    let metrics_loading = overview_result.is_none();
    let overview_failed = source_error(&overview_result);
    let model_state = model_module_state(overview_result.as_ref());
    let activity_state = activity_module_state(overview_result.as_ref());
    let tokens_loading = tokens_result.is_none();
    let history_failed = source_error(&history_result);
    let tokens_failed = source_error(&tokens_result);
    let api_key_readiness = derive_api_key_readiness(tokens_result.as_ref());
    let request_history = derive_request_history(history_result.as_ref());
    let conclusion = derive_overview_conclusion(api_key_readiness, overview.as_ref());
    let no_recorded_usage = request_history == RequestHistoryState::Empty;
    let no_api_keys = api_key_readiness == ApiKeyReadiness::Missing;
    let non_active_api_keys = api_key_readiness == ApiKeyReadiness::NonActive;
    let balance_needs_action = overview.as_ref().is_some_and(|snapshot| {
        matches!(
            snapshot.balance_state.as_str(),
            "low" | "critical" | "exhausted"
        )
    });
    let availability_needs_attention = overview.as_ref().is_some_and(|snapshot| {
        matches!(
            snapshot.api_availability.as_str(),
            "degraded" | "at_risk" | "unavailable"
        )
    });
    let balance_state_label = overview
        .as_ref()
        .map(|snapshot| display_state(&snapshot.balance_state))
        .unwrap_or_else(|| "Unknown".to_string());
    let balance_attention = overview
        .as_ref()
        .map(|snapshot| balance_attention_copy(&snapshot.balance_state))
        .unwrap_or(
            "Balance impact is unavailable. Contact support before assuming requests can continue.",
        );
    let availability_attention = overview
        .as_ref()
        .map(|snapshot| availability_attention_copy(&snapshot.api_availability))
        .unwrap_or("Service impact and BurnCloud action are unavailable. Contact support if requests are failing.");

    let (spend_value, spend_note) = spend_metric(overview.as_ref(), no_recorded_usage);
    let (balance_value, balance_note) = overview
        .as_ref()
        .map(|snapshot| {
            (
                format_balance(snapshot),
                format!("{} balance", display_state(&snapshot.balance_state)),
            )
        })
        .unwrap_or_else(|| {
            (
                "Unavailable".to_string(),
                "Current balance could not be confirmed".to_string(),
            )
        });
    let (availability_value, availability_note) = overview
        .as_ref()
        .map(|snapshot| {
            let note = snapshot
                .availability_percent
                .map(|percent| {
                    format!(
                        "{percent:.2}% · {} requests today",
                        snapshot.availability_sample_requests
                    )
                })
                .unwrap_or_else(|| "No Buyer requests observed today".to_string());
            (display_state(&snapshot.api_availability), note)
        })
        .unwrap_or_else(|| {
            (
                "Unknown".to_string(),
                "Buyer service status could not be confirmed".to_string(),
            )
        });
    let (tokens_value, tokens_note) = tokens_metric(overview.as_ref(), no_recorded_usage);

    rsx! {
        div { class: "page buyer-overview",
            div { class: "page-header buyer-overview-header",
                div {
                    h2 { class: "page-title", "Overview" }
                    p { class: "page-subtitle", "Your account and API usage at a glance." }
                }
                if balance_needs_action {
                    Link { class: "button button-primary button-sm", to: Route::BuyerBilling {}, Icon { name: "billing" } "Request funding" }
                } else if no_api_keys || non_active_api_keys {
                    Link { class: "button button-primary button-sm", to: Route::BuyerAPIKeys {}, Icon { name: "key" } if no_api_keys { "Create API key" } else { "Review API keys" } }
                } else if availability_needs_attention {
                    Link { class: "button button-primary button-sm", to: Route::BuyerLogs {}, Icon { name: "logs" } "Review requests" }
                } else {
                    Link { class: "button button-primary button-sm", to: Route::BuyerMarketplace {}, Icon { name: "models" } "Open Marketplace" }
                }
            }

            section { class: "card product-status-card buyer-overview-conclusion {conclusion_class(conclusion)}",
                span { class: "badge badge-neutral", "{conclusion_badge(conclusion)}" }
                div {
                    h3 { class: "product-status-title", "{conclusion_title(conclusion)}" }
                    p { class: "product-status-copy", "{conclusion_copy(conclusion)}" }
                }
            }

            section {
                class: "metrics buyer-overview-metrics",
                aria_label: if metrics_loading { "Loading Buyer account metrics" } else { "Buyer account metrics" },
                aria_busy: metrics_loading,
                role: "status",
                {metric("Today Spend", spend_value, spend_note, "dollar", "tone-gray", metrics_loading)}
                {metric("Balance", balance_value, balance_note, "billing", "tone-gray", metrics_loading)}
                {metric("API Availability", availability_value, availability_note, "wifi", "tone-gray", metrics_loading)}
                {metric("Tokens Today", tokens_value, tokens_note, "models", "tone-gray", metrics_loading)}
            }

            if overview_failed {
                section { class: "buyer-overview-module-state buyer-overview-module-error", role: "alert",
                    div {
                        strong { "Buyer account metrics unavailable" }
                        p { "Daily usage, balance, and service availability could not be loaded. No healthy state has been inferred." }
                    }
                    button { class: "button button-secondary button-sm buyer-overview-retry", aria_label: "Retry Buyer metrics", onclick: move |_| overview_resource.restart(), Icon { name: "activity" } "Retry metrics" }
                }
            }

            if no_api_keys || non_active_api_keys || balance_needs_action || availability_needs_attention {
                section { class: "buyer-overview-attention", aria_label: "Needs attention",
                    div { class: "product-section-head",
                        div {
                            h3 { "Needs attention" }
                            p { "Confirmed account and service conditions that may require action." }
                        }
                    }
                    if no_api_keys || non_active_api_keys {
                        div { class: "buyer-overview-attention-item", role: "alert",
                            Icon { name: "key" }
                            div {
                                strong { if no_api_keys { "No API key found" } else { "API key is not active" } }
                                p {
                                    if no_api_keys { "Requests cannot authenticate without a key. BurnCloud has not created one automatically. Open API Keys to create a credential before sending requests." }
                                    else { "Requests may not authenticate with a non-active key. BurnCloud has not changed its status automatically. Open API Keys to review or replace the credential before sending requests." }
                                }
                            }
                        }
                    }
                    if balance_needs_action {
                        div { class: "buyer-overview-attention-item", role: "alert",
                            Icon { name: "billing" }
                            div {
                                strong { "{balance_state_label} balance" }
                                p { "{balance_attention}" }
                            }
                        }
                    }
                    if availability_needs_attention {
                        div { class: "buyer-overview-attention-item", role: "alert",
                            Icon { name: "wifi" }
                            div {
                                strong { "API service needs attention" }
                                p { "{availability_attention}" }
                            }
                        }
                    }
                }
            } else if tokens_loading {
                section { class: "buyer-overview-module-state buyer-overview-token-state", aria_label: "Checking API keys",
                    div {
                        strong { "Checking API keys" }
                        p { "Needs Attention will appear only if the account key list confirms an issue." }
                    }
                }
            } else if tokens_failed {
                section { class: "buyer-overview-module-state buyer-overview-module-error buyer-overview-token-state", role: "alert",
                    div {
                        strong { "API key status unavailable" }
                        p { "The account key list could not be loaded. No key issue has been inferred." }
                    }
                    button { class: "button button-secondary button-sm buyer-overview-retry", aria_label: "Retry API key status", onclick: move |_| tokens_resource.restart(), Icon { name: "activity" } "Retry API keys" }
                }
            }

            if no_recorded_usage {
                section { class: "card buyer-overview-setup",
                    div { class: "buyer-overview-setup-copy",
                        span { class: "badge badge-neutral", "GET STARTED" }
                        h3 { "Welcome to BurnCloud" }
                        p { "Request history confirms that this account has not sent a request. Other setup status remains independent." }
                    }
                    ol { class: "buyer-overview-setup-steps",
                        li { span { "1" } div { strong { "Confirm funding" } small { "{balance_state_label}; contact your account administrator or support if funding is required" } } }
                        li { span { "2" } div { strong { "Choose a model" } small { "Model selection not confirmed" } } }
                        li { span { "3" } div { strong { "Create an API key" } small { if no_api_keys { "No key found; open API Keys to create one" } else if non_active_api_keys { "Open API Keys to review the non-active credential" } else if api_key_readiness == ApiKeyReadiness::Ready { "Active API key confirmed" } else { "Key readiness not confirmed" } } } }
                        li { span { "4" } div { strong { "Send your first request" } small { "No request has been recorded" } } }
                    }
                }
            }

            section { class: "card card-pad buyer-overview-models",
                div { class: "product-section-head",
                    div { h3 { "Models in use" } p { "Sanitized model activity recorded in UTC today." } }
                    Link { class: "button button-secondary button-sm", to: Route::BuyerMarketplace {}, "Browse Marketplace" }
                }
                if model_state == SnapshotModuleState::Partial {
                    div { class: "product-note", "Some model sources are unavailable; confirmed model rows remain visible." }
                }
                if model_state == SnapshotModuleState::Loading {
                    div {
                        class: "buyer-overview-model-list",
                        aria_label: "Loading models in use",
                        aria_busy: "true",
                        role: "status",
                        for _ in 0..3 { div { class: "buyer-overview-model-row", div { class: "buyer-overview-skeleton buyer-overview-skeleton-model" } div { class: "buyer-overview-skeleton buyer-overview-skeleton-detail" } } }
                    }
                } else if model_state == SnapshotModuleState::Unavailable {
                    div { class: "buyer-overview-module-state buyer-overview-module-error", role: "alert",
                        div { strong { "Model usage unavailable" } p { "Today's Buyer model summary could not be confirmed." } }
                        button { class: "button button-secondary button-sm buyer-overview-retry", onclick: move |_| overview_resource.restart(), "Retry" }
                    }
                } else if let Some(snapshot) = overview.as_ref() {
                    if model_state == SnapshotModuleState::Partial && snapshot.models_today.is_empty() {
                        div { class: "buyer-overview-module-state buyer-overview-module-error", role: "alert",
                            div { strong { "Model usage could not be fully confirmed" } p { "Some model sources are unavailable, so no-data status cannot be confirmed." } }
                            button { class: "button button-secondary button-sm buyer-overview-retry", onclick: move |_| overview_resource.restart(), "Retry" }
                        }
                    } else if snapshot.models_today.is_empty() {
                        div { class: "buyer-overview-module-state", div { strong { "No model usage today" } p { "Models appear after Buyer requests are recorded." } } }
                    } else {
                        div { class: "buyer-overview-model-list",
                            for model in snapshot.models_today.iter().take(5) {
                                Link { class: "buyer-overview-model-row", to: Route::BuyerMarketplace {}, key: "{model.name}",
                                    div { class: "buyer-overview-model-name", strong { "{model.name}" } span { "{model_tier_label(&model.tier)}" } }
                                    div { class: "buyer-overview-model-usage", strong { {format!("{} tokens", compact(model.tokens_today))} } span { "Today UTC · {display_state(&model.status)}" } }
                                }
                            }
                        }
                    }
                }
            }

            section { class: "card card-pad buyer-overview-activity",
                div { class: "product-section-head",
                    div { h3 { "Recent activity" } p { "Recorded Buyer account activity." } }
                    Link { class: "button button-secondary button-sm", to: Route::BuyerLogs {}, "Open Logs" }
                }
                if activity_state == SnapshotModuleState::Partial {
                    div { class: "product-note", "Some activity sources are unavailable; confirmed events remain visible." }
                }
                if activity_state == SnapshotModuleState::Loading {
                    div {
                        class: "buyer-overview-model-list",
                        aria_label: "Loading recent activity",
                        aria_busy: "true",
                        role: "status",
                        for _ in 0..3 { div { class: "buyer-overview-model-row", div { class: "buyer-overview-skeleton buyer-overview-skeleton-model" } div { class: "buyer-overview-skeleton buyer-overview-skeleton-detail" } } }
                    }
                } else if activity_state == SnapshotModuleState::Unavailable {
                    div { class: "buyer-overview-module-state buyer-overview-module-error", role: "alert", div { strong { "Recent activity unavailable" } p { "Buyer activity could not be loaded." } } }
                } else if let Some(snapshot) = overview.as_ref() {
                    if activity_state == SnapshotModuleState::Partial
                        && snapshot
                            .recent_activity
                            .iter()
                            .all(|event| !activity_is_supported(&event.kind))
                    {
                        div { class: "buyer-overview-module-state buyer-overview-module-error", role: "alert",
                            div { strong { "Recent activity could not be fully confirmed" } p { "Some activity sources are unavailable, so no-activity status cannot be confirmed." } }
                            button { class: "button button-secondary button-sm buyer-overview-retry", onclick: move |_| overview_resource.restart(), "Retry" }
                        }
                    } else if snapshot
                        .recent_activity
                        .iter()
                        .all(|event| !activity_is_supported(&event.kind))
                    {
                        div { class: "buyer-overview-module-state", div { strong { "No recent activity" } p { "Recorded recharges and API key creation appear here." } } }
                    } else {
                        div { class: "buyer-overview-model-list",
                            for event in snapshot
                                .recent_activity
                                .iter()
                                .filter(|event| activity_is_supported(&event.kind))
                                .take(6)
                            {
                                div { class: "buyer-overview-model-row", key: "{event.kind}-{event.occurred_at_utc}",
                                    div { class: "buyer-overview-model-name", strong { "{event.title}" } span { "{event.detail}" } }
                                    div { class: "buyer-overview-model-usage", strong { "{display_state(&event.kind)}" } span { "{event.occurred_at_utc}" } }
                                }
                            }
                        }
                    }
                }
            }

            if history_failed {
                section { class: "buyer-overview-module-state buyer-overview-module-error buyer-overview-usage-error", role: "alert",
                    div {
                        strong { "Setup status unavailable" }
                        p { "Request history could not be loaded. Other overview information remains available." }
                    }
                    button { class: "button button-secondary button-sm buyer-overview-retry", aria_label: "Retry request history", onclick: move |_| history_resource.restart(), Icon { name: "activity" } "Retry history" }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::TokenDto;

    fn key(status: &str) -> TokenDto {
        TokenDto {
            status: status.to_string(),
            ..TokenDto::default()
        }
    }

    #[test]
    fn api_key_readiness_distinguishes_loading_failure_and_empty() {
        assert_eq!(derive_api_key_readiness(None), ApiKeyReadiness::Loading);
        assert_eq!(
            derive_api_key_readiness(Some(&Err("failed".into()))),
            ApiKeyReadiness::Unknown
        );
        assert_eq!(
            derive_api_key_readiness(Some(&Ok(Vec::new()))),
            ApiKeyReadiness::Missing
        );
    }

    #[test]
    fn api_key_readiness_requires_active_keys() {
        assert_eq!(
            derive_api_key_readiness(Some(&Ok(vec![key("disabled")]))),
            ApiKeyReadiness::NonActive
        );
        assert_eq!(
            derive_api_key_readiness(Some(&Ok(vec![key("ACTIVE")]))),
            ApiKeyReadiness::Ready
        );
        assert_eq!(
            derive_api_key_readiness(Some(&Ok(vec![key("disabled"), key("active")]))),
            ApiKeyReadiness::Ready
        );
    }

    #[test]
    fn request_history_distinguishes_loading_failure_empty_and_present() {
        assert_eq!(derive_request_history(None), RequestHistoryState::Loading);
        assert_eq!(
            derive_request_history(Some(&Err("failed".into()))),
            RequestHistoryState::Unknown
        );
        assert_eq!(
            derive_request_history(Some(&Ok(Vec::new()))),
            RequestHistoryState::Empty
        );
        assert_eq!(
            derive_request_history(Some(&Ok(vec![BuyerLogSummary::default()]))),
            RequestHistoryState::Present
        );
    }

    fn snapshot(balance_state: &str, availability: &str) -> BuyerOverviewSnapshot {
        BuyerOverviewSnapshot {
            balance_state: balance_state.to_string(),
            api_availability: availability.to_string(),
            today_spend_usd: Some(0.0),
            tokens_today: Some(0),
            ..BuyerOverviewSnapshot::default()
        }
    }

    #[test]
    fn conclusion_uses_confirmed_account_and_service_state() {
        assert_eq!(
            derive_overview_conclusion(ApiKeyReadiness::Ready, None),
            OverviewConclusion::Unknown
        );
        assert_eq!(
            derive_overview_conclusion(
                ApiKeyReadiness::Ready,
                Some(&snapshot("healthy", "healthy")),
            ),
            OverviewConclusion::Healthy
        );
        assert_eq!(
            derive_overview_conclusion(ApiKeyReadiness::Ready, Some(&snapshot("low", "healthy")),),
            OverviewConclusion::NeedsAttention
        );
        assert_eq!(
            derive_overview_conclusion(
                ApiKeyReadiness::Ready,
                Some(&snapshot("healthy", "unknown")),
            ),
            OverviewConclusion::Unknown
        );
    }

    #[test]
    fn conclusion_stays_unknown_when_daily_usage_is_partial() {
        let mut missing_spend = snapshot("healthy", "healthy");
        missing_spend.today_spend_usd = None;
        assert_eq!(
            derive_overview_conclusion(ApiKeyReadiness::Ready, Some(&missing_spend)),
            OverviewConclusion::Unknown
        );

        let mut reported_issue = snapshot("healthy", "healthy");
        reported_issue
            .issues
            .push("daily_usage_unavailable".to_string());
        assert_eq!(
            derive_overview_conclusion(ApiKeyReadiness::Ready, Some(&reported_issue)),
            OverviewConclusion::Unknown
        );
    }

    #[test]
    fn conclusion_requires_confirmed_ready_key_for_healthy_state() {
        let healthy_snapshot = snapshot("healthy", "healthy");
        assert_eq!(
            derive_overview_conclusion(ApiKeyReadiness::Loading, Some(&healthy_snapshot)),
            OverviewConclusion::Unknown
        );
        assert_eq!(
            derive_overview_conclusion(ApiKeyReadiness::Unknown, Some(&healthy_snapshot)),
            OverviewConclusion::Unknown
        );
    }

    #[test]
    fn attention_copy_states_impact_action_and_next_step() {
        let exhausted = balance_attention_copy("exhausted");
        assert!(exhausted.contains("rejected"));
        assert!(exhausted.contains("not confirmed"));
        assert!(exhausted.contains("Contact"));

        let unavailable = availability_attention_copy("unavailable");
        assert!(unavailable.contains("unsuccessful"));
        assert!(unavailable.contains("No automatic"));
        assert!(unavailable.contains("contact support"));
    }

    #[test]
    fn unsupported_tier_and_synthetic_recovery_are_not_presented() {
        assert_eq!(model_tier_label("supplier-premium"), "Tier unavailable");
        assert_eq!(model_tier_label("Historical"), "Tier unavailable");
        assert!(!activity_is_supported("service_recovery"));
        assert!(!activity_is_supported("service_incident"));
        assert!(!activity_is_supported("model_usage"));
        assert!(!activity_is_supported("balance_warning"));
        assert!(activity_is_supported("api_key_created"));
        assert!(activity_is_supported("recharge"));
    }

    #[test]
    fn conclusion_prioritizes_confirmed_balance_risk_over_key_setup_gaps() {
        for balance_state in ["low", "critical", "exhausted"] {
            assert_eq!(
                derive_overview_conclusion(
                    ApiKeyReadiness::NonActive,
                    Some(&snapshot(balance_state, "healthy")),
                ),
                OverviewConclusion::NeedsAttention
            );
        }
        assert_eq!(
            derive_overview_conclusion(
                ApiKeyReadiness::NonActive,
                Some(&snapshot("healthy", "healthy")),
            ),
            OverviewConclusion::SetupRequired
        );
        assert_eq!(
            derive_overview_conclusion(ApiKeyReadiness::Missing, None),
            OverviewConclusion::SetupRequired
        );
    }

    #[test]
    fn confirmed_new_buyer_metrics_do_not_render_meaningless_usage_zeroes() {
        let mut snapshot = snapshot("exhausted", "unknown");
        snapshot.as_of_utc = "2026-08-22".to_string();

        assert_eq!(
            spend_metric(Some(&snapshot), true),
            (
                "No spend yet".to_string(),
                "No requests recorded today".to_string()
            )
        );
        assert_eq!(
            tokens_metric(Some(&snapshot), true),
            (
                "No usage yet".to_string(),
                "No requests recorded today".to_string()
            )
        );
        assert_eq!(spend_metric(Some(&snapshot), false).0, "$0.0000");
        assert_eq!(tokens_metric(Some(&snapshot), false).0, "0");
    }
}
