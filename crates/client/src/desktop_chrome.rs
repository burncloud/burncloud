use dioxus::prelude::*;

#[cfg(feature = "desktop")]
#[component]
pub fn DesktopTitleBar() -> Element {
    let window = dioxus::desktop::use_window();
    let mut is_maximized = use_signal(|| window.is_maximized());

    let min_window = window.clone();
    let max_window = window.clone();
    let close_window = window.clone();

    rsx! {
        div { class: "desktop-titlebar app-drag-region",
            div { class: "desktop-titlebar-spacer" }
            div { class: "desktop-window-controls app-no-drag",
                button {
                    class: "desktop-window-control",
                    title: "Minimize",
                    aria_label: "Minimize window",
                    onclick: move |_| min_window.set_minimized(true),
                    span { class: "desktop-win-icon", "\u{E921}" }
                }
                button {
                    class: "desktop-window-control",
                    title: if is_maximized() { "Restore" } else { "Maximize" },
                    aria_label: if is_maximized() { "Restore window" } else { "Maximize window" },
                    onclick: move |_| {
                        let next = !is_maximized();
                        max_window.set_maximized(next);
                        is_maximized.set(next);
                    },
                    span {
                        class: "desktop-win-icon",
                        if is_maximized() { "\u{E923}" } else { "\u{E922}" }
                    }
                }
                button {
                    class: "desktop-window-control danger",
                    title: "Close to tray",
                    aria_label: "Hide window to system tray",
                    onclick: move |_| close_window.set_visible(false),
                    span { class: "desktop-win-icon", "\u{E8BB}" }
                }
            }
        }
    }
}

#[cfg(all(feature = "desktop", target_os = "windows"))]
mod windows_tray {
    use std::fmt;
    use std::process;
    use std::sync::atomic::{AtomicBool, Ordering};
    use systray::Application;

    static ICON_DATA: &[u8] = include_bytes!("../assets/favicon.ico");
    static SHOULD_SHOW: AtomicBool = AtomicBool::new(false);

    #[derive(Debug)]
    struct TrayError(String);

    impl fmt::Display for TrayError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TrayError {}

    pub fn should_show_window() -> bool {
        SHOULD_SHOW.swap(false, Ordering::Relaxed)
    }

    pub fn start_tray() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = Application::new()?;

        let icon_path = std::env::temp_dir().join("burncloud_tray.ico");
        if std::fs::write(&icon_path, ICON_DATA).is_ok() {
            let _ = app.set_icon_from_file(&icon_path.to_string_lossy());
            let _ = std::fs::remove_file(&icon_path);
        }

        app.add_menu_item("显示界面", |_| -> Result<(), TrayError> {
            SHOULD_SHOW.store(true, Ordering::Relaxed);
            Ok(())
        })?;
        app.add_menu_separator()?;
        app.add_menu_item("退出程序", |_| -> Result<(), TrayError> {
            process::exit(0);
        })?;
        app.wait_for_message()?;
        Ok(())
    }
}

#[cfg(all(feature = "desktop", target_os = "windows"))]
pub use windows_tray::{should_show_window, start_tray};
