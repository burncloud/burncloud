use super::model::{DeploymentStatus, SupplierDeploymentsModel};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;

#[component]
pub fn SupplierDeployments() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = SupplierDeploymentsModel::reference();

    rsx! {
        BuyerShell {
            div { class: "supplier-deployments-stack",
                section { class: "page-header supplier-deployments-page-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.supplier_deployments_title} }
                        p { {copy.supplier_deployments_subtitle} }
                    }
                    div { class: "conclusion conclusion-healthy supplier-deployments-conclusion", role: "status",
                        Icon { name: IconName::CheckCircle, size: 16 }
                        span { class: "conclusion-text", {copy.supplier_deployments_conclusion} }
                    }
                }
                section { class: "panel supplier-deployments-panel",
                    header { class: "supplier-deployments-card-header",
                        div {
                            h2 { {copy.supplier_deployments_title} }
                            p { {copy.supplier_deployments_subtitle} }
                        }
                        div { class: "supplier-deployments-autopilot",
                            Icon { name: IconName::Shield, size: 16 }
                            span { {copy.supplier_deployments_autopilot_status} }
                        }
                    }
                    div { class: "table-scroll supplier-deployments-table-scroll",
                        table { class: "supplier-deployments-table",
                            thead {
                                tr {
                                    th { {copy.model} }
                                    th { {copy.supplier_node} }
                                    th { {copy.supplier_deployments_parallelism} }
                                    th { {copy.supplier_deployments_throughput} }
                                    th { {copy.supplier_deployments_contribution} }
                                    th { class: "supplier-deployments-status-heading", {copy.supplier_deployments_autopilot_status} }
                                }
                            }
                            tbody {
                                for deployment in model.deployments {
                                    tr {
                                        td { class: "supplier-deployments-model", {deployment.model} }
                                        td { class: "supplier-deployments-node", {deployment.node} }
                                        td { class: "supplier-deployments-parallelism", {deployment.parallelism} }
                                        td { class: "supplier-deployments-throughput", {deployment.throughput} }
                                        td { class: "supplier-deployments-contribution", {deployment.contribution} }
                                        td { class: "supplier-deployments-status-cell",
                                            if deployment.status == DeploymentStatus::Healthy {
                                                span { class: "badge-success", {copy.supplier_deployments_healthy} }
                                            } else {
                                                span { class: "badge-warning", {copy.supplier_degraded_status} }
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
