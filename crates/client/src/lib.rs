pub mod app;
pub mod auth_gate;
pub mod backend;
pub mod components;
pub mod critical_pages;
pub mod customer_layout;
pub mod data;
pub mod functional_api;
pub mod functional_layout;
pub mod functional_pages;
pub mod observability;
pub mod public_pages;
pub mod role_access;
pub mod route_aliases;

pub use app::App;

#[cfg(feature = "desktop")]
pub use app::launch_gui_with_tray;
#[cfg(feature = "web")]
pub use app::launch_web;

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
    let view = LiveViewPool::new();
    let html_handler = axum::routing::get(move |headers: axum::http::HeaderMap| async move {
        let host = headers.get("host").and_then(|h| h.to_str().ok()).unwrap_or("localhost:3000");
        axum::response::Html(format!(
            r#"<!doctype html>
<html>
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>BurnCloud</title>
  <link rel="icon" href="/favicon.ico">
</head>
<body>
  <div id="main"></div>
  {glue}
</body>
</html>"#,
            glue = dioxus_liveview::interpreter_glue(&format!("ws://{}{}", host, WS_PATH))
        ))
    });

    let mut app = Router::new().route("/", html_handler.clone());
    for path in [
        "/dashboard", "/home", "/landing", "/login", "/register", "/playground", "/routes",
        "/models", "/providers", "/keys", "/customers", "/users", "/guardrails", "/logs",
        "/evaluation", "/billing", "/team", "/settings",
    ] {
        app = app.route(path, html_handler.clone());
    }

    app.route(
        "/favicon.ico",
        axum::routing::get(|| async {
            ([(axum::http::header::CONTENT_TYPE, "image/x-icon")], include_bytes!("../assets/favicon.ico"))
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
    )
}
