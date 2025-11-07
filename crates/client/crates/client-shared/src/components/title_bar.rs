use dioxus::prelude::*;

#[component]
pub fn TitleBar() -> Element {
    rsx! {
        div {
            class: "title-bar",
            style: "-webkit-app-region: drag;",
            div { class: "flex items-center gap-md",
                span { class: "text-title font-semibold",
                    "BurnCloud - 大模型本地部署平台"
                }
            }
            div {
                class: "flex items-center gap-xs",
                style: "-webkit-app-region: no-drag;",
                button {
                    class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px;",
                    onclick: move |_| {
                        dioxus::desktop::window().set_minimized(true);
                    },
                    "—"
                }
                button {
                    class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px;",
                    onclick: move |_| {
                        let window = dioxus::desktop::window();
                        let is_maximized = window.is_maximized();
                        window.set_maximized(!is_maximized);
                    },
                    "☐"
                }
                button {
                    class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px; color: #d13438;",
                    onclick: move |_| {
                        
                        dioxus::desktop::window().set_visible(false);
                        //dioxus::desktop::window().close();
                    },
                    "✕"
                }
            }
        }
    }
}