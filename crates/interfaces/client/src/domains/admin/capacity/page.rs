use super::model::{AdminCapacityModel, CapacityStatus};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

#[component]
pub fn AdminCapacity() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = AdminCapacityModel::mock();

    rsx! {
        BuyerShell {
            div { class: "admin-capacity-stack",
                section { class: "page-header admin-capacity-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_capacity_title} }
                        p { {copy.admin_capacity_subtitle} }
                    }
                    button {
                        class: "admin-capacity-autopilot",
                        r#type: "button",
                        aria_label: copy.admin_capacity_active,
                        Icon { name: IconName::Zap, size: 14 }
                        span { ": " {copy.admin_capacity_active} }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-capacity-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_capacity_conclusion} }
                }
                section { class: "metric-grid admin-capacity-metrics", aria_label: copy.admin_capacity_title,
                    MetricCard {
                        label: String::new(),
                        value: model.safety_margin.to_string(),
                        detail: String::new(),
                        trend: Some(copy.admin_capacity_safety_margin.to_string()),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.peak_throughput.to_string(),
                        unit: Some(copy.admin_capacity_tokens_unit.to_string()),
                        detail: copy.admin_capacity_peak_detail.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.replicas.to_string(),
                        unit: Some(copy.admin_capacity_replicas_unit.to_string()),
                        detail: copy.admin_capacity_models_detail.to_string(),
                    }
                    MetricCard {
                        label: String::new(),
                        value: model.latency_target.to_string(),
                        detail: String::new(),
                        status: Some(copy.admin_capacity_latency_healthy.to_string()),
                        status_tone: "healthy".to_string(),
                    }
                }
                section { class: "panel admin-capacity-models-panel", aria_label: copy.admin_capacity_title,
                    h2 { class: "admin-visually-hidden", {copy.admin_capacity_title} }
                    div { class: "table-scroll admin-capacity-table-scroll",
                        table { class: "admin-capacity-table", aria_label: copy.admin_capacity_title,
                            thead { class: "admin-capacity-table-head",
                                tr {
                                    th { span { class: "admin-visually-hidden", {copy.admin_capacity_col_model} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_capacity_col_capacity} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_capacity_col_current} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_capacity_col_headroom} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_capacity_col_latency} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_capacity_col_autoscale} } }
                                    th { span { class: "admin-visually-hidden", {copy.admin_capacity_col_status} } }
                                }
                            }
                            tbody {
                                for row in model.models.iter().copied() {
                                    tr {
                                        td { class: "admin-capacity-model", strong { {row.model} } }
                                        td { class: "admin-capacity-provisioned", {row.provisioned} }
                                        td { class: "admin-capacity-active", strong { {row.active} } }
                                        td {
                                            class: if row.status == CapacityStatus::Degraded {
                                                "admin-capacity-headroom admin-capacity-headroom-degraded"
                                            } else {
                                                "admin-capacity-headroom"
                                            },
                                            {row.headroom}
                                        }
                                        td { class: "admin-capacity-latency", {row.latency} }
                                        td { class: "admin-capacity-action", {row.action} }
                                        td {
                                            span {
                                                class: if row.status == CapacityStatus::Degraded {
                                                    "status admin-capacity-status admin-capacity-status-degraded"
                                                } else {
                                                    "status admin-capacity-status"
                                                },
                                                span { class: "status-dot" }
                                                span {
                                                    class: "status-label",
                                                    if row.status == CapacityStatus::Degraded {
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
