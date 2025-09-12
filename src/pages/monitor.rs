use dioxus::prelude::*;
use crate::components::{sidebar::Sidebar, statusbar::StatusBar};

#[component]
pub fn Monitor() -> Element {
    rsx! {
        div { 
            class: "app-container",
            div { 
                class: "main-content",
                Sidebar {}
                div { 
                    class: "content-area",
                    div {
                        class: "monitor-page",
                        div { class: "page-header",
                            h1 { "📈 监控与日志" }
                            div { class: "header-actions",
                                button { class: "btn secondary", "📈 实时监控" }
                                button { class: "btn secondary", "📜 日志查看" }
                                button { class: "btn secondary", "📈 性能报告" }
                                button { class: "btn", "导出日志" }
                            }
                        }
                        div { class: "monitor-content",
                            p { "监控日志界面开发中..." }
                        }
                    }
                }
            }
            StatusBar { system_status: use_signal(|| crate::types::SystemStatus::default()) }
        }
    }
}