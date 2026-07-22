use dioxus::prelude::*;
use dioxus_router::components::Outlet;

use crate::app::Route;
use crate::components::app_styles::AppStyles;
use burncloud_client_shared::{components::TitleBar, DesktopMode};

#[component]
pub fn GuestLayout() -> Element {
    let is_desktop = try_use_context::<DesktopMode>().is_some();

    rsx! {
        if is_desktop {
            head {
                link { rel: "icon", href: "favicon.ico" }
                AppStyles {}
            }
        }

        div { class: "h-screen w-screen overflow-hidden flex flex-col bg-bc-canvas text-bc-text relative", "data-theme": "light",
            div { class: "absolute top-0 left-0 w-full z-50 pointer-events-none",
                div { class: "pointer-events-auto",
                    TitleBar {}
                }
            }

            main { class: "flex-1 flex flex-col relative z-0 overflow-y-auto min-h-0",
                Outlet::<Route> {}
            }
        }
    }
}
