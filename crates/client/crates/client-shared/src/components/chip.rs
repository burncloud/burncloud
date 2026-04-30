use dioxus::prelude::*;

#[component]
pub fn Chip(
    label: String,
    count: Option<i64>,
    active: Option<bool>,
    onclick: EventHandler<()>,
) -> Element {
    let is_active = active.unwrap_or(false);
    let active_class = if is_active { "active" } else { "" };

    rsx! {
        button {
            class: "chip {active_class}",
            onclick: move |_| onclick.call(()),
            "{label}"
            if let Some(c) = count {
                span { class: "chip-count", "{c}" }
            }
        }
    }
}

#[component]
pub fn ChipRow(max_visible: Option<usize>, children: Element) -> Element {
    rsx! {
        div { class: "chip-row",
            {children}
        }
    }
}
