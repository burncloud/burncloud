use dioxus::prelude::*;

#[component]
pub fn BCTable(
    #[props(default)] class: String,
    #[props(default)] pagination: Option<Element>,
    children: Element,
) -> Element {
    rsx! {
        div {
            class: "table-container {class}",
            table {
                class: "table",
                {children}
            }
            if let Some(pag) = pagination {
                div { class: "table-pagination p-md border-t border-subtle flex justify-end items-center gap-sm",
                    {pag}
                }
            }
        }
    }
}

#[component]
pub fn BCPagination(page: usize, total_pages: usize, on_change: EventHandler<usize>) -> Element {
    rsx! {
        div { class: "flex items-center gap-sm",
            button {
                class: "btn btn-sm btn-secondary",
                disabled: page <= 1,
                onclick: move |_| on_change.call(page - 1),
                "Prev"
            }
            span { class: "text-caption text-secondary", "Page {page} of {total_pages}" }
            button {
                class: "btn btn-sm btn-secondary",
                disabled: page >= total_pages,
                onclick: move |_| on_change.call(page + 1),
                "Next"
            }
        }
    }
}
