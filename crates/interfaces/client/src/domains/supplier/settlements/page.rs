use super::{
    actions::format_currency,
    model::{SettlementBatch, SupplierSettlementsModel},
    state::SupplierSettlementsState,
};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Badge, Icon, IconName},
    },
};
use dioxus::prelude::*;

#[component]
fn SettlementPayoutModal(
    mut state: Signal<SupplierSettlementsState>,
    model: SupplierSettlementsModel,
) -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());

    rsx! {
        div {
            class: "supplier-modal-backdrop",
            role: "presentation",
            onclick: move |_| state.with_mut(SupplierSettlementsState::close_payout_modal),
            section {
                class: "supplier-settlements-modal",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "supplier-settlements-modal-title",
                onclick: move |event| event.stop_propagation(),
                header { class: "supplier-settlements-modal-header",
                    h2 { id: "supplier-settlements-modal-title", {copy.supplier_settlements_confirm_payout} }
                    button {
                        r#type: "button",
                        class: "icon-button",
                        title: copy.close,
                        aria_label: copy.close,
                        onclick: move |_| state.with_mut(SupplierSettlementsState::close_payout_modal),
                        Icon { name: IconName::X, size: 16 }
                    }
                }
                div { class: "supplier-settlements-modal-summary",
                    div {
                        span { {copy.supplier_settlements_available_balance} }
                        strong { {format_currency(model.next_payout)} }
                    }
                    div {
                        span { {copy.supplier_settlements_payout_destination} }
                        strong { {format!("{} ({})", model.payout_destination, model.payout_account)} }
                    }
                    div { class: "supplier-settlements-modal-total",
                        span { {copy.supplier_settlements_transfer_total} }
                        strong { {format_currency(model.next_payout)} }
                    }
                }
                div { class: "supplier-settlements-modal-actions",
                    button {
                        r#type: "button",
                        class: "button button-secondary",
                        onclick: move |_| state.with_mut(SupplierSettlementsState::close_payout_modal),
                        {copy.supplier_cancel}
                    }
                    button {
                        r#type: "button",
                        class: "button button-primary",
                        onclick: move |_| state.with_mut(|value| value.confirm_payout(copy.supplier_settlements_success)),
                        {copy.supplier_confirm}
                    }
                }
            }
        }
    }
}

fn settlement_row(batch: SettlementBatch, copy: crate::i18n::LocaleStrings) -> Element {
    rsx! {
        tr {
            td { class: "supplier-settlements-id", {batch.id} }
            td { class: "supplier-settlements-date", {batch.date} }
            td { class: "supplier-settlements-method", {batch.payment_method} }
            td { class: "supplier-settlements-tokens", {batch.tokens} }
            td { class: "supplier-settlements-amount", {format_currency(batch.amount)} }
            td {
                Badge {
                    label: copy.supplier_settlements_completed_status.to_string(),
                    tone: "success".to_string(),
                }
            }
            td { class: "supplier-settlements-receipt",
                button {
                    r#type: "button",
                    class: "supplier-settlements-pdf-button",
                    onclick: move |_| {
                        let notice = format!(
                            "{}{}{}",
                            copy.supplier_settlements_download_notice,
                            batch.id,
                            copy.supplier_settlements_download_notice_suffix,
                        );
                        dioxus::document::eval(&format!("window.alert({notice:?});"));
                    },
                    Icon { name: IconName::Download, size: 12 }
                    span { {copy.supplier_settlements_pdf} }
                }
            }
        }
    }
}

#[component]
pub fn SupplierSettlements() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(SupplierSettlementsState::default);
    let snapshot = state.read().clone();
    let model = *SupplierSettlementsModel::reference();

    rsx! {
        BuyerShell {
            div { class: "supplier-settlements-stack",
                section { class: "page-header supplier-settlements-page-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.supplier_settlements_title} }
                        p { {copy.supplier_settlements_subtitle} }
                    }
                    button {
                        r#type: "button",
                        class: "supplier-settlements-payout-action",
                        title: copy.supplier_settlements_payout_action,
                        aria_label: copy.supplier_settlements_payout_action,
                        onclick: move |_| state.with_mut(SupplierSettlementsState::open_payout_modal),
                        Icon { name: IconName::DollarSign, size: 17 }
                    }
                }
                div { class: "conclusion conclusion-healthy supplier-settlements-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.supplier_settlements_conclusion} }
                }
                if let Some(message) = snapshot.success_message.clone() {
                    div { class: "supplier-settlements-success", role: "status",
                        Icon { name: IconName::CheckCircle, size: 15 }
                        span { {message} }
                    }
                }
                section { class: "supplier-settlements-summary-grid", aria_label: copy.supplier_settlements_title,
                    article { class: "supplier-settlements-card supplier-settlements-payout-card",
                        strong { class: "supplier-settlements-payout-value", {format_currency(model.next_payout)} }
                        div { class: "supplier-settlements-card-footer",
                            span { {format!("{}: {}", copy.supplier_settlements_next_payout, model.next_payout_date)} }
                            Icon { name: IconName::ArrowRight, size: 14 }
                        }
                    }
                    article { class: "supplier-settlements-card",
                        div { class: "supplier-settlements-card-heading",
                            Icon { name: IconName::Building, size: 16 }
                            strong { {model.payout_destination} }
                        }
                        div { class: "supplier-settlements-card-footer",
                            span { {format!("{} {}", copy.supplier_settlements_account_ending, model.payout_account)} }
                            if model.payout_verified {
                                Badge {
                                    label: copy.supplier_settlements_verified.to_string(),
                                    tone: "success".to_string(),
                                }
                            }
                        }
                    }
                    article { class: "supplier-settlements-card",
                        strong { class: "supplier-settlements-balance-value", {format_currency(model.available_balance)} }
                        div { class: "supplier-settlements-card-footer supplier-settlements-card-footer-muted",
                            span { {format!("{} {}", model.completed_batches, copy.supplier_settlements_completed)} }
                            span { {format!("{} {}", model.disputes, copy.supplier_settlements_disputes)} }
                        }
                    }
                }
                section { class: "panel supplier-settlements-panel",
                    div { class: "table-scroll supplier-settlements-table-scroll",
                        table { class: "supplier-settlements-table",
                            thead {
                                tr {
                                    th { {copy.supplier_settlements_col_batch} }
                                    th { {copy.supplier_settlements_col_date} }
                                    th { {copy.supplier_settlements_col_method} }
                                    th { {copy.supplier_settlements_col_tokens} }
                                    th { {copy.supplier_settlements_col_amount} }
                                    th { {copy.supplier_settlements_col_status} }
                                    th { class: "supplier-settlements-receipt-heading", {copy.supplier_settlements_col_receipt} }
                                }
                            }
                            tbody {
                                for batch in model.batches {
                                    {settlement_row(batch, *copy)}
                                }
                            }
                        }
                    }
                }
            }
            if snapshot.payout_modal_open {
                SettlementPayoutModal { state, model }
            }
        }
    }
}
