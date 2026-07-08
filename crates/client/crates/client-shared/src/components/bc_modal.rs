use dioxus::prelude::*;

use crate::components::{BCButton, ButtonVariant};

#[component]
pub fn BCModal(
    #[props(default)] open: bool,
    #[props(default)] title: String,
    onclose: EventHandler<()>,
    /// Use wider panel (e.g. provider picker grids).
    #[props(default)]
    wide: bool,
    #[props(default)]
    footer: Option<Element>,
    children: Element,
) -> Element {
    if !open {
        return rsx! {};
    }

    let width_class = if wide { "max-w-2xl" } else { "max-w-lg" };

    rsx! {
        div { class: "bc-modal-overlay",

            div {
                class: "bc-modal-backdrop",
                onclick: move |_| onclose.call(()),
            }

            div {
                class: "bc-card-solid relative z-10 w-full {width_class} mx-bc-3 animate-scale-in flex flex-col max-h-[90vh]",
                onclick: |e| e.stop_propagation(),

                div { class: "flex items-center justify-between p-bc-4 border-b border-bc-border shrink-0",
                    h3 { class: "text-subtitle font-bold text-bc-text m-0", "{title}" }
                    BCButton {
                        variant: ButtonVariant::Ghost,
                        class: "btn-subtle w-8 h-8 flex items-center justify-center rounded-full text-bc-lg".to_string(),
                        onclick: move |_| onclose.call(()),
                        "✕"
                    }
                }

                div { class: "p-bc-4 overflow-y-auto flex-1 min-h-0",
                    {children}
                }

                if let Some(footer) = footer {
                    div { class: "flex justify-end gap-bc-3 px-bc-4 py-bc-3 border-t border-bc-border shrink-0 bc-modal-footer-bg",
                        {footer}
                    }
                }
            }
        }
    }
}
