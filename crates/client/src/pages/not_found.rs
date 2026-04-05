use crate::app::Route;
use burncloud_client_shared::components::bc_button::{BCButton, ButtonVariant};
use dioxus::prelude::*;

#[component]
pub fn NotFoundPage(segments: Vec<String>) -> Element {
    let navigator = use_navigator();

    rsx! {
        div { class: "flex flex-col items-center justify-center h-full",
            h1 { class: "text-display font-bold mb-lg text-primary", "404" }
            p { class: "text-subtitle text-secondary mb-xxxl", "页面未找到 (Page Not Found)" }
            BCButton {
                variant: ButtonVariant::Primary,
                onclick: move |_| {
                    navigator.push(Route::Dashboard {});
                },
                "返回仪表盘"
            }
        }
    }
}
