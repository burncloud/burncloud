use dioxus::prelude::*;

use crate::{
    backend::{server_root, system_metrics, use_auth, SystemMetrics},
    functional_api::{cache_stats, clear_cache},
    components::Icon,
};

#[component]
pub fn Settings() -> Element {
    let auth = use_auth();
    let user = auth.user();
    let api_root = server_root();
    let mut metrics_resource = use_resource(move || async move { system_metrics().await });
    let mut cache_resource = use_resource(move || async move { cache_stats().await });
    let mut busy = use_signal(|| false);
    let mut notice = use_signal(String::new);
    let mut error = use_signal(String::new);

    let metrics_result = metrics_resource.read().clone();
    let cache_result = cache_resource.read().clone();
    let metrics: SystemMetrics = metrics_result.clone().and_then(Result::ok).unwrap_or_default();
    let metrics_error = metrics_result.as_ref().and_then(|r| r.as_ref().err().cloned());
    let cache_text = match cache_result {
        Some(Ok(value)) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
        Some(Err(message)) => format!("Cache statistics unavailable: {message}"),
        None => "Loading cache statistics…".to_string(),
    };
    let roles = user.as_ref().map(|u| u.roles.join(", ")).unwrap_or_else(|| "-".to_string());
    let username = user.as_ref().map(|u| u.username.clone()).unwrap_or_else(|| "-".to_string());
    let user_id = user.as_ref().map(|u| u.id.clone()).unwrap_or_else(|| "-".to_string());
    let memory_used_gib = metrics.memory.used as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_total_gib = metrics.memory.total as f64 / 1024.0 / 1024.0 / 1024.0;
    let memory_text = format!("{memory_used_gib:.1} / {memory_total_gib:.1} GiB");
    let cpu_text = format!("{:.1}%", metrics.cpu.usage_percent);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Settings" }
                    p { class: "page-subtitle", "Only settings and maintenance operations backed by the current BurnCloud server are shown as actionable." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        onclick: move |_| {
                            metrics_resource.restart();
                            cache_resource.restart();
                        },
                        "Refresh"
                    }
                }
            }

            if !notice().is_empty() { div { class: "terminal auth-status", "{notice}" } }
            if !error().is_empty() { div { class: "terminal auth-status auth-status-error", "{error}" } }

            div { class: "grid-2",
                div { class: "card card-pad stack-lg",
                    div { class: "row between",
                        h3 { "Server Connection" }
                        span { class: "badge badge-success", "CONNECTED CONFIG" }
                    }
                    div { class: "receipt-row", label { "API Root" } strong { class: "mono", "{api_root}" } }
                    div { class: "receipt-row", label { "Username" } strong { "{username}" } }
                    div { class: "receipt-row", label { "User ID" } strong { class: "mono", "{user_id}" } }
                    div { class: "receipt-row", label { "Roles" } strong { "{roles}" } }
                    p { class: "tiny subtle", "Override the local server root with BURNCLOUD_API_BASE. Otherwise the client uses 127.0.0.1 and BurnCloud's PORT/default port." }
                }

                div { class: "card card-pad stack-lg",
                    div { class: "row between",
                        h3 { "Runtime Health" }
                        Icon { name: "activity" }
                    }
                    if let Some(message) = metrics_error {
                        code { class: "terminal", "{message}" }
                    } else {
                        div { class: "receipt-row", label { "CPU" } strong { "{cpu_text}" } }
                        div { class: "receipt-row", label { "CPU Cores" } strong { "{metrics.cpu.core_count}" } }
                        div { class: "receipt-row", label { "Memory" } strong { "{memory_text}" } }
                        div { class: "receipt-row", label { "Disks" } strong { "{metrics.disks.len()} mounted entries" } }
                    }
                }
            }

            div { class: "grid-2",
                div { class: "card card-pad stack",
                    div { class: "row between",
                        h3 { "Router Cache" }
                        button { class: "button button-ghost button-sm", onclick: move |_| cache_resource.restart(), "Refresh Stats" }
                    }
                    pre { class: "terminal", style: "white-space:pre-wrap;max-height:300px;overflow:auto", "{cache_text}" }
                }

                div { class: "card card-pad stack",
                    h3 { class: "danger", "Cache Maintenance" }
                    p { class: "small muted", "Clear all BurnCloud application cache through the real /console/api/cache/clear endpoint. This does not delete users, channels, API keys or router logs." }
                    button {
                        class: "button button-primary",
                        disabled: busy(),
                        onclick: move |_| {
                            busy.set(true);
                            error.set(String::new());
                            notice.set("Clearing BurnCloud cache…".to_string());
                            spawn(async move {
                                match clear_cache().await {
                                    Ok(()) => {
                                        notice.set("BurnCloud cache cleared successfully.".to_string());
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
            }

            div { class: "card card-pad stack",
                h3 { "Unavailable Configuration" }
                p { class: "small muted", "The current BurnCloud server does not expose a general system-settings CRUD endpoint. Appearance, arbitrary gateway defaults, notification preferences and similar prototype controls are therefore not shown as fake save actions." }
            }
        }
    }
}
