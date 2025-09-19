use dioxus::prelude::*;
use dioxus_router::prelude::*;

use crate::app::Route;
use crate::components::{sidebar::Sidebar, title_bar::TitleBar};
use crate::styles::FLUENT_CSS;

#[component]
pub fn Layout() -> Element {
    rsx! {
        head {
            style { "{FLUENT_CSS}" }
        }
        div { class: "app-container",
            TitleBar {}
            div { class: "app-body",
                Sidebar {}
                main { class: "main-content",
                    Outlet::<Route> {}
                }
            }
        }
    }
}