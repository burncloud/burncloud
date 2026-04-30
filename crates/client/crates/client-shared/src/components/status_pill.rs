use dioxus::prelude::*;

#[component]
pub fn StatusPill(value: String, label: Option<String>) -> Element {
    let (class, display) = match value.to_lowercase().as_str() {
        "active" | "running" | "ok" | "success" => ("success", label.unwrap_or_else(|| value.clone())),
        "throttle" | "rate_limited" | "warning" => ("warning", label.unwrap_or_else(|| value.clone())),
        "down" | "error" | "failed" | "danger" => ("danger", label.unwrap_or_else(|| value.clone())),
        "maint" | "maintenance" | "info" => ("info", label.unwrap_or_else(|| value.clone())),
        "neutral" | "disabled" | "revoked" => ("neutral", label.unwrap_or_else(|| value.clone())),
        _ => ("neutral", label.unwrap_or_else(|| value.clone())),
    };
    let pulse = class == "success";

    rsx! {
        span { class: "pill {class}",
            span { class: if pulse { "dot pulse" } else { "dot" } }
            "{display}"
        }
    }
}
