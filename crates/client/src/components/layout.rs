use dioxus::prelude::*;

use crate::app::Route;
use crate::components::app_styles::AppStyles;
use crate::components::sidebar::Sidebar;
use burncloud_client_shared::components::TitleBar;
use burncloud_client_shared::use_auth;
use burncloud_client_shared::use_theme;
use burncloud_client_shared::DesktopMode;

#[component]
pub fn Layout() -> Element {
    let auth = use_auth();
    let navigator = use_navigator();
    let is_desktop = try_use_context::<DesktopMode>().is_some();
    let theme_ctx = use_theme();
    let theme_str = theme_ctx.data_theme_attr();

    use_effect(move || {
        if !auth.is_authenticated() {
            navigator.replace(Route::LoginPage {});
        }
    });

    if !auth.is_authenticated() {
        return rsx! {
            div { class: "h-screen w-screen flex items-center justify-center bg-bc-canvas",
                span { class: "bc-spinner bc-spinner--lg" }
            }
        };
    }

    rsx! {
        head {
            AppStyles {}
        }

        div { class: "relative flex flex-col h-screen w-screen bg-bc-canvas text-bc-text overflow-hidden select-none", "data-theme": "{theme_str}",

            if is_desktop {
                div { class: "absolute top-0 left-0 w-full z-50 pointer-events-none",
                    div { class: "pointer-events-auto",
                        TitleBar {}
                    }
                }
            }

            div { class: "flex flex-1 min-h-0 overflow-hidden w-full",

                div { class: "w-64 shrink-0 flex flex-col border-r border-bc-border/50 bg-bc-canvas/80 backdrop-blur-xl",
                    div { class: "flex-1 overflow-y-auto px-bc-2 py-bc-4",
                        Sidebar {}
                    }
                }

                div { class: "flex-1 flex flex-col bg-bc-canvas relative min-w-0",
                    main { class: "flex-1 overflow-y-auto overflow-x-hidden flex flex-col min-h-0",
                        Outlet::<Route> {}
                    }
                }
            }
        }
    }
}
