use dioxus::prelude::*;

use crate::components::layout::Layout;
use crate::pages::{
    dashboard::Dashboard,
    models::ModelManagement,
    deploy::DeployConfig,
    monitor::ServiceMonitor,
    api::ApiManagement,
    settings::SystemSettings,
};
pub use burncloud_client_tray::{start_tray, should_show_window};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(Layout)]
    #[route("/")]
    Dashboard {},
    #[route("/models")]
    ModelManagement {},
    #[route("/deploy")]
    DeployConfig {},
    #[route("/monitor")]
    ServiceMonitor {},
    #[route("/api")]
    ApiManagement {},
    #[route("/settings")]
    SystemSettings {},
}

#[component]
pub fn App() -> Element {
    // Inject Global Styles (BCDS)
    let styles = include_str!("../crates/client-api/assets/styles.css");

    rsx! {
        style { "{styles}" }
        Router::<Route> {}
    }
}

#[cfg(feature = "desktop")]
pub fn launch_gui() {
    launch_gui_with_tray();
}

#[cfg(feature = "desktop")]
pub fn launch_gui_with_tray() {
    use dioxus::desktop::{Config, WindowBuilder};

    let window = WindowBuilder::new()
        .with_title("BurnCloud - 大模型本地部署平台")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
        .with_resizable(true)
        .with_decorations(false);

    let config = Config::new().with_window(window);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(AppWithTray);
}

#[cfg(feature = "desktop")]
#[component]
fn AppWithTray() -> Element {
    let window = dioxus::desktop::use_window();

    let window_setup = window.clone();
    use_effect(move || {
        window_setup.set_maximized(true);

        // 启动托盘应用在后台线程
        std::thread::spawn(move || {
            if let Err(e) = start_tray() {
                eprintln!("Failed to start tray: {}", e);
            }
        });
    });

    // 轮询检查托盘操作
    use_effect(move || {
        let window_clone = window.clone();
        spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                if should_show_window() {
                    // 强制显示窗口
                    let _ = window_clone.set_visible(false);
                    let _ = window_clone.set_visible(true);
                    let _ = window_clone.set_focus();
                }
            }
        });
    });

    rsx! { App {} }
}
