use dioxus::prelude::*;

#[component]
pub fn BurnGridPage() -> Element {
    rsx! { 
        div { class: "flex flex-col h-full gap-4",
            h1 { class: "text-2xl font-bold", "BurnGrid" }
            div { class: "p-4 border rounded", "BurnGrid functionality coming soon..." }
        }
    }
}
