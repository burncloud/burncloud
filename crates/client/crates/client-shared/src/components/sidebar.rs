use dioxus::prelude::*;

use super::layout::CoreRoute;

#[component]
pub fn Sidebar() -> Element {
    let route = use_route::<CoreRoute>();

    rsx! {
        nav { class: "sidebar",
            div { class: "p-lg",
                div { class: "flex flex-col gap-xs",
                    Link {
                        to: CoreRoute::Dashboard {},
                        class: if matches!(route, CoreRoute::Dashboard {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "🏠" }
                        span { "仪表盘" }
                    }
                    Link {
                        to: CoreRoute::ModelManagement {},
                        class: if matches!(route, CoreRoute::ModelManagement {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "🧠" }
                        span { "模型管理" }
                    }
                    Link {
                        to: CoreRoute::DeployConfig {},
                        class: if matches!(route, CoreRoute::DeployConfig {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "⚙️" }
                        span { "部署配置" }
                    }
                    Link {
                        to: CoreRoute::ServiceMonitor {},
                        class: if matches!(route, CoreRoute::ServiceMonitor {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "📊" }
                        span { "监控日志" }
                    }
                    Link {
                        to: CoreRoute::ApiManagement {},
                        class: if matches!(route, CoreRoute::ApiManagement {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "🔗" }
                        span { "API管理" }
                    }
                    Link {
                        to: CoreRoute::ChannelPage {},
                        class: if matches!(route, CoreRoute::ChannelPage {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "📡" }
                        span { "渠道管理" }
                    }
                    Link {
                        to: CoreRoute::SystemSettings {},
                        class: if matches!(route, CoreRoute::SystemSettings {}) { "nav-item active" } else { "nav-item" },
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