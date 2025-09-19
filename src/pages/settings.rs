use dioxus::prelude::*;
use crate::components::{sidebar::Sidebar, statusbar::StatusBar};

#[component]
pub fn Settings() -> Element {
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
                            h1 { "🔧 系统设置" }
                            div { class: "header-actions",
                                button { class: "btn primary", "💾 保存设置" }
                                button { class: "btn secondary", "🔄 恢复默认" }
                                button { class: "btn secondary", "📁 导入配置" }
                                div { class: "search-box",
                                    input {
                                        r#type: "text",
                                        placeholder: "搜索设置...",
                                    }
                                }
                            }
                        }
                        div { class: "settings-content",
                            div { class: "settings-tabs",
                                button { class: "tab-btn active", "🎨 外观" }
                                button { class: "tab-btn", "🔧 系统" }
                                button { class: "tab-btn", "🔒 安全" }
                                button { class: "tab-btn", "📚 关于" }
                            }
                            div { class: "settings-panel",
                                p { "设置界面开发中..." }
                            }
                        }
                    }
                }
            }
            StatusBar { system_status: use_signal(|| crate::types::SystemStatus::default()) }
        }
    }
}