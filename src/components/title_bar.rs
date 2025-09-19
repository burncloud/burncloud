use dioxus::prelude::*;

#[component]
pub fn TitleBar() -> Element {
    let window = dioxus::desktop::use_window();

    rsx! {
        div { class: "title-bar",
            div { class: "flex items-center gap-md",
                span { class: "text-title font-semibold",
                    "BurnCloud - 大模型本地部署平台"
                }
            }
            div { class: "flex items-center gap-xs",
                button {
                    class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px;",
                    onclick: {
                        let window = window.clone();
                        move |_| {
                            window.set_minimized(true);
                        }
                    },
                    "—"
                }
                button {
                    class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px;",
                    onclick: {
                        let window = window.clone();
                        move |_| {
                            if window.is_maximized() {
                                window.set_maximized(false);
                            } else {
                                window.set_maximized(true);
                            }
                        }
                    },
                    "☐"
                }
                button {
                    class: "btn btn-subtle",
                    style: "min-height: 28px; padding: 4px 8px; color: #d13438;",
                    onclick: {
                        let window = window.clone();
                        move |_| {
                            window.close();
                        }
                    },
                    onmouseenter: move |_| {
                        // TODO: Handle hover state
                    },
                    "✕"
                }
            }
        }
    }
}