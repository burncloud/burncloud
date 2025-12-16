use dioxus::prelude::*;

use crate::components::{guest_layout::GuestLayout, layout::Layout};
use crate::pages::{
    api::ApiManagement, billing::BillingPage, burngrid::BurnGridPage, channels::ChannelPage,
    dashboard::Dashboard, deploy::DeployConfig, home::HomePage, login::LoginPage, logs::LogPage,
    models::ModelManagement, monitor::ServiceMonitor, playground::PlaygroundPage,
    register::RegisterPage, settings::SystemSettings, user::UserPage,
};
use burncloud_client_shared::DesktopMode;
pub use burncloud_client_tray::{should_show_window, start_tray};

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(GuestLayout)]
    #[route("/")]
    HomePage {},
    #[route("/login")]
    LoginPage {},
    #[route("/register")]
    RegisterPage {},
    #[end_layout]
    #[layout(Layout)]
    #[route("/console/dashboard")]
    Dashboard {},
    #[route("/console/models")]
    ModelManagement {},
    #[route("/console/deploy")]
    DeployConfig {},
    #[route("/console/monitor")]
    ServiceMonitor {},
    #[route("/console/access")]
    ApiManagement {},
    #[route("/console/channels")]
    ChannelPage {},
    #[route("/console/users")]
    UserPage {},
    #[route("/console/settings")]
    SystemSettings {},
    #[route("/console/finance")]
    BillingPage {},
    #[route("/console/logs")]
    LogPage {},
    #[route("/console/burngrid")]
    BurnGridPage {},
    #[route("/console/playground")]
    PlaygroundPage {},
}

#[component]
pub fn App() -> Element {
    // Initialize i18n context
    burncloud_client_shared::i18n::use_init_i18n();
    // Initialize Toast
    burncloud_client_shared::use_init_toast();
    // Initialize Auth Context
    burncloud_client_shared::use_init_auth();

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

    let config = Config::new().with_window(window);

    dioxus::LaunchBuilder::desktop()
        .with_cfg(config)
        .launch(AppWithTray);
}

#[cfg(feature = "desktop")]
#[component]
fn AppWithTray() -> Element {
    use_context_provider(|| DesktopMode);
    let window = dioxus::desktop::use_window();

    let window_setup = window.clone();
    use_effect(move || {
        window_setup.set_maximized(true);

        // 启动托盘应用在后台线程
        std::thread::spawn(move || {
            if let Err(e) = start_tray() {
                eprintln!("Failed to start tray: {}", e);
            }
        });
    });

    // 轮询检查托盘操作
    use_effect(move || {
        let window_clone = window.clone();
        spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                if should_show_window() {
                    // 强制显示窗口
                    window_clone.set_visible(false);
                    window_clone.set_visible(true);
                    window_clone.set_focus();
                }
            }
        });
    });

    rsx! { App {} }
}
