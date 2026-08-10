use dioxus::prelude::*;

#[cfg(feature = "desktop")]
#[path = "desktop_chrome.rs"]
mod desktop_chrome;

use crate::{
    auth_gate::AuthGate,
    critical_pages::{Customers, Login, Logs, Overview, Register},
    functional_pages::{
        APIKeys, Billing, Evaluation, Guardrails, Models, Playground, Providers, Routes, Settings,
        Team,
    },
    pages::{Home, Landing},
    route_aliases::{Dashboard, Users},
};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(AuthGate)]
    #[route("/")]
    Overview {},
    #[route("/dashboard")]
    Dashboard {},
    #[route("/playground")]
    Playground {},
    #[route("/routes")]
    Routes {},
    #[route("/models")]
    Models {},
    #[route("/providers")]
    Providers {},
    #[route("/keys")]
    APIKeys {},
    #[route("/customers")]
    Customers {},
    #[route("/users")]
    Users {},
    #[route("/guardrails")]
    Guardrails {},
    #[route("/logs")]
    Logs {},
    #[route("/evaluation")]
    Evaluation {},
    #[route("/billing")]
    Billing {},
    #[route("/team")]
    Team {},
    #[route("/settings")]
    Settings {},
    #[end_layout]
    #[route("/home")]
    Home {},
    #[route("/landing")]
    Landing {},
    #[route("/login")]
    Login {},
    #[route("/register")]
    Register {},
}

#[component]
fn DesktopChrome() -> Element {
    #[cfg(feature = "desktop")]
    {
        return rsx! { desktop_chrome::DesktopTitleBar {} };
    }

    #[cfg(not(feature = "desktop"))]
    {
        rsx! {}
    }
}

#[component]
pub fn App() -> Element {
    let _auth = crate::backend::use_init_auth();

    rsx! {
        head {
            meta { charset:"utf-8" }
            meta { name:"viewport", content:"width=device-width, initial-scale=1" }
            title { "BurnCloud" }
            style { dangerous_inner_html: include_str!("styles.css") }
            style { dangerous_inner_html: include_str!("critical_pages.css") }
            style { dangerous_inner_html: include_str!("desktop_chrome.css") }
        }
        DesktopChrome {}
        Router::<Route> {}
    }
}

#[cfg(feature = "desktop")]
pub fn launch_gui_with_tray() {
    use dioxus::desktop::{Config, WindowBuilder};

    let window = WindowBuilder::new()
        .with_title("BurnCloud - AI Local Deployment Platform")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1440.0, 900.0))
        .with_resizable(true)
        .with_decorations(false);

    #[cfg(target_os = "windows")]
    let window = {
        use dioxus::desktop::tao::platform::windows::IconExtWindows;
        match dioxus::desktop::tao::window::Icon::from_resource(1, None) {
            Ok(icon) => window.with_window_icon(Some(icon)),
            Err(_) => window,
        }
    };

    let data_dir = std::env::temp_dir().join("burncloud_dioxus_ui");
    let config = Config::new().with_window(window).with_data_directory(data_dir);
    dioxus::LaunchBuilder::desktop().with_cfg(config).launch(AppWithDesktop);
}

#[cfg(feature = "desktop")]
#[component]
fn AppWithDesktop() -> Element {
    let window = dioxus::desktop::use_window();

    let maximize_window = window.clone();
    use_effect(move || maximize_window.set_maximized(true));

    #[cfg(target_os = "windows")]
    {
        use_effect(move || {
            std::thread::spawn(move || {
                if let Err(error) = desktop_chrome::start_tray() {
                    eprintln!("Failed to start BurnCloud system tray: {error}");
                }
            });
        });

        let tray_window = window.clone();
        use_effect(move || {
            let poll_window = tray_window.clone();
            spawn(async move {
                loop {
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                    if desktop_chrome::should_show_window() {
                        poll_window.set_visible(false);
                        poll_window.set_visible(true);
                        poll_window.set_focus();
                    }
                }
            });
        });
    }

    rsx! { App {} }
}

#[cfg(feature = "web")]
pub fn launch_web() {
    dioxus::launch(App);
}
