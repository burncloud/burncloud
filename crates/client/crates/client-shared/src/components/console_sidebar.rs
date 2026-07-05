//! Shared console navigation sidebar — generic over the router enum (`Route` or `CoreRoute`).

use crate::components::logo::Logo;
use crate::i18n::{t, use_i18n};
use dioxus::prelude::*;
use dioxus_router::components::Link;
use dioxus_router::hooks::use_route;
use dioxus_router::Routable;

/// Console nav targets shared by the main app and dev shell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavId {
    Dashboard,
    Channel,
    Connect,
    Access,
    Playground,
    Monitor,
    Logs,
    Users,
    Finance,
    Settings,
}

/// Maps [`NavId`] to a concrete router enum variant.
pub trait ConsoleNav: Sized {
    type Route: Routable + Clone + PartialEq;
    fn nav_route(id: NavId) -> Self::Route;
}

struct NavItemDef {
    id: NavId,
    icon_path: &'static str,
    label_key: Option<&'static str>,
    fixed_label: Option<&'static str>,
}

const NAV_ITEMS: &[(NavSection, &[NavItemDef])] = &[
    (
        NavSection::Core,
        &[
            NavItemDef {
                id: NavId::Dashboard,
                icon_path: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z",
                label_key: Some("nav.dashboard"),
                fixed_label: None,
            },
            NavItemDef {
                id: NavId::Channel,
                icon_path: "M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z",
                label_key: Some("models.channel.title"),
                fixed_label: None,
            },
            NavItemDef {
                id: NavId::Connect,
                icon_path: "M13 10V3L4 14h7v7l9-11h-7z",
                label_key: None,
                fixed_label: Some("Connect"),
            },
            NavItemDef {
                id: NavId::Access,
                icon_path: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z",
                label_key: Some("nav.api"),
                fixed_label: None,
            },
        ],
    ),
    (
        NavSection::Apps,
        &[NavItemDef {
            id: NavId::Playground,
            icon_path: "M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z",
            label_key: Some("playground.title"),
            fixed_label: None,
        }],
    ),
    (
        NavSection::Operations,
        &[
            NavItemDef {
                id: NavId::Monitor,
                icon_path: "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z",
                label_key: Some("nav.monitor"),
                fixed_label: None,
            },
            NavItemDef {
                id: NavId::Logs,
                icon_path: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4",
                label_key: None,
                fixed_label: Some("Logs"),
            },
            NavItemDef {
                id: NavId::Users,
                icon_path: "M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z",
                label_key: Some("nav.users"),
                fixed_label: None,
            },
            NavItemDef {
                id: NavId::Finance,
                icon_path: "M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z",
                label_key: None,
                fixed_label: Some("Billing"),
            },
        ],
    ),
];

#[derive(Clone, Copy)]
enum NavSection {
    Core,
    Apps,
    Operations,
}

impl NavSection {
    fn title(self) -> &'static str {
        match self {
            NavSection::Core => "Core",
            NavSection::Apps => "Apps",
            NavSection::Operations => "Operations",
        }
    }
}

fn label_for(lang: crate::i18n::Language, item: &NavItemDef) -> String {
    if let Some(fixed) = item.fixed_label {
        fixed.to_string()
    } else if let Some(key) = item.label_key {
        t(lang, key).to_string()
    } else {
        String::new()
    }
}

#[component]
fn SidebarItemLink<R: Routable + Clone + PartialEq + 'static>(
    to: R,
    label: String,
    icon_path: &'static str,
    #[props(default)] icon_path_extra: Option<&'static str>,
) -> Element {
    let route = use_route::<R>();
    let active = route == to;

    rsx! {
        Link {
            to,
            class: format!("nav-item {}", if active { "active" } else { "" }),
            div { class: format!("icon {}", if active { "text-on-accent" } else { "opacity-70" }),
                svg {
                    fill: "none",
                    view_box: "0 0 24 24",
                    stroke: "currentColor",
                    stroke_width: "2",
                    path {
                        stroke_linecap: "round",
                        stroke_linejoin: "round",
                        d: icon_path,
                    }
                    if let Some(extra) = icon_path_extra {
                        path {
                            stroke_linecap: "round",
                            stroke_linejoin: "round",
                            d: extra,
                        }
                    }
                }
            }
            span { "{label}" }
        }
    }
}

#[component]
pub fn ConsoleSidebar<N: ConsoleNav>() -> Element
where
    N::Route: Routable + Clone + PartialEq + 'static,
{
    let i18n = use_i18n();
    let lang = *i18n.language.read();

    rsx! {
        div { class: "flex flex-col h-full gap-6 select-none pt-4",
            div { class: "px-6 pb-2",
                div { class: "flex items-center gap-3",
                    Logo { class: "w-8 h-8 fill-current" }
                    div { class: "flex flex-col",
                        span { class: "text-base font-semibold tracking-tight leading-none text-bc-text", "BurnCloud" }
                        span { class: "text-[11px] font-medium text-bc-text-tertiary uppercase tracking-widest leading-none mt-1", "Enterprise" }
                    }
                }
            }

            for (section, items) in NAV_ITEMS {
                div { class: "flex flex-col gap-1",
                    div { class: "px-6 text-[11px] font-semibold text-bc-text-tertiary uppercase tracking-widest mb-2 mt-2",
                        "{section.title()}"
                    }
                    for item in *items {
                        SidebarItemLink::<N::Route> {
                            to: N::nav_route(item.id),
                            label: label_for(lang, item),
                            icon_path: item.icon_path,
                        }
                    }
                }
            }

            div { class: "mt-auto flex flex-col gap-1 pb-4",
                div { class: "sidebar-divider my-2" }
                SidebarItemLink::<N::Route> {
                    to: N::nav_route(NavId::Settings),
                    label: t(lang, "nav.settings").to_string(),
                    icon_path: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z",
                    icon_path_extra: Some("M15 12a3 3 0 11-6 0 3 3 0 016 0z"),
                }
            }
        }
    }
}
