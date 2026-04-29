use crate::DesktopMode;
use dioxus::prelude::*;

#[cfg(feature = "desktop")]
use dioxus::desktop::use_window;

#[component]
pub fn TitleBar() -> Element {
    if try_use_context::<DesktopMode>().is_none() {
        return rsx! {};
    }

    #[cfg(not(feature = "desktop"))]
    {
        return rsx! {};
    }

    #[cfg(feature = "desktop")]
    {
        let window = use_window();

        let mac_close = window.clone();
        let mac_min = window.clone();
        let mac_max = window.clone();

        let win_min = window.clone();
        let win_max = window.clone();
        let win_close = window.clone();

        rsx! {
            div { class: "w-full h-8 flex items-center justify-between select-none app-drag-region px-4",

                if cfg!(target_os = "macos") {
                    div { class: "flex items-center gap-2 app-no-drag group",
                        button {
                            class: "w-3 h-3 rounded-full bg-macos-red border border-macos-red flex items-center justify-center text-xxxs text-macos-red-dark opacity-100 hover:opacity-100",
                            onclick: move |_| mac_close.set_visible(false),
                            div { class: "opacity-0 group-hover:opacity-100", "✕" }
                        }
                        button {
                            class: "w-3 h-3 rounded-full bg-macos-yellow border border-macos-yellow flex items-center justify-center text-xxxs text-macos-yellow-dark opacity-100 hover:opacity-100",
                            onclick: move |_| mac_min.set_minimized(true),
                            div { class: "opacity-0 group-hover:opacity-100", "−" }
                        }
                        button {
                            class: "w-3 h-3 rounded-full bg-macos-green border border-macos-green flex items-center justify-center text-xxxs text-macos-green-dark opacity-100 hover:opacity-100",
                            onclick: move |_| {
                                let is_max = mac_max.is_maximized();
                                mac_max.set_maximized(!is_max);
                            },
                            div { class: "opacity-0 group-hover:opacity-100", "＋" }
                        }
                    }
                }

                div { class: "flex-1" }

                if !cfg!(target_os = "macos") {
                    div { class: "flex items-center app-no-drag h-full mr-2",
                        button {
                            class: "h-full px-4 hover:bg-base-content/10 flex items-center justify-center transition-colors",
                            onclick: move |_| win_min.set_minimized(true),
                            svg { class: "w-4 h-4 opacity-70", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 14l-7 7m0 0l-7-7m7 7V3" } }
                        }
                        button {
                            class: "h-full px-4 hover:bg-base-content/10 flex items-center justify-center transition-colors",
                            onclick: move |_| {
                                 let is_max = win_max.is_maximized();
                                 win_max.set_maximized(!is_max);
                            },
                            svg { class: "w-4 h-4 opacity-70", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 8V4m0 0h4M4 4l5 5m11-1V4m0 0h-4m4 0l-5 5M4 16v4m0 0h4m-4 0l5-5m11 5l-5-5m5 5v-4m0 4h-4" } }
                        }
                        button {
                            class: "h-full px-4 hover:bg-red-500 hover:text-white flex items-center justify-center transition-colors",
                            onclick: move |_| win_close.set_visible(false),
                            svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M6 18L18 6M6 6l12 12" } }
                        }
                    }
                }
            }
        }
    }
}
