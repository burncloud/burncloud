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
        let mut is_maximized = use_signal(|| window.is_maximized());

        let mac_close = window.clone();
        let mac_min = window.clone();
        let mac_max = window.clone();

        let win_min = window.clone();
        let win_max = window.clone();
        let win_close = window.clone();

        rsx! {
            div { class: "bc-titlebar w-full flex items-center justify-between select-none app-drag-region",

                if cfg!(target_os = "macos") {
                    div { class: "flex items-center gap-bc-2 app-no-drag group pl-bc-3",
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
                                let next = !mac_max.is_maximized();
                                mac_max.set_maximized(next);
                            },
                            div { class: "opacity-0 group-hover:opacity-100", "＋" }
                        }
                    }
                }

                div { class: "flex-1 min-w-0" }

                if !cfg!(target_os = "macos") {
                    div { class: "bc-window-controls app-no-drag",
                        button {
                            class: "bc-window-control",
                            title: "Minimize",
                            aria_label: "Minimize window",
                            onclick: move |_| win_min.set_minimized(true),
                            span { class: "bc-win-icon", "\u{E921}" }
                        }
                        button {
                            class: "bc-window-control",
                            title: if is_maximized() { "Restore" } else { "Maximize" },
                            aria_label: if is_maximized() { "Restore window" } else { "Maximize window" },
                            onclick: move |_| {
                                let next = !is_maximized();
                                win_max.set_maximized(next);
                                is_maximized.set(next);
                            },
                            span {
                                class: "bc-win-icon",
                                if is_maximized() {
                                    "\u{E923}"
                                } else {
                                    "\u{E922}"
                                }
                            }
                        }
                        button {
                            class: "bc-window-control danger",
                            title: "Close",
                            aria_label: "Close window",
                            onclick: move |_| win_close.set_visible(false),
                            span { class: "bc-win-icon", "\u{E8BB}" }
                        }
                    }
                }
            }
        }
    }
}
