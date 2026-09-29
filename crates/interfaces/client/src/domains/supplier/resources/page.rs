use super::{model::SupplierResource, state::SupplierResourcesState};
use crate::{
    domains::supplier::overview::model::NodeStatus,
    i18n::{strings, Locale, LocaleStrings},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;

fn format_currency(value: f64) -> String {
    format!("${value:.2}")
}

fn status_label(status: NodeStatus, copy: &LocaleStrings) -> (&'static str, &'static str) {
    match status {
        NodeStatus::Online => (copy.supplier_online, "supplier-resource-status-online"),
        NodeStatus::Degraded => (
            copy.supplier_degraded_status,
            "supplier-resource-status-degraded",
        ),
    }
}

#[component]
fn SupplierResourceCard(
    resource: SupplierResource,
    copy: LocaleStrings,
    mut state: Signal<SupplierResourcesState>,
) -> Element {
    let node = resource.node;
    let (status, status_class) = status_label(node.status, &copy);
    let temperature_class = if node.status == NodeStatus::Degraded {
        "supplier-resource-temperature supplier-resource-temperature-warn"
    } else {
        "supplier-resource-temperature"
    };

    rsx! {
        article { class: "panel supplier-resource-card",
            header { class: "supplier-resource-card-header",
                div { class: "supplier-resource-heading",
                    div { class: "supplier-resource-name-row",
                        h2 { {node.name} }
                        span { class: "supplier-resource-status {status_class}", {status} }
                    }
                    p { {format!("{} • {}", node.region, node.site)} }
                }
                strong { class: "supplier-resource-earnings", {format_currency(node.earnings)} }
            }
            div { class: "supplier-resource-specs",
                div { class: "supplier-resource-spec",
                    strong { class: "supplier-resource-hardware-value", {node.hardware} }
                }
                div { class: "supplier-resource-spec",
                    strong { class: "supplier-resource-value", {node.vram} }
                }
                div { class: "supplier-resource-spec supplier-resource-spec-divider",
                    strong { class: "supplier-resource-value", {node.interconnect} }
                }
                div { class: "supplier-resource-spec supplier-resource-spec-divider",
                    span { class: "supplier-resource-spec-label", {copy.supplier_core_temperature} }
                    strong { class: "{temperature_class}", {format!("{} °C", node.temperature_c)} }
                }
            }
            div { class: "supplier-resource-utilization",
                div { class: "supplier-resource-utilization-heading",
                    span { {copy.supplier_utilization} }
                    strong { {format!("{:.1}%", node.utilization)} }
                }
                div { class: "supplier-resource-progress", role: "progressbar", aria_valuenow: node.utilization, aria_valuemin: "0", aria_valuemax: "100",
                    div { style: format!("width: {:.1}%", node.utilization) }
                }
            }
                div { class: "supplier-resource-model",
                    Icon { name: IconName::Cpu, size: 16 }
                strong { class: "supplier-resource-model-copy", {format!("{}: {}", copy.supplier_model, node.model)} }
            }
            footer { class: "supplier-resource-card-footer",
                span { class: "supplier-resource-uptime", {format!("{}: {} h", copy.supplier_uptime, resource.uptime_hours)} }
                if node.status == NodeStatus::Online {
                    button {
                        r#type: "button",
                        class: "supplier-resource-drain",
                        title: copy.supplier_graceful_drain,
                        aria_label: copy.supplier_graceful_drain,
                        onclick: move |_| state.with_mut(|value| value.open_drain(node.name)),
                        Icon { name: IconName::PowerOff, size: 12 }
                    }
                }
            }
        }
    }
}

#[component]
pub fn SupplierResources() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(SupplierResourcesState::default);
    let snapshot = state.read().clone();
    let selected_node = snapshot.selected_node();

    rsx! {
        BuyerShell {
            div { class: "supplier-resources-stack",
                section { class: "page-header supplier-resources-page-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.supplier_resources_title} }
                        p { {copy.supplier_resources_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy supplier-resources-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.supplier_resources_conclusion} }
                }
                div { class: "supplier-resource-grid",
                    for resource in snapshot.model.nodes.iter().copied() {
                        SupplierResourceCard { resource, copy: *copy, state }
                    }
                }
            }
            if let Some(resource) = selected_node {
                div {
                    class: "supplier-modal-backdrop",
                    role: "presentation",
                    onclick: move |_| state.with_mut(SupplierResourcesState::close_drain),
                    section {
                        class: "supplier-drain-modal",
                        role: "dialog",
                        aria_modal: "true",
                        aria_labelledby: "supplier-drain-title",
                        onclick: move |event| event.stop_propagation(),
                        header { class: "supplier-install-header",
                            h2 { id: "supplier-drain-title", {format!("{}: {}", copy.supplier_graceful_drain, resource.node.name)} }
                            button {
                                r#type: "button",
                                class: "icon-button",
                                title: copy.close,
                                aria_label: copy.close,
                                onclick: move |_| state.with_mut(SupplierResourcesState::close_drain),
                                Icon { name: IconName::X, size: 16 }
                            }
                        }
                        p { class: "supplier-install-description", {copy.supplier_graceful_drain_subtitle} }
                        div { class: "supplier-drain-notice",
                            strong {
                                Icon { name: IconName::AlertTriangle, size: 15 }
                                span { {copy.supplier_zero_downtime} }
                            }
                            p { {copy.supplier_zero_downtime_description} }
                        }
                        div { class: "supplier-drain-actions",
                            button {
                                r#type: "button",
                                class: "button button-secondary",
                                onclick: move |_| state.with_mut(SupplierResourcesState::close_drain),
                                {copy.supplier_cancel}
                            }
                            button {
                                r#type: "button",
                                class: "button button-warning",
                                onclick: move |_| state.with_mut(SupplierResourcesState::confirm_drain),
                                {copy.supplier_confirm}
                            }
                        }
                    }
                }
            }
        }
    }
}
