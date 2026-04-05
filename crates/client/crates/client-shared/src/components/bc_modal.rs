use dioxus::prelude::*;

#[component]
pub fn BCModal(
    #[props(default)] open: bool,
    #[props(default)] title: String,
    onclose: EventHandler<()>,
    children: Element,
) -> Element {
    let display_style = if open { "flex" } else { "none" };

    rsx! {
        div {
            class: "fixed inset-0 z-50 flex items-center justify-center",
            style: "display: {display_style}",

            // Backdrop
            div {
                class: "absolute inset-0 bg-[rgba(0,0,0,0.4)] backdrop-blur-sm",
                onclick: move |_| onclose.call(()),
            }

            // Modal content
            div {
                class: "bc-card-solid relative z-10 w-full max-w-lg mx-md animate-scale-in",
                onclick: |e| e.stop_propagation(),

                // Header
                div { class: "flex items-center justify-between p-lg border-b border-[var(--bc-border)]",
                    h3 { class: "text-subtitle font-bold text-primary m-0", "{title}" }
                    button {
                        class: "btn-subtle w-8 h-8 flex items-center justify-center rounded-full text-lg",
                        onclick: move |_| onclose.call(()),
                        "✕"
                    }
                }

                // Body
                div { class: "p-lg",
                    {children}
                }
            }
        }
    }
}
