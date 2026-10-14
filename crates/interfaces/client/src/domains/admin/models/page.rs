use super::model::{format_price, tier_label};
use crate::{
    domains::buyer::marketplace::model::{ModelStatus, MODEL_CATALOG},
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;

#[component]
pub fn AdminModels() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());

    rsx! {
        BuyerShell {
            div { class: "admin-models-stack",
                section { class: "page-header admin-models-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_models_title} }
                        p { {copy.admin_models_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-models-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_models_conclusion} }
                }
                section { class: "panel admin-models-panel", aria_label: copy.admin_models_title,
                    h2 { class: "admin-visually-hidden", {copy.admin_models_title} }
                    div { class: "table-scroll admin-models-table-scroll",
                        table { class: "admin-models-table", aria_label: copy.admin_models_title,
                            thead { class: "admin-models-table-head",
                                tr {
                                    for column in copy.admin_models_columns {
                                        th { scope: "col", {column} }
                                    }
                                }
                            }
                            tbody {
                                for model in MODEL_CATALOG.iter().copied() {
                                    tr {
                                        td { class: "admin-models-name", strong { {model.name} } }
                                        td { class: "admin-models-price",
                                            strong { {format!("$ {} / $ {}", format_price(model.input_price_per_million), format_price(model.output_price_per_million))} }
                                        }
                                        td { class: "admin-models-context", {model.context_window} }
                                        td { class: "admin-models-tiers",
                                            div { class: "admin-models-tier-list",
                                                for tier in model.supported_tiers {
                                                    span { class: "tier", {tier_label(*tier)} }
                                                }
                                            }
                                        }
                                        td { class: "admin-models-latency", {format!("{} ms", model.p95_latency_ms)} }
                                        td { class: "admin-models-availability", {format!("{} %", model.availability)} }
                                        td {
                                            span {
                                                class: if model.status == ModelStatus::Degraded {
                                                    "status status-warning admin-models-status"
                                                } else {
                                                    "status admin-models-status"
                                                },
                                                span { class: "status-dot" }
                                                span { class: "status-label",
                                                    if model.status == ModelStatus::Degraded {
                                                        {copy.admin_capacity_degraded}
                                                    } else {
                                                        {copy.admin_capacity_healthy}
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
            }
        }
    }
}
