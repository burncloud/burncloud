use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::app::Route;

#[component]
pub fn Sidebar() -> Element {
    rsx! {
        nav { class: "sidebar",
            div { class: "p-lg",
                div { class: "flex items-center gap-md mb-xl",
                    div { class: "text-title font-bold text-primary",
                        "🔥 BurnCloud"
                    }
                }

                div { class: "flex flex-col gap-xs",
                    Link {
                        to: Route::Dashboard {},
                        class: "nav-item",
                        span { class: "icon", "🏠" }
                        span { "仪表盘" }
                    }
                    Link {
                        to: Route::ModelManagement {},
                        class: "nav-item",
                        span { class: "icon", "🧠" }
                        span { "模型管理" }
                    }
                    Link {
                        to: Route::DeployConfig {},
                        class: "nav-item",
                        span { class: "icon", "⚙️" }
                        span { "部署配置" }
                    }
                    Link {
                        to: Route::ServiceMonitor {},
                        class: "nav-item",
                        span { class: "icon", "📊" }
                        span { "监控日志" }
                    }
                    Link {
                        to: Route::ApiManagement {},
                        class: "nav-item",
                        span { class: "icon", "🔗" }
                        span { "API管理" }
                    }
                    Link {
                        to: Route::SystemSettings {},
                        class: "nav-item",
                        span { class: "icon", "🔧" }
                        span { "系统设置" }
                    }
                }
            }

            div { class: "mt-auto p-lg border-t",
                style: "border-color: var(--neutral-quaternary);",
                div { class: "text-caption text-secondary",
                    "状态: 2个模型运行中"
                }
                div { class: "text-caption text-secondary",
                    "CPU: 45% | 内存: 2.1GB"
                }
            }
        }
    }
}