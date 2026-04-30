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
            div { style: "flex:1",
                div { class: "page-title", "{title}" }
                if let Some(sub) = subtitle {
                    div { class: "page-sub {sub_cls}", "{sub}" }
                }
            }
            if let Some(actions) = actions {
                div { style: "display:flex; gap:8px; align-items:center",
                    {actions}
                }
            }
        }
    }
}
