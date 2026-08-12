use dioxus::prelude::*;

use crate::{
    backend::{server_root, system_metrics, use_auth, SystemMetrics},
    components::Icon,
    functional_api::{cache_stats, clear_cache},
};

#[component]
pub fn Settings() -> Element {
    let auth = use_auth();
    let user = auth.user();
    let api_root = server_root();
    let mut metrics_resource = use_resource(move || async move { system_metrics().await });
    let mut cache_resource = use_resource(move || async move { cache_stats().await });
    let mut confirm_clear = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let metrics_result = metrics_resource.read().clone();
    let cache_result = cache_resource.read().clone();
    let metrics: SystemMetrics = metrics_result.clone().and_then(Result::ok).unwrap_or_default();
    let metrics_error = metrics_result.as_ref().and_then(|result| result.as_ref().err().cloned());
    let cache_text = match cache_result {
        Some(Ok(value)) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
        Some(Err(message)) => format!("Cache statistics unavailable: {message}"),
        None => "Loading cache statistics…".to_string(),
    };

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
                    p { class: "page-subtitle", "Inspect the connected BurnCloud environment and perform the maintenance operations the server actually supports." }
                }
                button {
                    class: "button button-secondary",
                    onclick: move |_| {
                        metrics_resource.restart();
                        cache_resource.restart();
                    },
                    "Refresh"
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

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

            div { class: "card card-pad stack-lg",
                div { class: "product-section-head",
                    div {
                        h3 { "Application cache" }
                        p { "Cache is an operational implementation detail, so raw server statistics are available on demand instead of dominating the page." }
                    }
                    button { class: "button button-ghost button-sm", onclick: move |_| cache_resource.restart(), "Refresh cache" }
                }
                details {
                    summary { class: "small strong", style: "cursor:pointer", "View raw cache statistics" }
                    pre { class: "terminal", style: "margin-top:12px;white-space:pre-wrap;max-height:300px;overflow:auto", "{cache_text}" }
                }
            }

            div { class: "card card-pad stack-lg danger-zone",
                div { class: "product-section-head",
                    div {
                        h3 { class: "danger", "Cache maintenance" }
                        p { "Clear application cache only when you are troubleshooting stale runtime state or following an operational procedure." }
                    }
                    span { class: "badge badge-error", "MAINTENANCE" }
                }
                p { class: "small muted", "Clearing cache does not delete customers, providers, API keys, or router logs, but it can temporarily change runtime behavior while caches warm again." }
                label { class: "row gap-2 small", style: "align-items:flex-start",
                    input { r#type: "checkbox", checked: confirm_clear(), onchange: move |_| confirm_clear.set(!confirm_clear()) }
                    span { "I understand this is a live maintenance operation on the connected BurnCloud server." }
                }
                button {
                    class: "button button-primary",
                    disabled: busy() || !confirm_clear(),
                    onclick: move |_| {
                        busy.set(true);
                        error.set(String::new());
                        notice.set("Clearing BurnCloud application cache…".to_string());
                        spawn(async move {
                            match clear_cache().await {
                                Ok(()) => {
                                    notice.set("BurnCloud application cache cleared.".to_string());
                                    confirm_clear.set(false);
                                    cache_resource.restart();
                                }
                                Err(message) => {
                                    notice.set(String::new());
                                    error.set(format!("Cache clear failed: {message}"));
                                }
                            }
                            busy.set(false);
                        });
                    },
                    if busy() { "Clearing…" } else { "Clear Application Cache" }
                }
            }

            div { class: "product-note",
                "General appearance preferences, arbitrary gateway defaults, and notification settings are intentionally absent because the current BurnCloud server does not expose a general settings CRUD API. The console only presents settings it can actually read or change."
            }
        }
    }
}
