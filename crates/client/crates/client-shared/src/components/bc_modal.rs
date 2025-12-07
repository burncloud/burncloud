use dioxus::prelude::*;

#[component]
pub fn BCModal(
    #[props(default)]
    open: bool,
    #[props(default)]
    title: String,
    onclose: EventHandler<()>,
    children: Element,
) -> Element {
    if !open {
        return rsx! {};
    }

    rsx! {
        div { 
            class: "modal-overlay", 
            onclick: move |_| onclose.call(()), // Click backdrop to close
            
            div { 
                class: "modal-content",
                onclick: |e| e.stop_propagation(), // Prevent click inside modal from closing it
                
                div { class: "modal-header",
                    h3 { class: "modal-title-text text-title font-bold m-0", "{title}" }
                    button { 
                        class: "btn-icon", 
                        onclick: move |_| onclose.call(()),
                        "✕"
                    }
                }
                div { class: "modal-body",
                    {children}
                }
            }
        }
    }
}
