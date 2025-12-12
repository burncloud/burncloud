use dioxus::prelude::*;
use dioxus_router::components::Outlet;
use dioxus_router::Routable;

#[allow(unused_imports)]
use super::placeholders::{
    ApiManagement, BillingPage, ChannelPage, Dashboard, DeployConfig, LogPage, ModelManagement,
    ServiceMonitor, SystemSettings, UserPage,
};
use super::sidebar::Sidebar;
use super::title_bar::TitleBar;
use crate::styles::FLUENT_CSS;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum CoreRoute {
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
    #[route("/channels")]
    ChannelPage {},
    #[route("/users")]
    UserPage {},
    #[route("/settings")]
    SystemSettings {},
    #[route("/billing")]
    BillingPage {},
    #[route("/logs")]
    LogPage {},
}

#[component]
pub fn Layout() -> Element {
    rsx! {
        head {
            style { "{FLUENT_CSS}" }
        }
        div { class: "app-container",
            TitleBar {}
            div { class: "app-body",
                Sidebar {}
                main { class: "main-content",
                    Outlet::<CoreRoute> {}
                }
            }
        }
    }
}
