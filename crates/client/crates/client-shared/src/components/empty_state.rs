use dioxus::prelude::*;

#[component]
pub fn EmptyState(
    icon: Element,
    title: String,
    description: Option<String>,
    cta: Option<Element>,
) -> Element {
    rsx! {
        div { class: "empty",
            div { class: "empty-icon",
                {icon}
            }
            div { class: "empty-title",
                "{title}"
            }
            if let Some(desc) = description {
                div { class: "empty-desc",
                    "{desc}"
                }
            }
            if let Some(cta) = cta {
                {cta}
            }
        }
    }
}