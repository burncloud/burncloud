use dioxus::prelude::*;

#[component]
pub fn DesktopTitleBar() -> Element {
    let window = dioxus::desktop::use_window();
    let mut is_maximized = use_signal(|| window.is_maximized());

    let min_window = window.clone();
    let toggle_window = window.clone();
    let close_window = window.clone();
    let resize_window = window.clone();

    dioxus::desktop::use_wry_event_handler(move |event, _| {
        if matches!(
            event,
            dioxus::desktop::tao::event::Event::WindowEvent {
                event: dioxus::desktop::WindowEvent::Resized(_),
                ..
            }
        ) {
            let maximized = resize_window.is_maximized();
            if is_maximized() != maximized {
                is_maximized.set(maximized);
            }
        }
    });

    rsx! {
        style { dangerous_inner_html: include_str!("desktop_chrome.css") }
        div { class: "desktop-titlebar",
            div { class: "desktop-window-controls",
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
                        let maximized = !toggle_window.is_maximized();
                        toggle_window.set_maximized(maximized);
                        is_maximized.set(maximized);
                    },
                    span { class: "desktop-win-icon", if is_maximized() { "\u{E923}" } else { "\u{E922}" } }
                }
                button {
                    class: "desktop-window-control danger",
                    title: "Close",
                    aria_label: "Close window",
                    onclick: move |_| close_window.close(),
                    span { class: "desktop-win-icon", "\u{E8BB}" }
                }
            }
        }
    }
}

#[cfg(all(feature = "desktop", target_os = "windows"))]
mod windows_tray {
    use dioxus::desktop::trayicon::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent,
    };
    use dioxus::desktop::{DesktopContext, WindowCloseBehaviour};
    use dioxus::prelude::*;

    const ICON_DATA: &[u8] = include_bytes!("../assets/favicon.ico");
    const SHOW_MENU_ID: &str = "burncloud-show-window";
    const QUIT_MENU_ID: &str = "burncloud-quit";

    fn create_tray() -> Result<TrayIcon, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        let show = MenuItem::with_id(SHOW_MENU_ID, "显示界面", true, None);
        let separator = PredefinedMenuItem::separator();
        let quit = MenuItem::with_id(QUIT_MENU_ID, "退出程序", true, None);
        menu.append_items(&[&show, &separator, &quit])?;

        let icon = dioxus::desktop::icon_from_memory::<Icon>(ICON_DATA)?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .with_tooltip("BurnCloud")
            .with_icon(icon)
            .build()?;

        Ok(tray)
    }

    fn show_window(window: &DesktopContext) {
        window.set_minimized(false);
        window.set_visible(true);
        window.set_focus();
    }

    fn handle_menu_action(menu_id: &str, window: &DesktopContext) {
        match menu_id {
            SHOW_MENU_ID => show_window(window),
            QUIT_MENU_ID => {
                window.set_close_behavior(WindowCloseBehaviour::WindowCloses);
                window.close();
            }
            _ => {}
        }
    }

    pub fn use_windows_tray(window: DesktopContext) {
        let setup_window = window.clone();
        let tray = use_hook(move || match create_tray() {
            Ok(tray) => {
                setup_window.set_close_behavior(WindowCloseBehaviour::WindowHides);
                Some(tray)
            }
            Err(error) => {
                eprintln!("Failed to start BurnCloud system tray: {error}");
                None
            }
        });
        let tray_is_active = tray.is_some();

        // Dioxus 0.7 routes tray menu items through the shared Muda menu channel.
        let muda_menu_window = window.clone();
        dioxus::desktop::use_muda_event_handler(move |event| {
            if tray_is_active {
                handle_menu_action(event.id().as_ref(), &muda_menu_window);
            }
        });

        let tray_menu_window = window.clone();
        dioxus::desktop::use_tray_menu_event_handler(move |event| {
            if tray_is_active {
                handle_menu_action(event.id().as_ref(), &tray_menu_window);
            }
        });

        let icon_window = window;
        dioxus::desktop::use_tray_icon_event_handler(move |event| {
            if tray_is_active
                && matches!(
                    event,
                    TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    }
                )
            {
                show_window(&icon_window);
            }
        });
    }
}

#[cfg(all(feature = "desktop", target_os = "windows"))]
pub use windows_tray::use_windows_tray;
