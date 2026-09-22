use super::{
    actions::{spend_progress_percent, CreateKeyError},
    model::{ApiKeyStatus, BuyerApiKey},
    state::ApiKeysState,
};
use crate::{
    i18n::{strings, Locale, LocaleStrings},
    shared::{
        layout::BuyerShell,
        ui::{Badge, Icon, IconName},
    },
};
use dioxus::prelude::*;

fn validation_message(error: CreateKeyError, copy: &LocaleStrings) -> &'static str {
    match error {
        CreateKeyError::NameRequired => copy.api_keys_name_required,
        CreateKeyError::RateLimitOutOfRange => copy.api_keys_rate_limit_range,
        CreateKeyError::MonthlySpendCapOutOfRange => copy.api_keys_spend_limit_range,
        CreateKeyError::InvalidSecret => copy.status_unavailable,
    }
}

fn format_cap(value: f64) -> String {
    let formatted = format!("{value:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

#[component]
fn ApiKeyRow(item: BuyerApiKey, mut state: Signal<ApiKeysState>) -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let progress = spend_progress_percent(item.spend_this_month, item.monthly_spend_cap);
    let row_class = if item.status == ApiKeyStatus::Revoked {
        "revoked"
    } else {
        ""
    };
    let key_id = item.id.clone();

    rsx! {
        tr { class: row_class,
            td {
                div { class: "api-key-name",
                    div { class: "api-key-name-label", {item.name} }
                    div { class: "api-key-name-created", {format!("{}: {}", copy.api_keys_created_at, item.created)} }
                }
            }
            td { code { class: "api-key-secret", {item.masked_key} } }
            td { span { class: "api-key-rate", {format!("{} req/min", item.rate_limit_rpm)} } }
            td {
                div { class: "api-key-spend",
                    strong { {format!("${:.2} / ${}", item.spend_this_month, format_cap(item.monthly_spend_cap))} }
                    span { class: "api-key-progress", aria_hidden: "true",
                        span { style: format!("width: {progress:.3}%") }
                    }
                }
            }
            td { span { class: "api-key-last-used", {item.last_used} } }
            td {
                if item.status == ApiKeyStatus::Active {
                    Badge { label: copy.active.to_string(), tone: "success".to_string() }
                } else {
                    Badge { label: copy.api_keys_revoked.to_string(), tone: "error".to_string() }
                }
            }
            td { class: "align-right",
                if item.status == ApiKeyStatus::Active {
                    button {
                        r#type: "button",
                        class: "api-key-revoke",
                        onclick: move |_| { state.with_mut(|value| value.revoke_key(&key_id)); },
                        {copy.api_keys_revoke}
                    }
                }
            }
        }
    }
}

#[component]
fn CreateKeyModal(mut state: Signal<ApiKeysState>) -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let snapshot = state.read().clone();
    let validation_error = snapshot.validation_error;

    rsx! {
        div { class: "api-key-modal-layer",
            button {
                r#type: "button",
                class: "api-key-modal-backdrop",
                aria_label: copy.close,
                onclick: move |_| state.with_mut(ApiKeysState::close_create_modal)
            }
            section {
                class: "api-key-modal",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "create-api-key-title",
                header { class: "api-key-modal-header",
                    div {
                        h2 { id: "create-api-key-title", {copy.api_keys_modal_title} }
                        p { {copy.api_keys_modal_description} }
                    }
                    button {
                        r#type: "button",
                        class: "icon-button api-key-modal-close",
                        title: copy.close,
                        aria_label: copy.close,
                        onclick: move |_| state.with_mut(ApiKeysState::close_create_modal),
                        Icon { name: IconName::X, size: 16 }
                    }
                }
                form {
                    class: "api-key-modal-body",
                    onsubmit: move |event| {
                        event.prevent_default();
                        state.with_mut(|value| { let _ = value.create_key(); });
                    },
                    div { class: "api-key-field",
                        label { r#for: "api-key-name", {copy.api_keys_name_label} }
                        input {
                            id: "api-key-name",
                            name: "name",
                            r#type: "text",
                            value: snapshot.new_key_name,
                            placeholder: copy.api_keys_name_placeholder,
                            required: true,
                            maxlength: "100",
                            autocomplete: "off",
                            autofocus: true,
                            oninput: move |event| state.with_mut(|value| value.new_key_name = event.value())
                        }
                        if validation_error == Some(CreateKeyError::NameRequired) {
                            p { class: "api-key-field-error", role: "alert", {validation_message(CreateKeyError::NameRequired, copy)} }
                        }
                    }
                    div { class: "api-key-form-grid",
                        div { class: "api-key-field",
                            label { r#for: "api-key-rate", {copy.api_keys_col_rate_limit} }
                            input {
                                id: "api-key-rate",
                                name: "rate",
                                r#type: "number",
                                value: snapshot.new_key_rate_limit.to_string(),
                                min: "10",
                                max: "5000",
                                required: true,
                                oninput: move |event| if let Ok(rate) = event.value().parse::<u32>() { state.with_mut(|value| value.new_key_rate_limit = rate); }
                            }
                            if validation_error == Some(CreateKeyError::RateLimitOutOfRange) {
                                p { class: "api-key-field-error", role: "alert", {validation_message(CreateKeyError::RateLimitOutOfRange, copy)} }
                            }
                        }
                        div { class: "api-key-field",
                            label { r#for: "api-key-cap", {copy.api_keys_spend_limit} }
                            input {
                                id: "api-key-cap",
                                name: "cap",
                                r#type: "number",
                                value: format_cap(snapshot.new_key_cap),
                                min: "10",
                                max: "50000",
                                step: "0.01",
                                required: true,
                                oninput: move |event| if let Ok(cap) = event.value().parse::<f64>() { state.with_mut(|value| value.new_key_cap = cap); }
                            }
                            if validation_error == Some(CreateKeyError::MonthlySpendCapOutOfRange) {
                                p { class: "api-key-field-error", role: "alert", {validation_message(CreateKeyError::MonthlySpendCapOutOfRange, copy)} }
                            }
                        }
                    }
                    div { class: "api-key-modal-actions",
                        button {
                            r#type: "button",
                            class: "api-key-action-button secondary",
                            onclick: move |_| state.with_mut(ApiKeysState::close_create_modal),
                            {copy.api_keys_cancel}
                        }
                        button { r#type: "submit", class: "api-key-action-button primary", {copy.api_keys_create} }
                    }
                }
            }
        }
    }
}

#[component]
fn KeyResultModal(mut state: Signal<ApiKeysState>) -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let snapshot = state.read().clone();
    let secret = snapshot.created_secret.unwrap_or_default();
    let secret_to_copy = secret.clone();

    rsx! {
        div { class: "api-key-modal-layer",
            button {
                r#type: "button",
                class: "api-key-modal-backdrop",
                aria_label: copy.close,
                onclick: move |_| state.with_mut(ApiKeysState::close_result_modal)
            }
            section {
                class: "api-key-modal",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "api-key-result-title",
                header { class: "api-key-modal-header",
                    div {
                        h2 { id: "api-key-result-title", {copy.api_keys_modal_title} }
                        p { {copy.api_keys_secret_notice} }
                    }
                    button {
                        r#type: "button",
                        class: "icon-button api-key-modal-close",
                        title: copy.close,
                        aria_label: copy.close,
                        onclick: move |_| state.with_mut(ApiKeysState::close_result_modal),
                        Icon { name: IconName::X, size: 16 }
                    }
                }
                div { class: "api-key-modal-body",
                    div { class: "api-key-warning", role: "alert",
                        Icon { name: IconName::AlertCircle, size: 16 }
                        p { {copy.api_keys_secret_notice} }
                    }
                    div { class: "api-key-secret-result",
                        code { {secret} }
                        button {
                            r#type: "button",
                            class: if snapshot.copied { "api-key-copy-button copied" } else { "api-key-copy-button" },
                            onclick: move |_| {
                                let script = format!("navigator.clipboard?.writeText({secret_to_copy:?});");
                                dioxus::document::eval(&script);
                                state.with_mut(ApiKeysState::mark_copied);
                                let mut copied_state = state;
                                spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                    copied_state.with_mut(ApiKeysState::clear_copied);
                                });
                            },
                            Icon { name: if snapshot.copied { IconName::Check } else { IconName::Copy }, size: 12 }
                            span { if snapshot.copied { {copy.api_keys_copied} } else { {copy.api_keys_copy} } }
                        }
                    }
                    div { class: "api-key-result-actions",
                        button {
                            r#type: "button",
                            class: "api-key-action-button primary",
                            onclick: move |_| state.with_mut(ApiKeysState::close_result_modal),
                            {copy.api_keys_copy_and_close}
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn BuyerApiKeys() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(ApiKeysState::default);
    let snapshot = state.read().clone();
    let active_count = snapshot.active_count();

    rsx! {
        BuyerShell {
            div { class: "api-keys-stack",
                div { class: "api-keys-header-block",
                    section { class: "page-header api-keys-page-header",
                        div { class: "page-heading-copy",
                            h1 { {copy.api_keys_title} }
                            p { {copy.api_keys_subtitle} }
                        }
                        div { class: "page-actions",
                            button {
                                r#type: "button",
                                class: "api-keys-create-button",
                                onclick: move |_| state.with_mut(ApiKeysState::open_create_modal),
                                Icon { name: IconName::Plus, size: 14 }
                                span { {copy.api_keys_create} }
                            }
                        }
                    }
                    div { class: "conclusion conclusion-healthy api-keys-conclusion", role: "status",
                        Icon { name: IconName::CheckCircle, size: 16 }
                        span { class: "conclusion-text", {copy.api_keys_conclusion} }
                    }
                }
                section { class: "panel api-keys-panel",
                    div { class: "section-header",
                        div {
                            h2 { {copy.api_keys_title} }
                            p { {copy.api_keys_subtitle} }
                        }
                        span { class: "api-keys-count", title: copy.api_keys_active_keys,
                            strong { {active_count.to_string()} }
                            " "
                            {copy.active}
                        }
                    }
                    if snapshot.keys.is_empty() {
                        div { class: "api-keys-empty",
                            span { class: "api-keys-empty-icon", Icon { name: IconName::Key, size: 19 } }
                            h3 { {copy.api_keys_empty_title} }
                            p { {copy.api_keys_empty_description} }
                            button {
                                r#type: "button",
                                class: "api-keys-create-button",
                                onclick: move |_| state.with_mut(ApiKeysState::open_create_modal),
                                Icon { name: IconName::Plus, size: 14 }
                                span { {copy.api_keys_create} }
                            }
                        }
                    } else {
                        div { class: "api-keys-table-scroll",
                            table { class: "api-keys-table",
                                thead { tr {
                                    th { {copy.api_keys_col_name} }
                                    th { {copy.api_keys_col_secret} }
                                    th { {copy.api_keys_col_rate_limit} }
                                    th { {copy.api_keys_col_spend_cap} }
                                    th { {copy.api_keys_col_last_used} }
                                    th { {copy.api_keys_col_status} }
                                    th { class: "align-right", {copy.api_keys_col_actions} }
                                } }
                                tbody {
                                    for key in snapshot.keys.clone() {
                                        ApiKeyRow { item: key, state }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if snapshot.create_modal_open {
                CreateKeyModal { state }
            }
            if snapshot.result_modal_open {
                KeyResultModal { state }
            }
        }
    }
}
