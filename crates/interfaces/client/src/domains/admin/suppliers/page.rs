use super::model::{AdminSupplier, SUPPLIERS};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Badge, Icon, IconName},
    },
};
use dioxus::prelude::*;

fn supplier_row(supplier: AdminSupplier) -> Element {
    rsx! {
        tr {
            td { class: "admin-suppliers-name", strong { {supplier.name} } }
            td {
                Badge {
                    label: supplier.tier.to_string(),
                    tone: if supplier.tier.starts_with("L4") {
                        "accent".to_string()
                    } else {
                        "brand".to_string()
                    },
                }
            }
            td { class: "admin-suppliers-gpus", {supplier.connected_gpus} }
            td { class: "admin-suppliers-uptime", {supplier.uptime_sla} }
            td { class: "admin-suppliers-share", {supplier.revenue_share} }
            td { class: "admin-suppliers-earnings", {supplier.earnings_30d} }
            td { class: "admin-suppliers-status-cell",
                span { class: "status admin-suppliers-status",
                    span { class: "status-dot" }
                    span { class: "status-label", {supplier.status} }
                }
            }
        }
    }
}

#[component]
pub fn AdminSuppliers() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());

    rsx! {
        BuyerShell {
            div { class: "admin-suppliers-stack",
                section { class: "page-header admin-suppliers-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_suppliers_title} }
                        p { {copy.admin_suppliers_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-suppliers-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_suppliers_conclusion} }
                }
                section { class: "panel admin-suppliers-panel", aria_label: copy.admin_suppliers_title,
                    div { class: "admin-suppliers-panel-heading",
                        h2 { {copy.admin_suppliers_directory_title} }
                    }
                    div { class: "table-scroll admin-suppliers-table-scroll",
                        table { class: "admin-suppliers-table", aria_label: copy.admin_suppliers_directory_title,
                            thead { class: "admin-suppliers-table-head",
                                tr {
                                    for column in copy.admin_suppliers_columns {
                                        th { scope: "col", {column} }
                                    }
                                }
                            }
                            tbody {
                                for supplier in SUPPLIERS.iter().copied() {
                                    {supplier_row(supplier)}
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
