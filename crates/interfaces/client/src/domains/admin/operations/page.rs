use super::model::{AdminOperationsLog, AdminOperationsModel, OperationsLogLevel};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Badge, Icon, IconName},
    },
};
use dioxus::prelude::*;

fn log_tone(level: OperationsLogLevel) -> &'static str {
    level.tone()
}

#[component]
pub fn AdminOperations() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let model = AdminOperationsModel::mock();
    let mut circuit_breaker_active = use_signal(|| false);

    rsx! {
        BuyerShell {
            div { class: "admin-operations-stack",
                section { class: "page-header admin-operations-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_operations_title} }
                        p { {copy.admin_operations_subtitle} }
                    }
                    button {
                        r#type: "button",
                        class: if circuit_breaker_active() {
                            "button button-warning admin-operations-breaker"
                        } else {
                            "button button-secondary admin-operations-breaker"
                        },
                        aria_pressed: circuit_breaker_active(),
                        onclick: move |_| circuit_breaker_active.set(!circuit_breaker_active()),
                        Icon { name: IconName::Shield, size: 14 }
                        span { {copy.admin_operations_circuit_breaker} }
                        span { class: "admin-operations-breaker-state", if circuit_breaker_active() { "ARMED" } else { "STANDBY" } }
                    }
                }
                div { class: "conclusion conclusion-healthy admin-operations-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.admin_operations_conclusion} }
                }
                section { class: "panel admin-operations-log-panel", aria_label: copy.admin_operations_log_title,
                    div { class: "admin-operations-log-header",
                        div {
                            h2 { {copy.admin_operations_log_title} }
                            p { {copy.admin_operations_log_subtitle} }
                        }
                    }
                    div { class: "admin-operations-log-list",
                        for log in model.logs.iter().copied() {
                            OperationsLogRow { log }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn OperationsLogRow(log: AdminOperationsLog) -> Element {
    rsx! {
        article { class: "admin-operations-log-row",
            div { class: "admin-operations-log-copy",
                div { class: "admin-operations-log-heading",
                    Badge {
                        label: log.category.to_string(),
                        tone: log_tone(log.level).to_string(),
                    }
                    strong { {log.title} }
                }
                p { {log.description} }
                div { class: "admin-operations-log-impact",
                    span { class: "admin-operations-log-impact-primary", {format!("Outcome: {}", log.impact)} }
                    span { class: "admin-operations-log-impact-separator", "•" }
                    span { class: "admin-operations-log-impact-secondary", {format!("Executed: {}", log.action_taken)} }
                }
            }
            time { {log.time} }
        }
    }
}
