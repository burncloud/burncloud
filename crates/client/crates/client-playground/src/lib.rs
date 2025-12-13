use dioxus::prelude::*;

#[component]
pub fn PlaygroundPage() -> Element {
    rsx! {
        div { class: "flex flex-col h-full gap-4",
            h1 { class: "text-2xl font-bold", "演练场" }
            div { class: "p-4 border rounded", "Playground functionality coming soon..." }
        }
    }
}
