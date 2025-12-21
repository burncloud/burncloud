use crate::app::Route;
use dioxus::prelude::*;

#[component]
pub fn NotFoundPage(segments: Vec<String>) -> Element {
    rsx! {
        div { class: "flex flex-col items-center justify-center h-full text-base-content",
            h1 { class: "text-4xl font-bold mb-4", "404" }
            p { class: "text-lg opacity-60 mb-8", "页面未找到 (Page Not Found)" }
            Link {
                to: Route::Dashboard {},
                class: "btn btn-primary",
                "返回仪表盘"
            }
        }
    }
}
