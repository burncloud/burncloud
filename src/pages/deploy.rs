use dioxus::prelude::*;
use crate::components::{sidebar::Sidebar, statusbar::StatusBar};

#[component]
pub fn Deploy() -> Element {
    rsx! {
        div { 
            class: "app-container",
            div { 
                class: "main-content",
                Sidebar {}
                div { 
                    class: "content-area",
                    div {
                        class: "models-page",
                        div { class: "page-header",
                            h1 { "⚙️ 部署配置" }
                            div { class: "header-actions",
                                button { class: "btn primary", "🚀 快速部署" }
                                button { class: "btn secondary", "📝 新建配置" }
                                button { class: "btn secondary", "📂 导入配置" }
                                div { class: "search-box",
                                    input {
                                        r#type: "text",
                                        placeholder: "搜索配置...",
                                    }
                                }
                            }
                        }
                        div { class: "deploy-content",
                            div { class: "config-section",
                                h2 { "📡 服务配置" }
                                p { "部署配置界面开发中..." }
                            }
                        }
                    }
                }
            }
            StatusBar { system_status: use_signal(|| crate::types::SystemStatus::default()) }
        }
    }
}