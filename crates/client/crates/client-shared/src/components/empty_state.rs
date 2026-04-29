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
            div { style: "font-size:15px; font-weight:600; color:var(--bc-text-primary)",
                "{title}"
            }
            if let Some(desc) = description {
                div { style: "font-size:13px; color:var(--bc-text-secondary)",
                    "{desc}"
                }
            }
            if let Some(cta) = cta {
                {cta}
            }
        }
    }
}