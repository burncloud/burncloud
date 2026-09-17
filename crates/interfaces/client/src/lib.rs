//! BurnCloud's single-crate Dioxus client.
//!
//! The module layout mirrors the directory contract in the UI architecture
//! documentation.  Feature-specific adapters stay at this boundary while
//! product domains remain grouped below `domains/`.

pub mod api;
pub mod app;
pub mod auth;
pub mod design;
pub mod domains;
pub mod i18n;
pub mod platform;
pub mod shared;

#[cfg(feature = "desktop")]
#[path = "desktop_chrome.rs"]
mod desktop_chrome;

#[cfg(feature = "desktop")]
use dioxus::prelude::*;

#[cfg(feature = "liveview")]
use std::sync::Arc;

#[cfg(feature = "liveview")]
use axum::Router;
#[cfg(feature = "liveview")]
use axum::{extract::ws::WebSocketUpgrade, response::Html, routing::get};
#[cfg(feature = "liveview")]
use burncloud_database::Database;
#[cfg(feature = "liveview")]
use dioxus::prelude::VirtualDom;
#[cfg(feature = "liveview")]
use dioxus_liveview::{axum_socket, interpreter_glue, LiveViewPool};

/// Build the server-side LiveView routes for the console.
#[cfg(feature = "liveview")]
pub fn liveview_router(_db: Arc<Database>) -> Router {
    // Register the paths explicitly so the websocket endpoint is absolute on
    // Axum 0.8 (the upstream helper emits a relative websocket path).
    let pool = std::sync::Arc::new(LiveViewPool::new());
    let app = std::sync::Arc::new(app::App as fn() -> dioxus::prelude::Element);
    let ws_pool = pool.clone();
    let ws_app = app.clone();
    let html = interpreter_glue("/console/ws");
    let index = move || {
        let html = html.clone();
        async move {
            Html(format!("<!doctype html><html><head><title>BurnCloud</title></head><body><div id=\"main\"></div>{html}</body></html>"))
        }
    };
    Router::new()
        .route(
            "/console/ws",
            get(move |ws: WebSocketUpgrade| {
                let pool = ws_pool.clone();
                let app = ws_app.clone();
                async move {
                    ws.on_upgrade(move |socket| async move {
                        let app = app.clone();
                        let _ = pool
                            .launch_virtualdom(axum_socket(socket), move || VirtualDom::new(*app))
                            .await;
                    })
                }
            }),
        )
        .route("/", get(index.clone()))
        .route("/home", get(index.clone()))
        .route("/landing", get(index.clone()))
        .route("/login", get(index.clone()))
        .route("/register", get(index.clone()))
        .route("/playground", get(index.clone()))
        .route("/marketplace", get(index.clone()))
        .route("/models", get(index.clone()))
        .route("/keys", get(index.clone()))
        .route("/usage", get(index.clone()))
        .route("/billing", get(index.clone()))
        .route("/logs", get(index.clone()))
        .route("/buyer", get(index.clone()))
        .route("/buyer/{*route}", get(index.clone()))
        .route("/supplier", get(index.clone()))
        .route("/supplier/{*route}", get(index.clone()))
        .route("/admin", get(index.clone()))
        .route("/admin/{*route}", get(index.clone()))
        .route("/console", get(index.clone()))
        .route("/console/{*route}", get(index))
}

/// Start the native desktop client.
#[cfg(feature = "desktop")]
pub fn launch_gui_with_tray() {
    use dioxus::desktop::{Config, LogicalPosition, LogicalSize, WindowBuilder};

    let icon = dioxus::desktop::icon_from_memory::<dioxus::desktop::tao::window::Icon>(
        include_bytes!("../assets/favicon.ico"),
    )
    .expect("failed to decode the embedded BurnCloud window icon");
    let window = WindowBuilder::new()
        .with_title("BurnCloud")
        .with_window_icon(Some(icon.clone()))
        .with_inner_size(LogicalSize::new(1440.0, 900.0))
        .with_position(LogicalPosition::new(100.0, 80.0))
        .with_visible(true)
        .with_maximized(true)
        .with_resizable(true)
        .with_decorations(false);
    #[cfg(target_os = "windows")]
    let window = {
        use dioxus::desktop::tao::platform::windows::WindowBuilderExtWindows;

        window.with_taskbar_icon(Some(icon))
    };
    let config = Config::new()
        .with_window(window)
        .with_on_window(|window, _| window.set_decorations(false));
    #[cfg(target_os = "windows")]
    let config = config.with_tray_icon_show_window_on_click(false);
    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(AppWithDesktop);
}

#[cfg(feature = "desktop")]
#[component]
fn AppWithDesktop() -> Element {
    #[cfg(target_os = "windows")]
    desktop_chrome::use_windows_tray(dioxus::desktop::use_window());

    rsx! { app::App {} }
}
