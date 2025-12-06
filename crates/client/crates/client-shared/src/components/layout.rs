use dioxus::prelude::*;

use crate::styles::FLUENT_CSS;
use super::sidebar::Sidebar;
use super::title_bar::TitleBar;
use super::placeholders::{Dashboard, ModelManagement, DeployConfig, ServiceMonitor, ApiManagement, SystemSettings};

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
    #[route("/settings")]
    SystemSettings {},
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