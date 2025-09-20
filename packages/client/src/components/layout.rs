#[cfg(feature = "gui")]
use dioxus::prelude::*;

#[cfg(feature = "gui")]
#[component]
pub fn Layout() -> Element {
    rsx! {
        div {
            style: "height: 100vh; display: flex; flex-direction: column;",
            "布局组件"
        }
    }
}