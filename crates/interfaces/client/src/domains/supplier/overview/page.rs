use super::{
    model::{NodeStatus, SupplierNode},
    state::SupplierOverviewState,
};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName, MetricCard},
    },
};
use dioxus::prelude::*;

fn status_cell(node: &SupplierNode, copy: &crate::i18n::LocaleStrings) -> Element {
    let (label, class) = match node.status {
        NodeStatus::Online => (copy.supplier_online, "supplier-node-status-online"),
        NodeStatus::Degraded => (
            copy.supplier_degraded_status,
            "supplier-node-status-degraded",
        ),
    };
    rsx! {
        span { class: "supplier-node-status {class}",
            span { class: "supplier-node-status-dot" }
            span { {label} }
        }
    }
}

fn format_currency(value: f64) -> String {
    format!("${value:.2}")
}

#[component]
pub fn SupplierOverview() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(SupplierOverviewState::default);
    let snapshot = state.read().clone();
    let model = snapshot.model.clone();
    let command = model.install_command;
    let copy_label = if snapshot.command_copied {
        copy.supplier_copied
    } else {
        copy.supplier_copy_command
    };

    rsx! {
        BuyerShell {
            div { class: "supplier-overview-stack",
                section { class: "page-header supplier-page-header",
                    div { class: "supplier-page-header-row",
                        div { class: "page-heading-copy",
                            h1 { {copy.supplier_overview_title} }
                            p { {copy.supplier_overview_subtitle} }
                        }
                        div { class: "page-actions supplier-page-actions",
                            Link {
                                class: "supplier-icon-action button button-secondary",
                                to: "/supplier/resources",
                                title: copy.supplier_view_resources,
                                aria_label: copy.supplier_view_resources,
                                Icon { name: IconName::Server, size: 16 }
                            }
                            button {
                                r#type: "button",
                                class: "supplier-icon-action supplier-icon-action-primary",
                                title: copy.supplier_add_node,
                                aria_label: copy.supplier_add_node,
                                onclick: move |_| state.with_mut(SupplierOverviewState::open_install_modal),
                                Icon { name: IconName::Plus, size: 17 }
                            }
                        }
                    }
                    div { id: "attention", class: "conclusion conclusion-warning supplier-attention", role: "alert",
                        Icon { name: IconName::AlertTriangle, size: 16 }
                        span { class: "conclusion-text",
                            {format!("{}: {} ({}°C)", copy.supplier_attention, model.alert_node, model.nodes.last().map(|node| node.temperature_c).unwrap_or_default())}
                        }
                    }
                }
                section { class: "metric-grid supplier-metrics", aria_label: copy.supplier_overview_title,
                    MetricCard {
                        label: copy.supplier_today_earnings.to_string(),
                        value: format_currency(model.today_earnings),
                        detail: String::new(),
                        trend: Some(format!("+12.4% {}", copy.supplier_earnings_trend)),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: copy.supplier_active_gpus.to_string(),
                        value: format!("{} / {}", model.active_gpu_count, model.total_gpu_count),
                        unit: Some("GPUs".to_string()),
                        detail: String::new(),
                        status: Some(format!("{} Clusters", model.cluster_count)),
                        status_tone: "healthy".to_string(),
                    }
                    MetricCard {
                        label: copy.supplier_gpu_utilization.to_string(),
                        value: format!("{:.1}%", model.gpu_utilization),
                        detail: String::new(),
                        trend: Some(format!("{:.1}% {}", model.peak_compute, copy.supplier_peak_compute)),
                        trend_positive: true,
                    }
                    MetricCard {
                        label: copy.supplier_token_throughput.to_string(),
                        value: format!("{:.1}M", model.tokens_today),
                        unit: Some("tokens".to_string()),
                        detail: String::new(),
                    }
                }
                section { class: "supplier-cluster-alert",
                    Icon { name: IconName::AlertTriangle, size: 18 }
                    strong { {format!("({})", model.alert_node)} }
                    span { class: "supplier-cluster-alert-indicator" }
                }
                section { class: "panel supplier-nodes-panel",
                    div { class: "supplier-nodes-heading",
                        h2 { {copy.supplier_cluster_nodes} }
                        Link {
                            class: "supplier-panel-link",
                            to: "/supplier/resources",
                            title: copy.supplier_view_resources,
                            aria_label: copy.supplier_view_resources,
                            Icon { name: IconName::ArrowRight, size: 16 }
                        }
                    }
                    div { class: "table-scroll",
                        table { class: "supplier-node-table",
                            thead { class: "supplier-table-head",
                                tr {
                                    th { {copy.supplier_node} }
                                    th { {copy.supplier_hardware} }
                                    th { {copy.supplier_model} }
                                    th { {copy.supplier_utilization} }
                                    th { {copy.supplier_temperature} }
                                    th { {copy.supplier_earnings} }
                                    th { {copy.supplier_status} }
                                }
                            }
                            tbody {
                                for node in model.nodes.iter() {
                                    tr {
                                        td {
                                            div { class: "supplier-node-name",
                                                strong { {node.name} }
                                                small { {format!("{} • {}", node.region, node.site)} }
                                            }
                                        }
                                        td {
                                            div { class: "supplier-node-hardware",
                                                strong { {node.hardware} }
                                                small { {node.memory} }
                                            }
                                        }
                                        td { class: "supplier-node-model", {node.model} }
                                        td { class: "supplier-node-number", {format!("{:.1}%", node.utilization)} }
                                        td { class: if node.status == NodeStatus::Degraded { "supplier-node-number supplier-node-temperature-warn" } else { "supplier-node-number" }, {format!("{} °C", node.temperature_c)} }
                                        td { class: "supplier-node-earnings", {format_currency(node.earnings)} }
                                        td { {status_cell(node, copy)} }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if snapshot.install_modal_open {
                div {
                    class: "supplier-modal-backdrop",
                    role: "presentation",
                    onclick: move |_| state.with_mut(SupplierOverviewState::close_install_modal),
                    section {
                        class: "supplier-install-modal",
                        role: "dialog",
                        aria_modal: "true",
                        aria_labelledby: "supplier-install-title",
                        onclick: move |event| event.stop_propagation(),
                        header { class: "supplier-install-header",
                            h2 { id: "supplier-install-title", {copy.supplier_install_title} }
                            button {
                                r#type: "button",
                                class: "icon-button",
                                title: copy.close,
                                aria_label: copy.close,
                                onclick: move |_| state.with_mut(SupplierOverviewState::close_install_modal),
                                Icon { name: IconName::X, size: 16 }
                            }
                        }
                        p { class: "supplier-install-description", {copy.supplier_install_description} }
                        code { class: "supplier-install-command", {command} }
                        button {
                            r#type: "button",
                            class: if snapshot.command_copied { "supplier-copy-command copied" } else { "supplier-copy-command" },
                            onclick: move |_| {
                                dioxus::document::eval(&format!("navigator.clipboard?.writeText({command:?});"));
                                state.with_mut(SupplierOverviewState::mark_command_copied);
                            },
                            Icon { name: if snapshot.command_copied { IconName::Check } else { IconName::Copy }, size: 14 }
                            span { {copy_label} }
                        }
                    }
                }
            }
        }
    }
}
