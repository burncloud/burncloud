use super::state::{BalanceState, OverviewState};
use crate::{
    i18n::{
        formatter::{currency, integer, tokens},
        strings, Locale,
    },
    shared::{
        layout::BuyerShell,
        ui::{Badge, Button, Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

fn model_status(
    status: super::model::ServiceStatus,
    copy: &crate::i18n::LocaleStrings,
) -> (&'static str, &'static str) {
    match status {
        super::model::ServiceStatus::Healthy => (copy.healthy_short, "healthy"),
        super::model::ServiceStatus::Degraded => (copy.degraded, "warning"),
        super::model::ServiceStatus::Unknown => (copy.status_unavailable, "neutral"),
    }
}

fn status_cell(status: super::model::ServiceStatus, copy: &crate::i18n::LocaleStrings) -> Element {
    let (label, tone) = model_status(status, copy);
    match tone {
        "healthy" => {
            rsx! { span { class: "status", span { class: "status-dot" } span { class: "status-label", {label} } } }
        }
        "warning" => {
            rsx! { span { class: "status status-warning", span { class: "status-dot" } span { class: "status-label", {label} } } }
        }
        _ => {
            rsx! { span { class: "status status-neutral", span { class: "status-label", {label} } } }
        }
    }
}

#[component]
pub fn BuyerOverview() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let state = use_signal(OverviewState::default);
    let snapshot = state.read().clone();
    let balance_state = snapshot.balance_state;
    let balance = snapshot.model.prepaid_balance;
    let routes_known = snapshot.model.api_availability.is_some()
        && snapshot
            .model
            .models
            .iter()
            .all(|model| !matches!(model.status, super::model::ServiceStatus::Unknown));
    let conclusion_class = if routes_known && !balance_state.needs_attention() {
        "conclusion conclusion-healthy"
    } else {
        "conclusion conclusion-warning"
    };
    let conclusion_icon = if routes_known && !balance_state.needs_attention() {
        IconName::CheckCircle
    } else {
        IconName::AlertTriangle
    };
    let conclusion = if routes_known && !balance_state.needs_attention() {
        copy.healthy
    } else if balance_state.needs_attention() {
        copy.warning
    } else {
        copy.status_unavailable
    };
    let balance_detail = match balance_state {
        BalanceState::Healthy | BalanceState::Low | BalanceState::Critical => {
            copy.metric_balance_sub
        }
        BalanceState::Unknown => copy.status_unavailable,
    };
    let balance_badge = match balance_state {
        BalanceState::Healthy => copy.healthy_badge,
        BalanceState::Low => copy.low_badge,
        BalanceState::Critical => copy.critical_badge,
        BalanceState::Unknown => copy.unknown_badge,
    };
    let balance_tone = match balance_state {
        BalanceState::Healthy => "healthy",
        BalanceState::Low | BalanceState::Critical => "warning",
        BalanceState::Unknown => "neutral",
    };
    let availability_known = snapshot.model.api_availability.is_some();
    let availability_detail = if availability_known {
        copy.metric_availability_sub
    } else {
        copy.status_unavailable
    };
    let availability_status = if availability_known {
        copy.healthy_short
    } else {
        copy.status_unavailable
    };
    let availability_tone = if availability_known {
        "healthy"
    } else {
        "neutral"
    };

    rsx! {
        BuyerShell {
            div { class: "overview-stack",
                section { class: "page-header",
                    div { class: "page-heading-copy", h1 { {copy.overview} } p { {copy.subtitle} } }
                    div { class: "page-actions", Button { label: copy.playground.to_string(), href: Some("/buyer/playground".to_string()), secondary: true, icon: Some(IconName::Terminal) } Button { label: copy.marketplace.to_string(), href: Some("/buyer/marketplace".to_string()), icon: Some(IconName::Store) } }
                }
                div { class: conclusion_class, role: "status", Icon { name: conclusion_icon, size: 16 } span { class: "conclusion-text", {conclusion} } }
                section { class: "metric-grid", aria_label: copy.overview,
                    MetricCard { label: copy.today_spend.to_string(), value: currency(snapshot.model.today_spend), detail: copy.metric_today_spend_sub.to_string(), trend: Some(copy.spend_trend.to_string()), trend_positive: false }
                    MetricCard { label: copy.prepaid_balance.to_string(), value: balance.map(currency).unwrap_or_else(|| "—".to_string()), detail: balance_detail.to_string(), badge: Some(balance_badge.to_string()), status_tone: balance_tone.to_string() }
                    MetricCard { label: copy.availability.to_string(), value: snapshot.model.api_availability.map(|value| format!("{value:.2}%")).unwrap_or_else(|| "—".to_string()), detail: availability_detail.to_string(), status: Some(availability_status.to_string()), status_tone: availability_tone.to_string() }
                    MetricCard { label: copy.tokens_today.to_string(), value: snapshot.model.tokens_today.map(tokens).unwrap_or_else(|| "—".to_string()), unit: Some("tokens".to_string()), detail: copy.metric_tokens_sub.to_string(), trend: Some(copy.tokens_trend.to_string()), trend_positive: true }
                }
                if balance_state.needs_attention() {
                    section { id: "attention", class: "attention", role: "alert",
                        div { class: "attention-copy", Icon { name: IconName::AlertTriangle, size: 20 } div { h4 { {copy.attention_title} } p { {copy.attention_description} } } }
                        Button { label: copy.top_up_now.to_string(), href: Some("/buyer/billing".to_string()), icon: Some(IconName::CreditCard), warning: true }
                    }
                }
                section { class: "panel models-panel",
                    div { class: "section-header", div { h3 { {copy.models} } p { {copy.models_description} } } a { role: "button", class: "ghost-link", href: "/buyer/marketplace", span { {copy.explore_models} } Icon { name: IconName::ArrowRight, size: 14 } } }
                    div { class: "table-scroll", table {
                        thead { tr { th { {copy.model} } th { {copy.tier} } th { {copy.tokens_column} } th { {copy.latency} } th { {copy.today_cost} } th { {copy.service_status} } th { class: "align-right", {copy.action} } } }
                        tbody { for item in snapshot.model.models.iter() {
                            tr {
                                td { div { class: "model-cell", span { class: "model-mark", {item.name.chars().next().unwrap_or('M').to_string()} } span { strong { {item.name} } small { {item.family} } } } }
                                td { Badge { label: item.tier.to_string(), tone: if item.tier == "Performance" { "accent".to_string() } else { "neutral".to_string() } } }
                                td { class: "mono strong", {integer(item.tokens)} }
                                td { class: "mono", {format!("{} ms", item.latency_ms)} }
                                td { class: "mono strong", {currency(item.cost)} }
                                td { {status_cell(item.status, copy)} }
                                td { class: "align-right", Link { role: "button", class: "table-link", to: "/buyer/playground", {copy.test_playground} } }
                            }
                        } }
                    } }
                }
                section { class: "panel activity-panel",
                    div { class: "section-header", div { h3 { {copy.activity} } p { {copy.activity_description} } } a { role: "button", class: "ghost-link", href: "/buyer/logs", span { {copy.view_all} } Icon { name: IconName::ArrowRight, size: 14 } } }
                    div { class: "activity-list", for (index, event) in snapshot.model.recent_activity.iter().enumerate() {
                        article { class: "activity-item",
                            div { class: "activity-main",
                                span { class: "activity-icon", Icon { name: if index == 0 { IconName::DollarSign } else if index == 1 { IconName::Zap } else { IconName::Key }, size: 14 } }
                                div { class: "activity-copy", strong { {event.title} } p { {event.detail} } }
                            }
                            time { {event.time} }
                        }
                    } }
                }
            }
        }
    }
}
