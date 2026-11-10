use super::model::{AdminCustomersModel, CustomerRisk};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;

#[component]
pub fn AdminCustomers() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = AdminCustomersModel::mock();

    rsx! {
        BuyerShell {
            div { class: "admin-customers-stack",
                section { class: "page-header admin-customers-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_customers_title} }
                        p { {copy.admin_customers_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-customers-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_customers_conclusion} }
                }
                section { class: "panel admin-customers-panel", aria_label: copy.admin_customers_accounts_title,
                    div { class: "admin-customers-panel-header",
                        h3 { {copy.admin_customers_accounts_title} }
                        p { {copy.admin_customers_accounts_subtitle} }
                    }
                    div { class: "table-scroll admin-customers-table-scroll",
                        table { class: "admin-customers-table", aria_label: copy.admin_customers_accounts_title,
                            thead {
                                tr {
                                    th { {copy.admin_customers_col_tenant_org} }
                                    th { {copy.admin_customers_col_contact_email} }
                                    th { {copy.admin_customers_col_active_api_keys} }
                                    th { {copy.admin_customers_col_30d_spend} }
                                    th { {copy.admin_customers_col_escrow_balance} }
                                    th { {copy.admin_customers_col_credit_risk} }
                                    th { class: "align-right", {copy.admin_customers_col_status} }
                                }
                            }
                            tbody {
                                for customer in model.customers.iter().copied() {
                                    tr {
                                        td { class: "admin-customers-name", strong { {customer.name} } }
                                        td { class: "admin-customers-email", {customer.email} }
                                        td { class: "admin-customers-keys", {format!("{} keys", customer.keys)} }
                                        td { class: "admin-customers-spend", {customer.spend} }
                                        td { class: "admin-customers-balance", {customer.balance} }
                                        td { class: "admin-customers-risk",
                                            span {
                                                class: if customer.risk == CustomerRisk::Low {
                                                    "badge-success"
                                                } else {
                                                    "badge-warning"
                                                },
                                                if customer.risk == CustomerRisk::Low { "Low Risk" } else { "Low Balance" }
                                            }
                                        }
                                        td { class: "admin-customers-status align-right",
                                            span { class: "status",
                                                span { class: "status-dot" }
                                                span { class: "status-label", {customer.status} }
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
