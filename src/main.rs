use dioxus::prelude::*;
use dioxus_router::prelude::*;
use dioxus::desktop;

mod components;
mod pages;
mod types;

#[derive(Clone, Routable, Debug, PartialEq)]
enum Route {
    #[route("/")]
    Dashboard {},
    #[route("/models")]
    Models {},
    #[route("/deploy")]
    Deploy {},
    #[route("/monitor")]
    Monitor {},
    #[route("/settings")]
    Settings {},
}

fn main() {
    LaunchBuilder::desktop()
        .with_cfg(desktop::Config::new()
            .with_menu(None)  // 移除菜单栏
            .with_window(desktop::WindowBuilder::new()
                .with_title("BurnCloud - 大模型本地部署平台")
                .with_inner_size(desktop::LogicalSize::new(1200.0, 800.0))
                .with_min_inner_size(desktop::LogicalSize::new(900.0, 600.0))
                .with_resizable(true)
                .with_decorations(true)
                .with_always_on_top(false)  // 确保窗口不总在最前面
                .with_focused(true)  // 启动时获得焦点
                .with_visible(true)  // 确保窗口可见
            )
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        style { {include_str!("../assets/styles.css")} }
        Router::<Route> {}
    }
}

#[component]
fn Dashboard() -> Element {
    rsx! {
        pages::dashboard::Dashboard {}
    }
}

#[component] 
fn Models() -> Element {
    rsx! {
        pages::models::Models {}
    }
}

#[component]
fn Deploy() -> Element {
    rsx! {
        pages::deploy::Deploy {}
    }
}

#[component]
fn Monitor() -> Element {
    rsx! {
        pages::monitor::Monitor {}
    }
}

#[component]
fn Settings() -> Element {
    rsx! {
        pages::settings::Settings {}
    }
}