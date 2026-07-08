//! E2E preview routes (`/preview/*`) — mock data, no login flow.
//! Compiled only in debug or with `e2e-preview` feature. Production default routes unchanged.

use crate::components::app_styles::AppStyles;
use crate::components::sidebar::Sidebar;
use crate::pages::{
    api::AccessPage, billing::FinancePage, dashboard::Dashboard, home::HomePage, login::LoginPage,
    models::ChannelPage, monitor::ServiceMonitor, playground::PlaygroundPage,
    settings::SystemSettings,
};
use burncloud_client_shared::components::TitleBar;
use burncloud_client_shared::e2e_mock::{E2eMockPage, E2eMockPageShell};
use burncloud_client_shared::use_theme;
use burncloud_client_shared::DesktopMode;
use dioxus::prelude::*;

/// Console chrome without router outlet — preview pages render page content as children.
#[component]
fn PreviewConsoleFrame(children: Element) -> Element {
    let is_desktop = try_use_context::<DesktopMode>().is_some();
    let theme_ctx = use_theme();
    let theme_str = theme_ctx.data_theme_attr();

    rsx! {
        head { AppStyles {} }
        div { class: "relative flex flex-col h-screen w-screen bg-bc-canvas text-bc-text overflow-hidden select-none", "data-theme": "{theme_str}",
            if is_desktop {
                div { class: "absolute top-0 left-0 w-full z-50 pointer-events-none",
                    div { class: "pointer-events-auto", TitleBar {} }
                }
            }
            div { class: "flex flex-1 min-h-0 overflow-hidden w-full",
                div { class: "w-64 shrink-0 flex flex-col border-r border-bc-border/50 bg-bc-canvas/80 backdrop-blur-xl",
                    div { class: "flex-1 overflow-y-auto px-bc-2 py-bc-4", Sidebar {} }
                }
                div { class: "flex-1 flex flex-col bg-bc-canvas relative min-w-0",
                    main { class: "flex-1 overflow-y-auto overflow-x-hidden flex flex-col min-h-0",
                        {children}
                    }
                }
            }
        }
    }
}

#[component]
fn PreviewGuestFrame(children: Element) -> Element {
    rsx! {
        head {
            link { rel: "icon", href: "favicon.ico" }
            AppStyles {}
        }
        div { class: "h-screen w-screen overflow-hidden flex flex-col bg-bc-canvas text-bc-text relative", "data-theme": "light",
            div { class: "absolute top-0 left-0 w-full z-50 pointer-events-none",
                div { class: "pointer-events-auto", TitleBar {} }
            }
            main { class: "flex-1 flex flex-col relative z-0 overflow-y-auto min-h-0",
                {children}
            }
        }
    }
}

#[component]
pub fn PreviewHomePage() -> Element {
    rsx! {
        PreviewGuestFrame { HomePage {} }
    }
}

#[component]
pub fn PreviewLoginPage() -> Element {
    rsx! {
        PreviewGuestFrame { LoginPage {} }
    }
}

#[component]
pub fn PreviewDashboardPage() -> Element {
    rsx! {
        E2eMockPageShell { page: E2eMockPage::Dashboard,
            PreviewConsoleFrame { Dashboard {} }
        }
    }
}

#[component]
pub fn PreviewModelsPage() -> Element {
    rsx! {
        E2eMockPageShell { page: E2eMockPage::Models,
            PreviewConsoleFrame { ChannelPage {} }
        }
    }
}

#[component]
pub fn PreviewAccessPage() -> Element {
    rsx! {
        E2eMockPageShell { page: E2eMockPage::Access,
            PreviewConsoleFrame { AccessPage {} }
        }
    }
}

#[component]
pub fn PreviewSettingsPage() -> Element {
    rsx! {
        E2eMockPageShell { page: E2eMockPage::Settings,
            PreviewConsoleFrame { SystemSettings {} }
        }
    }
}

#[component]
pub fn PreviewFinancePage() -> Element {
    rsx! {
        E2eMockPageShell { page: E2eMockPage::Finance,
            PreviewConsoleFrame { FinancePage {} }
        }
    }
}

#[component]
pub fn PreviewMonitorPage() -> Element {
    rsx! {
        E2eMockPageShell { page: E2eMockPage::Monitor,
            PreviewConsoleFrame { ServiceMonitor {} }
        }
    }
}

#[component]
pub fn PreviewPlaygroundPage() -> Element {
    rsx! {
        E2eMockPageShell { page: E2eMockPage::Playground,
            PreviewConsoleFrame { PlaygroundPage {} }
        }
    }
}
