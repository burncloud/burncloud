use dioxus::prelude::*;

#[component]
pub fn BCCard(
    #[props(default)] header: Option<String>,
    #[props(default)] class: String,
    children: Element,
) -> Element {
    rsx! {
        div { class: "card shadow-sm {class}",
            if let Some(h) = header {
                div { class: "card-header bg-transparent border-bottom-0 pt-3 pb-0",
                    h3 { class: "card-title h5 mb-0", "{h}" }
                }
            }
            div { class: "card-body",
                {children}
            }
        }
    }
}
