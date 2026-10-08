use super::model::{AdminEventKind, AdminOverviewModel};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

fn event_class(kind: AdminEventKind) -> &'static str {
    match kind {
        AdminEventKind::Capacity => "admin-event-tag admin-event-tag-capacity",
        AdminEventKind::Failover => "admin-event-tag admin-event-tag-failover",
        AdminEventKind::Economics => "admin-event-tag admin-event-tag-economics",
    }
}

#[component]
pub fn AdminOverview() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = AdminOverviewModel::mock();
    let event_impacts = copy
        .admin_overview_event_impacts
        .map(|impact| impact.split_once('•').unwrap_or((impact, "")));

    rsx! {
        BuyerShell {
            div { class: "admin-overview-stack",
                section { class: "page-header admin-overview-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_overview_title} }
                        p { {copy.admin_overview_subtitle} }
                    }
                    div { class: "page-actions admin-overview-actions",
                        Link {
                            class: "admin-overview-icon-action button button-secondary",
                            to: "/admin/operations",
                            title: copy.admin_overview_operations_action,
                            aria_label: copy.admin_overview_operations_action,
                            Icon { name: IconName::Workflow, size: 16 }
                        }
                        Link {
                            class: "admin-overview-icon-action admin-overview-icon-action-primary",
                            to: "/admin/capacity",
                            title: copy.admin_overview_capacity_action,
                            aria_label: copy.admin_overview_capacity_action,
                            Icon { name: IconName::Gauge, size: 17 }
                        }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-overview-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_overview_sla_summary} }
                }
                section { class: "metric-grid admin-overview-metrics", aria_label: copy.admin_overview_title,
                    MetricCard {
                        label: copy.admin_overview_gmv_label.to_string(),
                        value: model.gmv.to_string(),
                        detail: String::new(),
                        trend: Some(copy.admin_overview_gmv_trend.to_string()),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: copy.admin_overview_demand_label.to_string(),
                        value: model.demand_share.to_string(),
                        detail: String::new(),
                        trend: Some(copy.admin_overview_demand_trend.to_string()),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: copy.admin_overview_compute_label.to_string(),
                        value: model.compute.to_string(),
                        unit: Some(copy.admin_overview_gpu_unit.to_string()),
                        detail: String::new(),
                        status: Some(copy.admin_overview_tflops.to_string()),
                        status_tone: "healthy".to_string(),
                    }
                    MetricCard {
                        label: copy.admin_overview_sla_label.to_string(),
                        value: model.sla.to_string(),
                        detail: String::new(),
                        status: Some(copy.admin_overview_healthy.to_string()),
                        status_tone: "healthy".to_string(),
                    }
                }
                section {
                    id: "attention",
                    class: "admin-overview-attention",
                    role: "alert",
                    aria_label: copy.admin_overview_attention_label,
                    Icon { name: IconName::AlertTriangle, size: 18 }
                    span { class: "admin-overview-attention-indicator" }
                }
                section { class: "panel admin-overview-events-panel", aria_label: copy.admin_overview_events_label,
                    h2 { class: "admin-visually-hidden", {copy.admin_overview_events_label} }
                    div { class: "admin-overview-panel-toolbar",
                        Link {
                            class: "admin-overview-panel-arrow",
                            to: "/admin/operations",
                            title: copy.admin_overview_events_action,
                            aria_label: copy.admin_overview_events_action,
                            Icon { name: IconName::ArrowRight, size: 16 }
                        }
                    }
                    div { class: "admin-event-list",
                        for (index, event) in model.events.iter().enumerate() {
                            article { class: "admin-event-item",
                                div { class: "admin-event-content",
                                    div { class: "admin-event-heading",
                                        span { class: event_class(event.kind), {copy.admin_overview_event_labels[index]} }
                                        strong { {copy.admin_overview_event_titles[index]} }
                                    }
                                    p { {copy.admin_overview_event_details[index]} }
                                    div { class: "admin-event-impact",
                                        span { class: "admin-event-impact-primary", {event_impacts[index].0.trim()} }
                                        span { class: "admin-event-impact-separator", "•" }
                                        span { class: "admin-event-impact-secondary", {event_impacts[index].1.trim()} }
                                    }
                                }
                                time { {copy.admin_overview_event_times[index]} }
                            }
                        }
                    }
                }
                div { class: "admin-overview-lower-grid",
                    section { class: "panel admin-overview-stat-panel", aria_label: copy.admin_overview_supply_label,
                        div { class: "admin-overview-stat-toolbar",
                            h2 { class: "admin-overview-stat-title",
                                Icon { name: IconName::Server, size: 16 }
                                span { {copy.admin_overview_supply_label} }
                            }
                            Link {
                                class: "admin-overview-panel-arrow",
                                to: "/admin/supply",
                                title: copy.admin_overview_supply_action,
                                aria_label: copy.admin_overview_supply_action,
                                "→"
                            }
                        }
                        div { class: "admin-overview-stat-list",
                            for value in model.supply_breakdown.iter() {
                                div { class: "admin-overview-stat-row",
                                    span {}
                                    span { class: "admin-overview-stat-value", {*value} }
                                }
                            }
                        }
                    }
                    section { class: "panel admin-overview-stat-panel", aria_label: copy.admin_overview_demand_panel_label,
                        div { class: "admin-overview-stat-toolbar",
                            h2 { class: "admin-overview-stat-title",
                                Icon { name: IconName::Trending, size: 16 }
                                span { {copy.admin_overview_demand_panel_label} }
                            }
                            Link {
                                class: "admin-overview-panel-arrow",
                                to: "/admin/demand",
                                title: copy.admin_overview_demand_action,
                                aria_label: copy.admin_overview_demand_action,
                                "→"
                            }
                        }
                        div { class: "admin-overview-stat-list",
                            for (index, value) in model.demand_breakdown.iter().enumerate() {
                                div {
                                    class: if index == 1 {
                                        "admin-overview-stat-row admin-overview-stat-row-emphasis"
                                    } else {
                                        "admin-overview-stat-row"
                                    },
                                    span {}
                                    span { class: "admin-overview-stat-value", {*value} }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
