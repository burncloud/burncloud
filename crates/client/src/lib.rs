pub mod app;
pub mod components;
pub mod pages;

mod tests;

pub use app::App;

#[cfg(feature = "desktop")]
pub use app::launch_gui_with_tray;

#[cfg(feature = "liveview")]
use axum::Router;
#[cfg(feature = "liveview")]
use dioxus_liveview::LiveViewPool;
#[cfg(feature = "liveview")]
use std::sync::Arc;
#[cfg(feature = "liveview")]
use burncloud_database::Database;

#[cfg(feature = "liveview")]
use burncloud_common::constants::WS_PATH;

#[cfg(feature = "liveview")]
pub fn liveview_router(_db: Arc<Database>) -> Router {
    let view = LiveViewPool::new();

    let html_handler = axum::routing::get(move |headers: axum::http::HeaderMap| async move {
        let host = headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or("localhost:3000");
        axum::response::Html(format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>BurnCloud</title>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                <style>{}</style>
            </head>
            <body>
                <div id="main"></div>
                {}
            </body>
            </html>
            "#,
            include_str!("../crates/client-api/assets/styles.css"),
            dioxus_liveview::interpreter_glue(&format!("ws://{}{}", host, WS_PATH))
        ))
    });

    let app = Router::new()
        .route("/", html_handler.clone())
        .route("/login", html_handler.clone())
        .route("/register", html_handler.clone())
        .route("/models", html_handler.clone())
        .route("/deploy", html_handler.clone())
        .route("/monitor", html_handler.clone())
        .route("/api", html_handler.clone())
        .route("/channels", html_handler.clone())
        .route("/settings", html_handler.clone())
        .route(WS_PATH, axum::routing::get(move |ws: axum::extract::WebSocketUpgrade| async move {
            ws.on_upgrade(move |socket| async move {
                _ = view.launch(dioxus_liveview::axum_socket(socket), app::App as fn() -> dioxus::prelude::Element).await;
            })
        }));

    app
}