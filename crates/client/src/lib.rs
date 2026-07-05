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
use burncloud_database::Database;
#[cfg(feature = "liveview")]
use dioxus_liveview::LiveViewPool;
#[cfg(feature = "liveview")]
use std::sync::Arc;

#[cfg(feature = "liveview")]
use burncloud_common::constants::WS_PATH;

#[cfg(feature = "liveview")]
pub fn liveview_router(_db: Arc<Database>) -> Router {
    use burncloud_client_shared::liveview_style_tags;

    let view = LiveViewPool::new();

    let html_handler = axum::routing::get(move |headers: axum::http::HeaderMap| async move {
        let host = headers
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost:3000");
        axum::response::Html(format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>BurnCloud</title>
                <link rel="icon" href="/favicon.ico">
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1.0">
                {styles}
            </head>
            <body>
                <div id="main"></div>
                {glue}
            </body>
            </html>
            "#,
            styles = liveview_style_tags(),
            glue = dioxus_liveview::interpreter_glue(&format!("ws://{}{}", host, WS_PATH))
        ))
    });

    let app = Router::new()
        .route("/", html_handler.clone())
        .route("/home", html_handler.clone())
        .route("/login", html_handler.clone())
        .route("/register", html_handler.clone())
        // Auth recovery routes (forgot/reset password)
        .route("/forgot-password", html_handler.clone())
        .route("/reset-password", html_handler.clone())
        // Console Routes (SPA Mode)
        // Axum's catch-all {*path} requires at least one path segment.
        // We must explicitly handle /console and /console/ to avoid fallback to router_app.
        .route("/console", html_handler.clone())
        .route("/console/", html_handler.clone())
        // Use a single wildcard to handle all /console/* paths including undefined ones (404s)
        .route("/console/{*path}", html_handler.clone())
        .route(
            "/favicon.ico",
            axum::routing::get(|| async {
                (
                    [(axum::http::header::CONTENT_TYPE, "image/x-icon")],
                    include_bytes!("../assets/favicon.ico"),
                )
            }),
        )
        .route(
            WS_PATH,
            axum::routing::get(move |ws: axum::extract::WebSocketUpgrade| async move {
                ws.on_upgrade(move |socket| async move {
                    _ = view
                        .launch(
                            dioxus_liveview::axum_socket(socket),
                            app::App as fn() -> dioxus::prelude::Element,
                        )
                        .await;
                })
            }),
        );

    app
}
