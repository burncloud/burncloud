#[cfg(feature = "gui")]
use dioxus::prelude::*;

#[cfg(feature = "gui")]
#[component]
pub fn Dashboard() -> Element {
    rsx! {
        div {
            h1 { "仪表板" }
            p { "系统状态和统计信息" }
        }
    }
}