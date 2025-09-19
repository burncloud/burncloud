use dioxus::prelude::*;

#[component]
pub fn TitleBar() -> Element {
    rsx! {
        div { class: "title-bar",
            div { class: "flex items-center gap-md",
                span { class: "text-title font-semibold",
                    "BurnCloud - 大模型本地部署平台"
                }
            }
            div { class: "flex items-center gap-xs",
                button { class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px;",
                    "—"
                }
                button { class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px;",
                    "☐"
                }
                button { class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px; color: #d13438;",
                    onmouseenter: move |_| {
                        // TODO: Handle hover state
                    },
                    "✕"
                }
            }
        }
    }
}