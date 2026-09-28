use super::{
    actions::{format_amount, remaining_days},
    model::{BillingInvoice, BillingModel},
    state::{BalanceState, BillingState, TopUpError, PRESET_TOP_UP_AMOUNTS},
};
use crate::{
    i18n::{formatter::currency, strings, Locale, LocaleStrings},
    shared::{
        layout::BuyerShell,
        ui::{Badge, Icon, IconName},
    },
};
use dioxus::core::spawn_isomorphic;
use dioxus::prelude::*;

fn conclusion_text(copy: &LocaleStrings, balance: Option<f64>, today_spend: f64) -> String {
    let Some(balance) = balance.filter(|value| value.is_finite()) else {
        return copy.status_unavailable.to_string();
    };
    let Some(days) = remaining_days(Some(balance), today_spend) else {
        return copy.status_unavailable.to_string();
    };
    copy.billing_conclusion_template
        .replace("{balance}", &currency(balance))
        .replace("{rate}", &currency(today_spend))
        .replace("{days}", &days.to_string())
}

fn balance_tone(state: BalanceState) -> &'static str {
    if state.is_alert() {
        "conclusion conclusion-warning billing-conclusion"
    } else {
        "conclusion conclusion-healthy billing-conclusion"
    }
}

fn balance_icon(state: BalanceState) -> IconName {
    if state.is_alert() {
        IconName::AlertTriangle
    } else {
        IconName::CheckCircle
    }
}

fn validation_message(error: TopUpError, copy: &LocaleStrings) -> &'static str {
    match error {
        TopUpError::BelowMinimum | TopUpError::InvalidAmount => copy.billing_minimum_amount,
        TopUpError::Unavailable => copy.status_unavailable,
    }
}

fn invoice_amount(invoice: BillingInvoice) -> String {
    currency(invoice.amount)
}

#[component]
fn TopUpModal(mut state: Signal<BillingState>, model: BillingModel) -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let snapshot = state.read().clone();
    let amount = snapshot.amount_for_display();
    let amount_text = if amount.is_finite() && amount > 0.0 {
        format_amount(amount)
    } else {
        format_amount(0.0)
    };
    let validation_error = snapshot.validation_error;

    rsx! {
        div { class: "billing-modal-layer",
            button {
                r#type: "button",
                class: "billing-modal-backdrop",
                aria_label: copy.close,
                onclick: move |_| state.with_mut(BillingState::close_modal)
            }
            section {
                class: "billing-modal",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "billing-modal-title",
                header { class: "billing-modal-header",
                    div {
                        h2 { id: "billing-modal-title", {copy.billing_modal_title} }
                        p { {copy.billing_modal_subtitle} }
                    }
                    button {
                        r#type: "button",
                        class: "icon-button billing-modal-close",
                        title: copy.close,
                        aria_label: copy.close,
                        onclick: move |_| state.with_mut(BillingState::close_modal),
                        Icon { name: IconName::X, size: 16 }
                    }
                }
                form {
                    class: "billing-modal-body",
                    onsubmit: move |event| {
                        event.prevent_default();
                        let result = state.with_mut(BillingState::submit_top_up);
                        if let Ok(amount) = result {
                            let message = format!(
                                "{}{}{}",
                                copy.billing_success_prefix,
                                format_amount(amount),
                                copy.billing_success_suffix
                            );
                            state.with_mut(|value| value.set_success_message(message));
                        }
                    },
                    div { class: "billing-field",
                        label { {copy.billing_select_amount} }
                        div { class: "billing-preset-grid",
                            for preset in PRESET_TOP_UP_AMOUNTS {
                                button {
                                    r#type: "button",
                                    class: if snapshot.custom_amount.is_empty() && (snapshot.selected_amount - preset).abs() < f64::EPSILON { "billing-preset selected" } else { "billing-preset" },
                                    onclick: move |_| state.with_mut(|value| value.select_preset(preset)),
                                    {format_amount(preset)}
                                }
                            }
                        }
                    }
                    div { class: "billing-field",
                        label { r#for: "billing-custom-amount", {copy.billing_custom_amount} }
                        input {
                            id: "billing-custom-amount",
                            r#type: "number",
                            min: "10",
                            step: "0.01",
                            placeholder: "e.g. 1500",
                            value: snapshot.custom_amount.clone(),
                            oninput: move |event| state.with_mut(|value| value.set_custom_amount(event.value()))
                        }
                    }
                    div { class: "billing-payment-summary",
                        div { span { {format!("{}:", copy.billing_payment_method)} } strong { {model.payment_method} } }
                        div { span { {format!("{}:", copy.billing_instant_credit)} } strong { class: "billing-credit-amount", {amount_text} } }
                    }
                    if let Some(error) = validation_error {
                        p { class: "billing-field-error", role: "alert", {validation_message(error, copy)} }
                    }
                    div { class: "billing-modal-actions",
                        button {
                            r#type: "button",
                            class: "billing-action-button secondary",
                            onclick: move |_| state.with_mut(BillingState::close_modal),
                            {copy.billing_cancel}
                        }
                        button {
                            r#type: "submit",
                            class: "billing-action-button primary",
                            disabled: !amount.is_finite() || amount < 10.0,
                            {copy.billing_confirm_recharge}
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn BuyerBilling() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(BillingState::default);
    use_effect(move || {
        let snapshot = state.read();
        if snapshot.success_message.is_none() {
            return;
        }
        let expected_version = snapshot.success_message_version;
        drop(snapshot);
        let mut toast_state = state;
        spawn_isomorphic(async move {
            tokio::time::sleep(tokio::time::Duration::from_secs(4)).await;
            toast_state.with_mut(|value| value.clear_success_message_if(expected_version));
        });
    });
    let snapshot = state.read().clone();
    let model = *BillingModel::reference();
    let balance_state = snapshot.balance_state();
    let balance = snapshot.balance;
    let balance_text = balance
        .filter(|value| value.is_finite())
        .map(currency)
        .unwrap_or_else(|| "—".to_string());
    let conclusion = conclusion_text(copy, balance, snapshot.today_spend);

    rsx! {
        BuyerShell {
            div { class: "billing-stack",
                section { class: "billing-header-block",
                    div { class: "page-header billing-page-header",
                        div { class: "page-heading-copy",
                            h1 { {copy.billing_title} }
                            p { {copy.billing_subtitle} }
                        }
                        button {
                            r#type: "button",
                            class: "billing-top-up-button",
                            title: copy.billing_top_up,
                            aria_label: copy.billing_top_up,
                            onclick: move |_| state.with_mut(BillingState::open_modal),
                            Icon { name: IconName::Plus, size: 14 }
                        }
                    }
                    div { class: balance_tone(balance_state), role: "status",
                        Icon { name: balance_icon(balance_state), size: 16 }
                        span { class: "conclusion-text", {conclusion} }
                    }
                    if let Some(message) = snapshot.success_message.clone() {
                        div { class: "billing-success", role: "status",
                            Icon { name: IconName::CheckCircle, size: 16 }
                            span { {message} }
                        }
                    }
                }
                section { class: "billing-summary-grid", aria_label: copy.billing_title,
                    article { class: "billing-card billing-balance-card",
                        div { class: "billing-card-header",
                            span { class: "billing-eyebrow", {copy.billing_prepaid_balance} }
                            Icon { name: IconName::Shield, size: 16 }
                        }
                        strong { class: "billing-balance-value", {balance_text} }
                        div { class: "billing-card-footer",
                            span { {format!("{}: ~{} / day", copy.billing_burn_rate, currency(snapshot.today_spend))} }
                            button {
                                r#type: "button",
                                class: "billing-inline-top-up",
                                title: copy.billing_top_up,
                                aria_label: copy.billing_top_up,
                                onclick: move |_| state.with_mut(BillingState::open_modal),
                                Icon { name: IconName::Plus, size: 14 }
                            }
                        }
                    }
                    article { class: "billing-card",
                        div { class: "billing-card-header",
                            span { class: "billing-eyebrow", {copy.billing_auto_recharge} }
                            Badge {
                                label: if snapshot.auto_recharge_enabled { copy.billing_enabled.to_string() } else { copy.billing_disabled.to_string() },
                                tone: if snapshot.auto_recharge_enabled { "success".to_string() } else { "neutral".to_string() }
                            }
                        }
                        strong { class: "billing-auto-rule", {copy.billing_auto_recharge_rule} }
                        div { class: "billing-card-footer billing-card-footer-muted",
                            span { {copy.billing_payment} }
                            button {
                                r#type: "button",
                                class: "billing-toggle-button",
                                aria_pressed: snapshot.auto_recharge_enabled,
                                onclick: move |_| state.with_mut(BillingState::toggle_auto_recharge),
                                {copy.billing_toggle}
                            }
                        }
                    }
                    article { class: "billing-card",
                        div { class: "billing-card-header",
                            span { class: "billing-eyebrow", {copy.billing_metering_policy} }
                            Icon { name: IconName::Zap, size: 16 }
                        }
                        p { class: "billing-metering-description", {copy.billing_metering_description} }
                        div { class: "billing-markup-note", {copy.billing_markup_note} }
                    }
                }
                section { class: "panel billing-invoices-panel",
                    div { class: "section-header billing-invoices-header",
                        div {
                            h2 { {copy.billing_invoices_title} }
                            p { {copy.billing_invoices_description} }
                        }
                    }
                    div { class: "billing-table-scroll",
                        table { class: "billing-table",
                            thead { tr {
                                th { {copy.billing_col_invoice_id} }
                                th { {copy.billing_col_date} }
                                th { {copy.billing_col_description} }
                                th { {copy.billing_col_payment_method} }
                                th { {copy.billing_col_amount} }
                                th { {copy.billing_col_status} }
                                th { class: "align-right", {copy.billing_col_receipt} }
                            } }
                            tbody {
                                for invoice in model.invoices {
                                    tr {
                                        td { class: "billing-invoice-id", {invoice.id} }
                                        td { class: "billing-date", {invoice.date} }
                                        td { class: "billing-description", {invoice.description} }
                                        td { class: "billing-payment-method", {invoice.payment_method} }
                                        td { class: "billing-amount", {invoice_amount(invoice)} }
                                        td {
                                            Badge {
                                                label: if invoice.status == "PAID" {
                                                    copy.billing_paid.to_string()
                                                } else {
                                                    invoice.status.to_string()
                                                },
                                                tone: if invoice.status == "PAID" {
                                                    "success".to_string()
                                                } else {
                                                    "neutral".to_string()
                                                }
                                            }
                                        }
                                        td { class: "align-right",
                                            if invoice.pdf_available {
                                                button {
                                                    r#type: "button",
                                                    class: "billing-pdf-button",
                                                    onclick: move |_| {
                                                        let notice = format!(
                                                            "{}{}{}",
                                                            copy.billing_pdf_notice,
                                                            invoice.id,
                                                            copy.billing_pdf_notice_suffix
                                                        );
                                                        dioxus::document::eval(&format!("window.alert({notice:?});"));
                                                    },
                                                    Icon { name: IconName::Download, size: 12 }
                                                    span { {copy.billing_pdf} }
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
            if snapshot.modal_open {
                TopUpModal { state, model }
            }
        }
    }
}
