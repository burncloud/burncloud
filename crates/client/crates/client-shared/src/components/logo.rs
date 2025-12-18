use dioxus::prelude::*;

const LOGO_SVG: &str = include_str!("../../assets/logo.svg");

#[component]
pub fn Logo(class: Option<String>) -> Element {
    let class_name = class.unwrap_or_default();
    rsx! {
        div {
            class: "{class_name} [&>svg]:w-full [&>svg]:h-full",
            dangerous_inner_html: LOGO_SVG,
        }
    }
}
