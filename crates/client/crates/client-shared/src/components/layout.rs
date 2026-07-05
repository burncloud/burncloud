use dioxus::prelude::*;
use dioxus_router::components::Outlet;
use dioxus_router::Routable;

#[allow(unused_imports)]
use super::placeholders::{
    ApiManagement, ChannelPage, ConnectPage, Dashboard, DeployConfig, FinancePage, LogPage,
    NotFoundPage, PlaygroundPage, ServiceMonitor, SystemSettings, UsersPage,
};
use super::sidebar::Sidebar;
use super::title_bar::TitleBar;
use crate::app_styles::AppStyles;

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
    UsersPage {},
    #[route("/console/settings")]
    SystemSettings {},
    #[route("/console/finance")]
    FinancePage {},
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
            AppStyles {}
        }

        div { class: "relative flex flex-col h-screen w-screen bg-bc-canvas text-bc-text overflow-hidden select-none", "data-theme": "light",

            div { class: "absolute top-0 left-0 w-full z-50 pointer-events-none",
                div { class: "pointer-events-auto",
                    TitleBar {}
                }
            }

            div { class: "flex flex-1 min-h-0 overflow-hidden w-full pt-8",

                div { class: "w-64 shrink-0 flex flex-col border-r border-bc-border/50 bg-bc-canvas/80 backdrop-blur-xl",
                    div { class: "flex-1 overflow-y-auto px-2 py-4",
                        Sidebar {}
                    }
                }

                div { class: "flex-1 flex flex-col bg-bc-canvas relative min-w-0",
                    main { class: "flex-1 overflow-y-auto overflow-x-hidden flex flex-col min-h-0",
                        Outlet::<CoreRoute> {}
                    }
                }
            }
        }
    }
}
