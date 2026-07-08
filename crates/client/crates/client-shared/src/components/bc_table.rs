use dioxus::prelude::*;

use crate::components::{BCButton, ButtonVariant};

#[component]
pub fn BCTable(
    #[props(default)] class: String,
    #[props(default)] pagination: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "bc-card-solid overflow-hidden {class}",
            div { class: "overflow-x-auto",
                table {
                    class: "w-full",
                    {children}
                }
            }
            if let Some(pag) = pagination {
                div { class: "p-bc-3 border-t border-bc-border flex justify-end items-center gap-bc-2",
                    {pag}
                }
            }
        }
    }
}

#[component]
pub fn BCPagination(page: usize, total_pages: usize, on_change: EventHandler<usize>) -> Element {
    rsx! {
        div { class: "flex items-center gap-bc-2",
            BCButton {
                variant: ButtonVariant::Secondary,
                disabled: page <= 1,
                onclick: move |_| on_change.call(page - 1),
                "Prev"
            }
            span { class: "text-caption text-bc-text-secondary", "Page {page} of {total_pages}" }
            BCButton {
                variant: ButtonVariant::Secondary,
                disabled: page >= total_pages,
                onclick: move |_| on_change.call(page + 1),
                "Next"
            }
        }
    }
}
