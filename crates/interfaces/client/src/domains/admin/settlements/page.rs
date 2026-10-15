use super::{
    model::{AdminSettlementBatch, SettlementBatchStatus},
    state::AdminSettlementsState,
};
use crate::{
    i18n::{strings, Locale, LocaleStrings},
    shared::{
        layout::BuyerShell,
        ui::{Badge, Icon, IconName},
    },
};
use dioxus::prelude::*;

fn settlement_row(
    batch: AdminSettlementBatch,
    mut state: Signal<AdminSettlementsState>,
    copy: LocaleStrings,
) -> Element {
    rsx! {
        tr {
            td { class: "admin-settlements-id", {batch.id} }
            td { class: "admin-settlements-period", {batch.billing_period} }
            td { class: "admin-settlements-suppliers",
                {format!("{} Providers ({} GPUs)", batch.total_suppliers, batch.total_gpus)}
            }
            td { class: "admin-settlements-tokens", {batch.inference_tokens} }
            td { class: "admin-settlements-payout", {batch.total_payout} }
            td {
                Badge {
                    label: batch.status.label().to_string(),
                    tone: batch.status.tone().to_string(),
                }
            }
            td { class: "admin-settlements-action",
                if batch.status == SettlementBatchStatus::PendingReview {
                    button {
                        r#type: "button",
                        class: "button button-primary admin-settlements-approve",
                        onclick: move |_| state.with_mut(|value| value.approve_batch(batch.id)),
                        {copy.admin_settlements_approve}
                    }
                } else {
                    span { class: "admin-settlements-completed", "Completed" }
                }
            }
        }
    }
}

#[component]
pub fn AdminSettlements() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let state = use_signal(AdminSettlementsState::default);
    let snapshot = *state.read();

    rsx! {
        BuyerShell {
            div { class: "admin-settlements-stack",
                section { class: "page-header admin-settlements-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_settlements_title} }
                        p { {copy.admin_settlements_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-settlements-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_settlements_conclusion} }
                }
                section { class: "panel admin-settlements-panel", aria_label: copy.admin_settlements_batches_title,
                    div { class: "admin-settlements-panel-heading",
                        h2 { {copy.admin_settlements_batches_title} }
                        p { {copy.admin_settlements_batches_subtitle} }
                    }
                    div { class: "table-scroll admin-settlements-table-scroll",
                        table { class: "admin-settlements-table", aria_label: copy.admin_settlements_batches_title,
                            thead { class: "admin-settlements-table-head",
                                tr {
                                    for column in copy.admin_settlements_columns {
                                        th { scope: "col", {column} }
                                    }
                                }
                            }
                            tbody {
                                for batch in snapshot.batches {
                                    {settlement_row(batch, state, *copy)}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
