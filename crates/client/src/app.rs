use dioxus::prelude::*;

use crate::{
    components::ConsoleLayout,
    critical_pages::{Customers, Dashboard, Login, Logs, Overview, Register, Users},
    pages::{
        APIKeys, Billing, Evaluation, Guardrails, Home, Landing, Models, Playground, Providers,
        Routes, Settings, Team,
    },
};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(ConsoleLayout)]
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
pub fn App() -> Element {
    rsx! {
        head {
            meta { charset:"utf-8" }
            meta { name:"viewport", content:"width=device-width, initial-scale=1" }
            title { "BurnCloud" }
            style { dangerous_inner_html: include_str!("styles.css") }
            style { dangerous_inner_html: include_str!("critical_pages.css") }
        }
        Router::<Route> {}
    }
}

#[cfg(feature = "desktop")]
pub fn launch_gui_with_tray() {
    use dioxus::desktop::{Config, WindowBuilder};
    let window = WindowBuilder::new()
        .with_title("BurnCloud")
        .with_inner_size(dioxus::desktop::LogicalSize::new(1440.0, 900.0))
        .with_resizable(true);
    let data_dir = std::env::temp_dir().join("burncloud_dioxus_ui");
    let config = Config::new().with_window(window).with_data_directory(data_dir);
    dioxus::LaunchBuilder::desktop().with_cfg(config).launch(AppWithDesktop);
}

#[cfg(feature = "desktop")]
#[component]
fn AppWithDesktop() -> Element {
    let window = dioxus::desktop::use_window();
    use_effect(move || window.set_maximized(true));
    rsx! { App {} }
}

#[cfg(feature = "web")]
pub fn launch_web() {
    dioxus::launch(App);
}
