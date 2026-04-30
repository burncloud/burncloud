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
        span { class: "pill {class}", style: "font-family:var(--bc-font-mono); font-size:11px; padding:2px 8px; letter-spacing:0.04em",
            "{value}"
        }
    }
}
