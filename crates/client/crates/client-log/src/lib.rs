use dioxus::prelude::*;

#[component]
pub fn LogPage() -> Element {
    rsx! {
        div { class: "flex flex-col h-full gap-4",
            h1 { class: "text-2xl font-bold", "日志审查" }
            div { class: "p-4 border rounded", "日志审查功能开发中..." }
        }
    }
}
