use dioxus::prelude::*;
use dioxus_router::components::Link;
use dioxus_router::hooks::use_route;
use crate::i18n::{use_i18n, t};

use super::layout::CoreRoute;

#[component]
pub fn Sidebar() -> Element {
    let route = use_route::<CoreRoute>();
    let i18n = use_i18n();
    let lang = i18n.language.read(); // Subscribe to updates

    rsx! {
        nav { class: "sidebar",
            div { class: "p-lg",
                div { class: "flex flex-col gap-xs",
                    Link {
                        to: CoreRoute::Dashboard {},
                        class: if matches!(route, CoreRoute::Dashboard {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "🏠" }
                        span { "{t(*lang, \"nav.dashboard\")}" }
                    }
                    Link {
                        to: CoreRoute::ModelManagement {},
                        class: if matches!(route, CoreRoute::ModelManagement {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "🧠" }
                        span { "{t(*lang, \"nav.models\")}" }
                    }
                    Link {
                        to: CoreRoute::DeployConfig {},
                        class: if matches!(route, CoreRoute::DeployConfig {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "⚙️" }
                        span { "{t(*lang, \"nav.deploy\")}" }
                    }
                    Link {
                        to: CoreRoute::ServiceMonitor {},
                        class: if matches!(route, CoreRoute::ServiceMonitor {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "📊" }
                        span { "{t(*lang, \"nav.monitor\")}" }
                    }
                    Link {
                        to: CoreRoute::ApiManagement {},
                        class: if matches!(route, CoreRoute::ApiManagement {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "🔗" }
                        span { "{t(*lang, \"nav.api\")}" }
                    }
                    Link {
                        to: CoreRoute::UserPage {},
                        class: if matches!(route, CoreRoute::UserPage {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "👥" }
                        span { "{t(*lang, \"nav.users\")}" }
                    }
                    Link {
                        to: CoreRoute::ChannelPage {},
                        class: if matches!(route, CoreRoute::ChannelPage {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "📡" }
                        span { "{t(*lang, \"nav.channels\")}" }
                    }
                    Link {
                        to: CoreRoute::SystemSettings {},
                        class: if matches!(route, CoreRoute::SystemSettings {}) { "nav-item active" } else { "nav-item" },
                        span { class: "icon", "🔧" }
                        span { "{t(*lang, \"nav.settings\")}" }
                    }
                }
            }

            div { class: "mt-auto p-lg border-t",
                style: "border-color: var(--neutral-quaternary);",
                div { class: "text-caption text-secondary",
                    "Status: 2 {t(*lang, \"status.models_running\")}"
                }
                div { class: "text-caption text-secondary",
                    "CPU: 45% | RAM: 2.1GB"
                }
            }
        }
    }
}