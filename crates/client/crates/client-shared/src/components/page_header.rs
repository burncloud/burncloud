use dioxus::prelude::*;

#[component]
pub fn PageHeader(
    title: String,
    subtitle: Option<String>,
    subtitle_class: Option<String>,
    actions: Option<Element>,
) -> Element {
    let sub_cls = subtitle_class.unwrap_or_default();
    rsx! {
        div { class: "page-header",
            div { class: "flex-1",
                div { class: "page-title", "{title}" }
                if let Some(sub) = subtitle {
                    div { class: "page-sub {sub_cls}", "{sub}" }
                }
            }
            if let Some(actions) = actions {
                div { class: "flex gap-sm items-center",
                    {actions}
                }
            }
        }
    }
}
