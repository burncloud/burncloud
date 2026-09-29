use super::model::{DeploymentStatus, SupplierDeployment, SupplierDeploymentsModel};
use crate::{
    i18n::{strings, Locale, LocaleStrings},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;

fn status_cell(deployment: &SupplierDeployment, copy: &LocaleStrings) -> Element {
    let (label, class) = match deployment.status {
        DeploymentStatus::Healthy => (copy.healthy_short, "supplier-deployment-status-healthy"),
        DeploymentStatus::Degraded => (
            copy.supplier_degraded_status,
            "supplier-deployment-status-degraded",
        ),
    };

    rsx! {
        span { class: "supplier-deployment-status {class}",
            span { class: "supplier-deployment-status-dot" }
            span { {label} }
        }
    }
}

#[component]
pub fn SupplierDeployments() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = SupplierDeploymentsModel::mock();

    rsx! {
        BuyerShell {
            div { class: "supplier-deployments-stack",
                section { class: "page-header supplier-deployments-page-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.supplier_deployments_title} }
                        p { {copy.supplier_deployments_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy supplier-deployments-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.supplier_deployments_conclusion} }
                }
                section { class: "panel supplier-deployments-panel",
                    header { class: "section-header supplier-deployments-header",
                        div {
                            h2 { {copy.supplier_deployments_title} }
                            p { {copy.supplier_deployments_table_subtitle} }
                        }
                        span {
                            class: "supplier-deployments-protection",
                            title: copy.supplier_deployments_protection,
                            aria_label: copy.supplier_deployments_protection,
                            Icon { name: IconName::Shield, size: 16 }
                        }
                    }
                    div { class: "table-scroll",
                        table { class: "supplier-deployment-table",
                            thead { class: "supplier-deployment-table-head",
                                tr {
                                    th { {copy.supplier_deployments_model} }
                                    th { {copy.supplier_deployments_node} }
                                    th { {copy.supplier_deployments_runtime} }
                                    th { {copy.supplier_deployments_throughput} }
                                    th { {copy.supplier_deployments_score} }
                                    th { {copy.supplier_status} }
                                }
                            }
                            tbody {
                                for deployment in model.deployments.iter() {
                                    tr {
                                        td { class: "supplier-deployment-model", {deployment.model} }
                                        td { class: "supplier-deployment-node", {deployment.node} }
                                        td { class: "supplier-deployment-runtime", {deployment.runtime} }
                                        td { class: "supplier-deployment-throughput", {deployment.throughput} }
                                        td { class: "supplier-deployment-score", {deployment.score} }
                                        td { class: "supplier-deployment-status-cell", {status_cell(deployment, copy)} }
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
