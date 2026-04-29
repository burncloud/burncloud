use dioxus::prelude::*;

#[component]
pub fn StatusPill(value: String) -> Element {
    let (class, label) = match value.to_lowercase().as_str() {
        "active" | "running" | "ok" => ("success", value.clone()),
        "throttle" | "rate_limited" => ("warning", value.clone()),
        "down" | "error" | "failed" => ("danger", value.clone()),
        "maint" | "maintenance" => ("info", value.clone()),
        _ => ("neutral", value.clone()),
    };

    rsx! {
        span { class: "pill {class}",
            span { class: "dot" }
            "{label}"
        }
    }
}
