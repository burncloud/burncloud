pub mod app;
pub mod components;
pub mod pages;

mod tests;

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
use dioxus::prelude::{use_context_provider, Element};
#[cfg(feature = "liveview")]
use std::str::FromStr;
#[cfg(feature = "liveview")]
use std::sync::Arc;

#[cfg(feature = "liveview")]
use burncloud_common::constants::WS_PATH;

#[cfg(feature = "liveview")]
#[dioxus::prelude::component]
fn LiveViewApp() -> Element {
    use_context_provider(|| app::ExternalStylesProvided);
    app::App()
}

#[cfg(feature = "liveview")]
pub fn liveview_router(_db: Arc<Database>) -> Router {
    use burncloud_client_shared::liveview_style_tags;

    let view = LiveViewPool::new();

    let html_handler = axum::routing::get(move |headers: axum::http::HeaderMap| async move {
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
            glue = dioxus_liveview::interpreter_glue(&websocket_url(&headers))
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
        .route("/console/{*path}", html_handler.clone());

    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    let app = app
        .route("/preview/home", html_handler.clone())
        .route("/preview/login", html_handler.clone())
        .route("/preview/console", html_handler.clone())
        .route("/preview/console/", html_handler.clone())
        .route("/preview/console/{*path}", html_handler.clone());

    let app = app
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
                            LiveViewApp as fn() -> dioxus::prelude::Element,
                        )
                        .await;
                })
            }),
        );

    app
}

#[cfg(feature = "liveview")]
fn websocket_url(headers: &axum::http::HeaderMap) -> String {
    let host = headers
        .get(axum::http::header::HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| axum::http::uri::Authority::from_str(value).ok())
        .map(|authority| authority.to_string())
        .unwrap_or_else(|| "localhost:3000".to_string());
    let is_https = headers
        .get("x-forwarded-proto")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("https"));
    let scheme = if is_https { "wss" } else { "ws" };

    format!("{scheme}://{host}{WS_PATH}")
}

#[cfg(all(test, feature = "liveview"))]
mod liveview_tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::websocket_url;

    #[test]
    fn websocket_url_uses_secure_scheme_behind_https_proxy() {
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("console.example.test"));
        headers.insert("x-forwarded-proto", HeaderValue::from_static("https"));

        assert_eq!(websocket_url(&headers), "wss://console.example.test/ws");
    }

    #[test]
    fn websocket_url_defaults_to_plain_websocket() {
        let mut headers = HeaderMap::new();
        headers.insert("host", HeaderValue::from_static("localhost:8080"));

        assert_eq!(websocket_url(&headers), "ws://localhost:8080/ws");
    }
}
