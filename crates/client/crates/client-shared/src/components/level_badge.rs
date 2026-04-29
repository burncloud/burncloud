use dioxus::prelude::*;

#[component]
pub fn LevelBadge(value: String) -> Element {
    let class = match value.to_uppercase().as_str() {
        "ERROR" | "ERR" | "FATAL" => "danger",
        "WARN" | "WARNING" => "warning",
        "INFO" => "info",
        "DEBUG" | "TRACE" => "neutral",
        _ => "neutral",
    };

    rsx! {
        span { class: "pill {class}",
            span { class: "dot" }
            "{value}"
        }
    }
}
