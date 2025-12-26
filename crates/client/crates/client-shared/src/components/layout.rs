use dioxus::prelude::*;
use dioxus_router::components::Outlet;
use dioxus_router::Routable;

#[allow(unused_imports)]
use super::placeholders::{
    ApiManagement, BillingPage, ChannelPage, ConnectPage, Dashboard, DeployConfig, LogPage,
    NotFoundPage, PlaygroundPage, ServiceMonitor, SystemSettings, UserPage,
};
use super::sidebar::Sidebar;
use super::title_bar::TitleBar;
use crate::styles::FLUENT_CSS;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum CoreRoute {
    #[layout(Layout)]
    #[route("/console/dashboard")]
    Dashboard {},
    #[route("/console/deploy")]
    DeployConfig {},
    #[route("/console/monitor")]
    ServiceMonitor {},
    #[route("/console/access")]
    ApiManagement {},
    #[route("/console/models")]
    ChannelPage {},
    #[route("/console/users")]
    UserPage {},
    #[route("/console/settings")]
    SystemSettings {},
    #[route("/console/finance")]
    BillingPage {},
    #[route("/console/logs")]
    LogPage {},
    #[route("/console/connect")]
    ConnectPage {},
    #[route("/console/playground")]
    PlaygroundPage {},
    #[route("/console/:..segments")]
    NotFoundPage { segments: Vec<String> },
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
