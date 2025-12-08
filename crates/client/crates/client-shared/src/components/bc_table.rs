use dioxus::prelude::*;

#[component]
pub fn BCTable(
    #[props(default)]
    class: String,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "table-container {class}",
            table {
                class: "table",
                {children}
            }
        }
    }
}
