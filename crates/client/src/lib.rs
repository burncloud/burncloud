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
    dioxus_desktop::launch::launch(app::App, Vec::new(), Vec::new());
}
