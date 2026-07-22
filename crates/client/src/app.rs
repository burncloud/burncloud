use dioxus::prelude::*;

use crate::components::{guest_layout::GuestLayout, layout::Layout};
use crate::pages::{
    api::AccessPage,
    billing::FinancePage,
    connect::ConnectPage,
    dashboard::Dashboard,
    deploy::DeployConfig,
    forgot_password::ForgotPasswordPage,
    home::{HomePage, Root},
    login::LoginPage,
    logs::LogPage,
    models::ChannelPage,
    monitor::ServiceMonitor,
    not_found::NotFoundPage,
    playground::PlaygroundPage,
    reset_password::ResetPasswordPage,
    settings::SystemSettings,
    user::UsersPage,
};
#[cfg(any(debug_assertions, feature = "e2e-preview"))]
use crate::pages::e2e_preview::{
    PreviewAccessPage, PreviewDashboardPage, PreviewFinancePage, PreviewHomePage,
    PreviewLoginPage, PreviewModelsPage, PreviewMonitorPage, PreviewPlaygroundPage,
    PreviewSettingsPage,
};
use burncloud_client_register::RegisterPage;
#[cfg(feature = "desktop")]
use burncloud_client_shared::DesktopMode;
#[cfg(all(feature = "desktop", target_os = "windows"))]
pub use burncloud_client_tray::{should_show_window, start_tray};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Root {},
    #[layout(GuestLayout)]
    #[route("/home")]
    HomePage {},
    #[route("/login")]
    LoginPage {},
    #[route("/register")]
    RegisterPage {},
    #[route("/forgot-password")]
    ForgotPasswordPage {},
    #[route("/reset-password?:token")]
    ResetPasswordPage { token: Option<String> },
    #[end_layout]
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/home")]
    PreviewHomePage {},
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/login")]
    PreviewLoginPage {},
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/console/dashboard")]
    PreviewDashboardPage {},
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/console/models")]
    PreviewModelsPage {},
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/console/access")]
    PreviewAccessPage {},
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/console/settings")]
    PreviewSettingsPage {},
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/console/finance")]
    PreviewFinancePage {},
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/console/monitor")]
    PreviewMonitorPage {},
    #[cfg(any(debug_assertions, feature = "e2e-preview"))]
    #[route("/preview/console/playground")]
    PreviewPlaygroundPage {},
    #[layout(Layout)]
    #[route("/console/dashboard")]
    Dashboard {},
    #[route("/console/deploy")]
    DeployConfig {},
    #[route("/console/monitor")]
    ServiceMonitor {},
    #[route("/console/access")]
    AccessPage {},
    #[route("/console/models")]
    ChannelPage {},
    #[route("/console/users")]
    UsersPage {},
    #[route("/console/settings")]
    SystemSettings {},
    #[route("/console/finance")]
    FinancePage {},
    #[route("/console/logs")]
    LogPage {},
    #[route("/console/connect")]
    ConnectPage {},
    #[route("/console/playground")]
    PlaygroundPage {},
    #[route("/console/:..segments")]
    NotFoundPage { segments: Vec<String> },
}

#[component]
pub fn App() -> Element {
    // Initialize i18n context
    burncloud_client_shared::i18n::use_init_i18n();
    // Initialize Toast
    burncloud_client_shared::use_init_toast();
    // Initialize Auth Context
    burncloud_client_shared::use_init_auth();
    // Theme (console layout reads data-theme from this)
    burncloud_client_shared::use_init_theme();

    rsx! {
        burncloud_client_shared::ToastContainer {}
        Router::<Route> {}
    }
}

#[cfg(feature = "desktop")]
pub fn launch_gui() {
    launch_gui_with_tray();
}

#[cfg(feature = "desktop")]
pub fn launch_gui_with_tray() {
    use dioxus::desktop::{Config, WindowBuilder};

    let window = WindowBuilder::new()
        .with_title("BurnCloud - AI Local Deployment Platform") // Changed to English/Bilingual
        .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0))
        .with_resizable(true)
        .with_decorations(false);

    // Load window icon from embedded multi-resolution ICO resource (set via winres in build.rs)
    #[cfg(target_os = "windows")]
    let window = {
        use dioxus::desktop::tao::platform::windows::IconExtWindows;
        match dioxus::desktop::tao::window::Icon::from_resource(1, None) {
            Ok(icon) => window.with_window_icon(Some(icon)),
            Err(_) => window,
        }
    };

    // Use a specific data directory in temp to avoid permission issues or path conflicts
    let data_dir = std::env::temp_dir().join("burncloud_webview_data");
    let config = Config::new()
        .with_window(window)
        .with_data_directory(data_dir);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(AppWithTray);
}

#[cfg(feature = "web")]
pub fn launch_web() {
    dioxus::LaunchBuilder::web().launch(App);
}

#[cfg(all(feature = "desktop", target_os = "windows"))]
#[component]
fn AppWithTray() -> Element {
    use_context_provider(|| DesktopMode);
    let window = dioxus::desktop::use_window();

    let window_setup = window.clone();
    use_effect(move || {
        window_setup.set_maximized(true);

        // ???????????
        std::thread::spawn(move || {
            if let Err(e) = start_tray() {
                eprintln!("Failed to start tray: {}", e);
            }
        });
    });

    // ????????
    use_effect(move || {
        let window_clone = window.clone();
        spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                if should_show_window() {
                    // ??????
                    window_clone.set_visible(false);
                    window_clone.set_visible(true);
                    window_clone.set_focus();
                }
            }
        });
    });

    rsx! { App {} }
}

#[cfg(all(feature = "desktop", not(target_os = "windows")))]
#[component]
fn AppWithTray() -> Element {
    use_context_provider(|| DesktopMode);
    let window = dioxus::desktop::use_window();

    use_effect(move || {
        window.set_maximized(true);
    });

    rsx! { App {} }
}
