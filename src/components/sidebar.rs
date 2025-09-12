use dioxus::prelude::*;
use dioxus_router::prelude::*;
use crate::Route;

#[component]
pub fn Sidebar() -> Element {
    rsx! {
        nav { 
            class: "sidebar",
            div { 
                class: "sidebar-header",
                h2 { "BurnCloud" }
            }
            ul { 
                class: "nav-menu",
                li { 
                    class: "nav-item",
                    Link { 
                        to: Route::Dashboard {},
                        class: "nav-link",
                        span { class: "nav-icon", "🏠" }
                        span { class: "nav-text", "仪表盘" }
                    }
                }
                li { 
                    class: "nav-item",
                    Link { 
                        to: Route::Models {},
                        class: "nav-link",
                        span { class: "nav-icon", "🧠" }
                        span { class: "nav-text", "模型管理" }
                    }
                }
                li { 
                    class: "nav-item",
                    Link { 
                        to: Route::Deploy {},
                        class: "nav-link",
                        span { class: "nav-icon", "⚙️" }
                        span { class: "nav-text", "部署配置" }
                    }
                }
                li { 
                    class: "nav-item",
                    Link { 
                        to: Route::Monitor {},
                        class: "nav-link",
                        span { class: "nav-icon", "📊" }
                        span { class: "nav-text", "监控日志" }
                    }
                }
                li { 
                    class: "nav-item",
                    Link { 
                        to: Route::Settings {},
                        class: "nav-link",
                        span { class: "nav-icon", "🔧" }
                        span { class: "nav-text", "设置" }
                    }
                }
            }
        }
    }
}