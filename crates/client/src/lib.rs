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
                <style>{}</style>
                <style>{}</style>
                <style>
                    /* Custom JIT Shims for LiveView match Desktop Layout */
                    .text-xxs {{ font-size: 10px; }}
                    .text-xxxs {{ font-size: 8px; }}
                    .bg-macos-red {{ background-color: #FF5F56; }}
                    .bg-macos-yellow {{ background-color: #FFBD2E; }}
                    .bg-macos-green {{ background-color: #27C93F; }}
                    .shadow-glow-green {{ box-shadow: 0 0 8px rgba(34,197,94,0.6); }}

                    /* ========== HOMEPAGE ANIMATIONS ========== */
                    /* (Same animations as GuestLayout to ensure visual consistency in LiveView) */
                    /* ... (Truncated for brevity in replacement, assuming existing styles remain if I don't replace them? No, replace replaces everything) */
                    /* WAIT, I need to include ALL CSS or use the existing block. */
                    /* Since I am replacing the FUNCTION, I must provide the FULL FUNCTION content. */
                    
                    /* Aurora - Ethereal flowing background */
                    @keyframes aurora {{
                        0%, 100% {{ transform: translateX(0) translateY(0) rotate(0deg) scale(1); opacity: 0.6; }}
                        25% {{ transform: translateX(50px) translateY(-30px) rotate(5deg) scale(1.1); opacity: 0.8; }}
                        50% {{ transform: translateX(-30px) translateY(50px) rotate(-5deg) scale(1.05); opacity: 0.7; }}
                        75% {{ transform: translateX(-50px) translateY(-20px) rotate(3deg) scale(0.95); opacity: 0.9; }}
                    }}
                    .animate-aurora {{
                        animation: aurora 20s ease-in-out infinite;
                    }}

                    /* Float - Gentle levitation */
                    @keyframes float {{
                        0%, 100% {{ transform: translateY(0px); }}
                        50% {{ transform: translateY(-12px); }}
                    }}
                    .animate-float {{
                        animation: float 6s ease-in-out infinite;
                    }}

                    /* Glow Pulse - Pulsating glow effect */
                    @keyframes glow-pulse {{
                        0%, 100% {{ box-shadow: 0 0 20px rgba(0, 113, 227, 0.3), 0 0 40px rgba(0, 113, 227, 0.1); }}
                        50% {{ box-shadow: 0 0 30px rgba(0, 113, 227, 0.5), 0 0 60px rgba(0, 113, 227, 0.2); }}
                    }}
                    .animate-glow-pulse {{
                        animation: glow-pulse 3s ease-in-out infinite;
                    }}

                    /* Shimmer - Light sweep effect */
                    @keyframes shimmer {{
                        0% {{ background-position: -200% 0; }}
                        100% {{ background-position: 200% 0; }}
                    }}
                    .animate-shimmer {{
                        background: linear-gradient(90deg, transparent, rgba(255,255,255,0.4), transparent);
                        background-size: 200% 100%;
                        animation: shimmer 2s linear infinite;
                    }}

                    /* Gradient Flow - Moving gradient background */
                    @keyframes gradient-flow {{
                        0% {{ background-position: 0% 50%; }}
                        50% {{ background-position: 100% 50%; }}
                        100% {{ background-position: 0% 50%; }}
                    }}
                    .animate-gradient-flow {{
                        background-size: 200% 200%;
                        animation: gradient-flow 8s ease infinite;
                    }}

                    /* Scale In - Entrance animation */
                    @keyframes scale-in {{
                        0% {{ transform: scale(0.9); opacity: 0; }}
                        100% {{ transform: scale(1); opacity: 1; }}
                    }}
                    .animate-scale-in {{
                        animation: scale-in 0.6s cubic-bezier(0.16, 1, 0.3, 1) forwards;
                    }}

                    /* Slide Up Fade - Staggered entrance */
                    @keyframes slide-up-fade {{
                        0% {{ transform: translateY(30px); opacity: 0; }}
                        100% {{ transform: translateY(0); opacity: 1; }}
                    }}
                    .animate-slide-up {{
                        animation: slide-up-fade 0.8s cubic-bezier(0.16, 1, 0.3, 1) forwards;
                    }}
                    .animate-delay-100 {{ animation-delay: 0.1s; opacity: 0; }}
                    .animate-delay-200 {{ animation-delay: 0.2s; opacity: 0; }}
                    .animate-delay-300 {{ animation-delay: 0.3s; opacity: 0; }}
                    .animate-delay-400 {{ animation-delay: 0.4s; opacity: 0; }}
                    .animate-delay-500 {{ animation-delay: 0.5s; opacity: 0; }}

                    /* Typing Cursor */
                    @keyframes blink {{
                        0%, 50% {{ opacity: 1; }}
                        51%, 100% {{ opacity: 0; }}
                    }}
                    .animate-blink {{
                        animation: blink 1s step-end infinite;
                    }}

                    /* Orbit - Rotating particles */
                    @keyframes orbit {{
                        0% {{ transform: rotate(0deg) translateX(100px) rotate(0deg); }}
                        100% {{ transform: rotate(360deg) translateX(100px) rotate(-360deg); }}
                    }}
                    .animate-orbit {{
                        animation: orbit 20s linear infinite;
                    }}

                    /* Morph - Shape morphing blob */
                    @keyframes morph {{
                        0%, 100% {{ border-radius: 60% 40% 30% 70% / 60% 30% 70% 40%; }}
                        25% {{ border-radius: 30% 60% 70% 40% / 50% 60% 30% 60%; }}
                        50% {{ border-radius: 50% 60% 30% 60% / 30% 60% 70% 40%; }}
                        75% {{ border-radius: 60% 40% 60% 30% / 70% 30% 50% 60%; }}
                    }}
                    .animate-morph {{
                        animation: morph 8s ease-in-out infinite;
                    }}

                    /* Counter animation */
                    @keyframes count-up {{
                        from {{ opacity: 0; transform: translateY(10px); }}
                        to {{ opacity: 1; transform: translateY(0); }}
                    }}
                    .animate-count {{
                        animation: count-up 0.5s ease-out forwards;
                    }}

                    /* Ripple effect */
                    @keyframes ripple {{
                        0% {{ transform: scale(0.8); opacity: 1; }}
                        100% {{ transform: scale(2.5); opacity: 0; }}
                    }}
                    .animate-ripple {{
                        animation: ripple 2s ease-out infinite;
                    }}

                    /* Magnetic hover effect helper */
                    .magnetic-hover {{
                        transition: transform 0.3s cubic-bezier(0.16, 1, 0.3, 1);
                    }}
                    .magnetic-hover:hover {{
                        transform: scale(1.02) translateY(-2px);
                    }}

                    /* Glass morphism */
                    .glass {{
                        background: rgba(255, 255, 255, 0.7);
                        backdrop-filter: blur(20px);
                        -webkit-backdrop-filter: blur(20px);
                        border: 1px solid rgba(255, 255, 255, 0.3);
                    }}
                </style>
            </head>
            <body>
                <div id="main"></div>
                {}
            </body>
            </html>
            "#,
            include_str!("./assets/tailwind.css"),
            include_str!("./assets/daisyui.css"),
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
        .route("/users", html_handler.clone())
        .route("/settings", html_handler.clone())
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
