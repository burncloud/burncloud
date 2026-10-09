use super::model::AdminSupplyModel;
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

#[component]
pub fn AdminSupply() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = AdminSupplyModel::mock();

    rsx! {
        BuyerShell {
            div { class: "admin-supply-stack",
                section { class: "page-header admin-supply-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_supply_title} }
                        p { {copy.admin_supply_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-supply-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_supply_conclusion} }
                }
                section { class: "metric-grid admin-supply-metrics", aria_label: copy.admin_supply_title,
                    MetricCard {
                        label: String::new(),
                        value: model.total_gpus.to_string(),
                        unit: Some(copy.admin_supply_gpu_unit.to_string()),
                        detail: copy.admin_supply_online_rate.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.providers.to_string(),
                        detail: copy.admin_supply_provider_tiers.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.core_dc_gpus.to_string(),
                        unit: Some(copy.admin_supply_gpu_unit.to_string()),
                        detail: copy.admin_supply_core_dc.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.standby_gpus.to_string(),
                        unit: Some(copy.admin_supply_gpu_unit.to_string()),
                        detail: copy.admin_supply_standby.to_string(),
                    }
                }
                section { class: "panel admin-supply-nodes-panel", aria_label: copy.admin_supply_clusters,
                    h2 { class: "admin-visually-hidden", {copy.admin_supply_clusters} }
                    div { class: "table-scroll admin-supply-table-scroll",
                        table { class: "admin-supply-table", aria_label: copy.admin_supply_clusters,
                            thead { class: "admin-supply-table-head",
                                tr {
                                    th { span { class: "admin-visually-hidden", {copy.admin_supply_col_cluster} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_supply_col_source} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_supply_col_hardware} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_supply_col_region} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_supply_col_utilization} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_supply_col_attestation} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_supply_col_status} } }
                                }
                            }
                            tbody {
                                for cluster in model.clusters.iter().copied() {
                                    tr {
                                        td { class: "admin-supply-cluster-name", strong { {cluster.name} } }
                                        td { class: "admin-supply-source", {cluster.source} }
                                        td { class: "admin-supply-hardware", {cluster.hardware} }
                                        td { class: "admin-supply-region", {cluster.region} }
                                        td { class: "admin-supply-utilization", strong { {cluster.utilization} } }
                                        td { class: "admin-supply-attestation", strong { {cluster.attestation} } }
                                        td {
                                            span { class: "status admin-supply-status",
                                                span { class: "status-dot" }
                                                span { class: "status-label", {copy.admin_supply_online} }
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
