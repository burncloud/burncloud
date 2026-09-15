use dioxus::prelude::*;

use crate::{
    backend::{server_root, system_metrics, use_auth, SystemMetrics},
    components::Icon,
};

#[component]
pub fn Settings() -> Element {
    let auth = use_auth();
    let user = auth.user();
    let api_root = server_root();
    let mut metrics_resource = use_resource(move || async move { system_metrics().await });

    let metrics_result = metrics_resource.read().clone();
    let metrics: SystemMetrics = metrics_result.clone().and_then(Result::ok).unwrap_or_default();
    let metrics_error = metrics_result.as_ref().and_then(|result| result.as_ref().err().cloned());
    let roles = user.as_ref().map(|user| user.roles.join(", ")).unwrap_or_else(|| "-".to_string());
    let username = user.as_ref().map(|user| user.username.clone()).unwrap_or_else(|| "-".to_string());
    let user_id = user.as_ref().map(|user| user.id.clone()).unwrap_or_else(|| "-".to_string());
    let memory_used_gib = metrics.memory.used as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_total_gib = metrics.memory.total as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_text = format!("{memory_used_gib:.1} / {memory_total_gib:.1} GiB");
    let cpu_text = format!("{:.1}%", metrics.cpu.usage_percent);
    let memory_percent = format!("{:.1}%", metrics.memory.usage_percent);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Settings" }
                    p { class: "page-subtitle", "Inspect the connected BurnCloud environment and runtime health." }
                }
                button {
                    class: "button button-secondary",
                    onclick: move |_| {
                        metrics_resource.restart();
                    },
                    "Refresh"
                }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack-lg",
                    div { class: "product-section-head",
                        div {
                            h3 { "Environment" }
                            p { "The server endpoint and identity this console is currently using." }
                        }
                        span { class: "badge badge-success", "CONNECTED" }
                    }
                    div { class: "receipt-row", label { "Server" } strong { class: "mono", "{api_root}" } }
                    div { class: "receipt-row", label { "Signed in as" } strong { "{username}" } }
                    div { class: "receipt-row", label { "Roles" } strong { "{roles}" } }
                    details {
                        summary { class: "small strong", style: "cursor:pointer", "Connection details" }
                        div { class: "stack", style: "margin-top:12px",
                            div { class: "receipt-row", label { "User ID" } strong { class: "mono", "{user_id}" } }
                            div { class: "product-note", "Set BURNCLOUD_API_BASE to point the client at a different BurnCloud server. Otherwise the client uses the configured local host/port behavior." }
                        }
                    }
                }

                div { class: "card card-pad stack-lg",
                    div { class: "product-section-head",
                        div {
                            h3 { "Runtime health" }
                            p { "Host pressure can explain latency or capacity issues, but it is not the same as routing health." }
                        }
                        Icon { name: "activity" }
                    }
                    if let Some(message) = metrics_error {
                        code { class: "terminal", "{message}" }
                    } else {
                        div { class: "grid-2",
                            div { class: "receipt-row", label { "CPU" } strong { class: "mono", "{cpu_text}" } }
                            div { class: "receipt-row", label { "Memory" } strong { class: "mono", "{memory_percent}" } }
                        }
                        div { class: "receipt-row", label { "CPU cores" } strong { "{metrics.cpu.core_count}" } }
                        div { class: "receipt-row", label { "Memory used" } strong { "{memory_text}" } }
                        div { class: "receipt-row", label { "Mounted disks" } strong { "{metrics.disks.len()}" } }
                    }
                }
            }

            div { class: "product-note",
                "General appearance preferences, arbitrary gateway defaults, and notification settings are intentionally absent because the current BurnCloud server does not expose a general settings CRUD API. The console only presents settings it can actually read or change."
            }
        }
    }
}
