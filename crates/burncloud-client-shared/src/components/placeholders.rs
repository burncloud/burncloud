use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        div { class: "page-container",
            h1 { "仪表盘" }
            p { "这是仪表盘页面的占位符内容" }
        }
    }
}

#[component]
pub fn ModelManagement() -> Element {
    rsx! {
        div { class: "page-container",
            h1 { "模型管理" }
            p { "这是模型管理页面的占位符内容" }
        }
    }
}

#[component]
pub fn DeployConfig() -> Element {
    rsx! {
        div { class: "page-container",
            h1 { "部署配置" }
            p { "这是部署配置页面的占位符内容" }
        }
    }
}

#[component]
pub fn ServiceMonitor() -> Element {
    rsx! {
        div { class: "page-container",
            h1 { "服务监控" }
            p { "这是服务监控页面的占位符内容" }
        }
    }
}

#[component]
pub fn ApiManagement() -> Element {
    rsx! {
        div { class: "page-container",
            h1 { "API管理" }
            p { "这是API管理页面的占位符内容" }
        }
    }
}

#[component]
pub fn SystemSettings() -> Element {
    rsx! {
        div { class: "page-container",
            h1 { "系统设置" }
            p { "这是系统设置页面的占位符内容" }
        }
    }
}