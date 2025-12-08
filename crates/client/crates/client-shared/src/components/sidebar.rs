use dioxus::prelude::*;
use dioxus_router::components::Link;
use dioxus_router::hooks::use_route;
use crate::i18n::{use_i18n, t};

use super::layout::CoreRoute;

#[derive(Props, Clone, PartialEq)]
struct SidebarItemProps {
    to: CoreRoute,
    label: String,
    icon: Element,
}

#[component]
fn SidebarItem(props: SidebarItemProps) -> Element {
    let route = use_route::<CoreRoute>();
    // Comparison: check if current route matches target
    let active = matches!(route, _r if _r == props.to);

    rsx! {
        Link {
            to: props.to,
            class: format!("flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-all duration-200 group {}", 
                if active { "bg-base-content/10 font-semibold text-base-content" } 
                else { "text-base-content/70 hover:bg-base-content/5 hover:text-base-content" }
            ),
            // Icon
            div { class: format!("w-5 h-5 {}", if active { "text-primary" } else { "opacity-70 group-hover:opacity-100" }),
                {props.icon}
            }
            // Label
            span { "{props.label}" }
        }
    }
}

#[component]
pub fn Sidebar() -> Element {
    let i18n = use_i18n();
    let lang = i18n.language.read();

    rsx! {
        div { class: "flex flex-col h-full gap-6 select-none",
            
            // Brand Area - Minimalist & Premium
            div { class: "px-2 pt-2 pb-4",
                div { class: "flex items-center gap-3",
                    div { class: "w-8 h-8 bg-gradient-to-tr from-orange-500 to-red-600 rounded-lg shadow-sm flex items-center justify-center text-white",
                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2.5",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M17.657 18.657A8 8 0 016.343 7.343S7 9 9 10c0-2 .5-5 2.986-7C14 5 16.09 5.777 17.656 7.343A7.975 7.975 0 0120 13a7.975 7.975 0 01-2.343 5.657z" }
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.879 16.121A3 3 0 1012.015 11L11 14H9c0 .768.293 1.536.879 2.121z" }
                        }
                    }
                    div { class: "flex flex-col",
                        span { class: "text-sm font-bold tracking-tight leading-none", "BurnCloud" }
                        span { class: "text-[10px] font-medium opacity-50 uppercase tracking-widest leading-none mt-1", "Enterprise" }
                    }
                }
            }

            // Section 1: Intelligence
            div { class: "flex flex-col gap-1",
                div { class: "px-3 text-[10px] font-bold text-base-content/40 uppercase tracking-widest mb-1", "Intelligence" }
                
                SidebarItem { 
                    to: CoreRoute::Dashboard {}, 
                    label: t(*lang, "nav.dashboard"),
                    icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" } } }
                }
                SidebarItem { 
                    to: CoreRoute::ModelManagement {}, 
                    label: "Library", // Was "Models"
                    icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253" } } }
                }
                SidebarItem { 
                    to: CoreRoute::DeployConfig {}, 
                    label: "Memory", // New Placeholder for Universal Memory
                    icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.428 15.428a2 2 0 00-1.022-.547l-2.384-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" } } }
                }
            }

            // Section 2: Network
            div { class: "flex flex-col gap-1",
                div { class: "px-3 text-[10px] font-bold text-base-content/40 uppercase tracking-widest mb-1 mt-2", "Network" }
                
                SidebarItem { 
                    to: CoreRoute::ChannelPage {}, 
                    label: "BurnGrid", // Was "Channels"
                    icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9" } } }
                }
                SidebarItem { 
                    to: CoreRoute::ApiManagement {}, 
                    label: "Connect", // Was "API"
                    icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" } } }
                }
                SidebarItem { 
                    to: CoreRoute::ServiceMonitor {}, 
                    label: t(*lang, "nav.monitor"),
                    icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" } } }
                }
                 SidebarItem { 
                    to: CoreRoute::UserPage {}, 
                    label: t(*lang, "nav.users"),
                    icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 4.354a4 4 0 110 5.292M15 21H3v-1a6 6 0 0112 0v1zm0 0h6v-1a6 6 0 00-9-5.197M13 7a4 4 0 11-8 0 4 4 0 018 0z" } } }
                }
            }

            // Bottom Section: Settings
            div { class: "mt-auto flex flex-col gap-1 pb-4",
                 div { class: "divider my-2 opacity-50" }
                 SidebarItem { 
                    to: CoreRoute::SystemSettings {}, 
                    label: t(*lang, "nav.settings"),
                    icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" } 
                                 path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" } } }
                }
                 div { class: "px-3 mt-2",
                     div { class: "text-[10px] text-base-content/30 font-mono", "v0.1.5 build.821" }
                 }
            }
        }
    }
}