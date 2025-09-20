use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::components::layout::Layout;
use crate::pages::{
    dashboard::Dashboard,
    models::ModelManagement,
    deploy::DeployConfig,
    monitor::ServiceMonitor,
    api::ApiManagement,
    settings::SystemSettings,
};

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
    let window = dioxus::desktop::use_window();

    use_effect(move || {
        window.set_maximized(true);
    });

    rsx! {
        Router::<Route> {}
    }
}

pub fn launch_gui() {
    launch_gui_impl();
}

fn launch_gui_impl() {
    use dioxus::desktop::{Config, WindowBuilder};

    let window = WindowBuilder::new()
        .with_title("BurnCloud - 大模型本地部署平台")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
        .with_resizable(true)
        .with_decorations(false);

    let config = Config::new().with_window(window);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(App);
}