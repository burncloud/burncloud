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
    rsx! {
        Router::<Route> {}
    }
}