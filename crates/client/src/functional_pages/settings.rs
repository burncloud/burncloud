use dioxus::prelude::*;

use crate::{
    backend::{server_root, system_metrics, use_auth, SystemMetrics},
    components::Icon,
    functional_api::{cache_stats, clear_cache},
};

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.2} GiB", bytes as f64 / 1024.0 / 1024.0 / 1024.0)
    } else if bytes >= 1024 * 1024 {
        format!("{:.2} MiB", bytes as f64 / 1024.0 / 1024.0)
    } else if bytes >= 1024 {
        format!("{:.2} KiB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

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
    let metrics_error = metrics_result
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let (environment_badge_class, environment_badge, environment_note) = match metrics_result.as_ref() {
        None => (
            "badge badge-neutral",
            "CHECKING",
            "Verifying the configured server with live runtime telemetry.",
        ),
        Some(Ok(_)) => (
            "badge badge-success",
            "REACHABLE",
            "The configured server responded to the latest runtime telemetry request.",
        ),
        Some(Err(_)) => (
            "badge badge-warning",
            "UNVERIFIED",
            "The endpoint is configured, but the latest runtime check did not succeed.",
        ),
    };

    let cache_value = cache_result
        .as_ref()
        .and_then(|result| result.as_ref().ok());
    let cache_error = cache_result
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let cache_enabled = cache_value
        .and_then(|value| value.get("enabled"))
        .and_then(|value| value.as_bool());
    let cache_connected = cache_value
        .and_then(|value| value.get("connected"))
        .and_then(|value| value.as_bool());
    let cache_key_count = cache_value
        .and_then(|value| value.get("key_count"))
        .and_then(|value| value.as_u64());
    let cache_memory = cache_value
        .and_then(|value| value.get("memory_usage"))
        .and_then(|value| value.as_u64());
    let cache_operational = cache_enabled == Some(true) && cache_connected == Some(true);

    let (cache_badge_class, cache_badge, cache_note) = if cache_result.is_none() {
        (
            "badge badge-neutral",
            "CHECKING",
            "Loading the server's Redis cache state before maintenance is allowed.".to_string(),
        )
    } else if cache_error.is_some() {
        (
            "badge badge-warning",
            "UNAVAILABLE",
            "Cache state could not be verified, so destructive cache maintenance is blocked.".to_string(),
        )
    } else if cache_enabled == Some(false) {
        (
            "badge badge-neutral",
            "DISABLED",
            "Application caching is disabled. There is no active Redis cache namespace to clear.".to_string(),
        )
    } else if cache_connected == Some(false) {
        (
            "badge badge-warning",
            "DISCONNECTED",
            "Caching is configured but Redis is not currently reported as connected. Maintenance is blocked.".to_string(),
        )
    } else if cache_operational {
        (
            "badge badge-success",
            "AVAILABLE",
            "Redis cache is enabled and connected. Token, channel, price, and quota cache entries may be present.".to_string(),
        )
    } else {
        (
            "badge badge-warning",
            "UNKNOWN",
            "The cache response did not contain enough state to safely enable maintenance.".to_string(),
        )
    };

    let cache_keys_text = cache_key_count
        .map(|value| value.to_string())
        .unwrap_or_else(|| "—".to_string());
    let cache_memory_text = cache_memory
        .map(format_bytes)
        .unwrap_or_else(|| "—".to_string());
    let cache_text = match cache_result.clone() {
        Some(Ok(value)) => serde_json::to_string_pretty(&value).unwrap_or_else(|_| value.to_string()),
        Some(Err(message)) => format!("Cache statistics unavailable: {message}"),
        None => "Loading cache statistics…".to_string(),
    };

    let roles = user
        .as_ref()
        .map(|user| user.roles.join(", "))
        .unwrap_or_else(|| "-".to_string());
    let username = user
        .as_ref()
        .map(|user| user.username.clone())
        .unwrap_or_else(|| "-".to_string());
    let user_id = user
        .as_ref()
        .map(|user| user.id.clone())
        .unwrap_or_else(|| "-".to_string());
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
                    p { class: "page-subtitle", "Inspect the configured BurnCloud environment, verify runtime reachability, and perform only maintenance the server can verify." }
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
                            p { "The endpoint and identity this console is configured to use." }
                        }
                        span { class: "{environment_badge_class}", "{environment_badge}" }
                    }
                    div { class: "product-note", "{environment_note}" }
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
                        div { class: "stack",
                            strong { class: "danger", "Runtime telemetry unavailable" }
                            code { class: "terminal", "{message}" }
                            button { class: "button button-secondary button-sm", onclick: move |_| metrics_resource.restart(), "Retry runtime check" }
                        }
                    } else if metrics_result.is_none() {
                        p { class: "small muted", "Checking runtime telemetry…" }
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
                        p { "Redis cache state is shown as an operational dependency rather than raw implementation noise." }
                    }
                    div { class: "row gap-2",
                        span { class: "{cache_badge_class}", "{cache_badge}" }
                        button { class: "button button-ghost button-sm", onclick: move |_| cache_resource.restart(), "Refresh cache" }
                    }
                }
                div { class: "product-note", "{cache_note}" }
                div { class: "grid-2",
                    div { class: "receipt-row", label { "BurnCloud cache keys" } strong { class: "mono", "{cache_keys_text}" } }
                    div { class: "receipt-row", label { "Redis memory" } strong { class: "mono", "{cache_memory_text}" } }
                }
                p { class: "small muted", "The current cache namespace covers BurnCloud token, channel, price, and quota cache entries under bc:*; durable customer, provider, API-key, billing, and router-log records remain in the database." }
                details {
                    summary { class: "small strong", style: "cursor:pointer", "View raw cache statistics" }
                    pre { class: "terminal", style: "margin-top:12px;white-space:pre-wrap;max-height:300px;overflow:auto", "{cache_text}" }
                }
            }

            div { class: "card card-pad stack-lg danger-zone",
                div { class: "product-section-head",
                    div {
                        h3 { class: "danger", "Cache maintenance" }
                        p { "Clear BurnCloud's Redis cache namespace only when troubleshooting stale cached state or following an operational procedure." }
                    }
                    span { class: "badge badge-error", "MAINTENANCE" }
                }
                if !cache_operational {
                    div { class: "readiness-strip blocked",
                        span { class: "readiness-dot" }
                        strong { "Cache clear is unavailable" }
                        span { class: "muted", "BurnCloud requires cache stats to confirm that Redis caching is enabled and connected before allowing this operation." }
                    }
                }
                p { class: "small muted", "Clear Application Cache deletes BurnCloud's bc:* Redis cache keys. It does not delete durable customers, providers, API keys, billing records, or router logs, but subsequent requests may repopulate cached token, channel, price, and quota data." }
                label { class: "row gap-2 small", style: "align-items:flex-start",
                    input {
                        r#type: "checkbox",
                        checked: confirm_clear(),
                        disabled: !cache_operational || busy(),
                        onchange: move |_| confirm_clear.set(!confirm_clear()),
                    }
                    span { "I understand this deletes the live BurnCloud Redis cache namespace on the configured server." }
                }
                button {
                    class: "button button-primary",
                    disabled: busy() || !cache_operational || !confirm_clear(),
                    onclick: move |_| {
                        if !cache_operational {
                            error.set("Cache state is not enabled and connected; maintenance was not sent.".to_string());
                            return;
                        }
                        busy.set(true);
                        error.set(String::new());
                        notice.set("Clearing BurnCloud Redis cache namespace…".to_string());
                        spawn(async move {
                            match clear_cache().await {
                                Ok(()) => {
                                    notice.set("BurnCloud Redis cache clear completed. Cache statistics are being refreshed.".to_string());
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
