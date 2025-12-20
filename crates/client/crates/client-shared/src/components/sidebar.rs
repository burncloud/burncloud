use crate::i18n::{t, use_i18n};
use dioxus::prelude::*;
use dioxus_router::components::Link;
use dioxus_router::hooks::use_route;

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
            class: format!("flex items-center gap-3 px-4 py-2.5 mx-2 rounded-md text-sm transition-all duration-200 group {}",
                if active { "bg-base-content/5 text-base-content font-medium shadow-sm" }
                else { "text-base-content/60 hover:bg-base-content/5 hover:text-base-content" }
            ),
            // Icon
            div { class: format!("w-5 h-5 {}", if active { "text-base-content" } else { "opacity-70 group-hover:opacity-100" }),
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
        div { class: "flex flex-col h-full gap-6 select-none pt-4",

            // Brand Area - Minimalist & Premium
            div { class: "px-6 pb-2",
                div { class: "flex items-center gap-3",
                    div { class: "w-8 h-8 bg-black rounded-lg shadow-sm flex items-center justify-center text-white",
                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M17.657 18.657A8 8 0 016.343 7.343S7 9 9 10c0-2 .5-5 2.986-7C14 5 16.09 5.777 17.656 7.343A7.975 7.975 0 0120 13a7.975 7.975 0 01-2.343 5.657z" }
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9.879 16.121A3 3 0 1012.015 11L11 14H9c0 .768.293 1.536.879 2.121z" }
                        }
                    }
                    div { class: "flex flex-col",
                        span { class: "text-base font-semibold tracking-tight leading-none text-base-content", "BurnCloud" }
                        span { class: "text-[11px] font-medium text-base-content/40 uppercase tracking-widest leading-none mt-1", "Enterprise" }
                    }
                }
            }

                        // Section 1: Core Assets
                        div { class: "flex flex-col gap-1",
                            div { class: "px-6 text-[11px] font-semibold text-base-content/40 uppercase tracking-widest mb-2 mt-2", "核心资产" }

                            SidebarItem {
                                to: CoreRoute::Dashboard {},
                                label: "仪表盘", // Dashboard
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" } } }
                            }
                            SidebarItem {
                                to: CoreRoute::ChannelPage {},
                                label: "模型网络", // Model Network
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" } } }
                            }
                            SidebarItem {
                                to: CoreRoute::BurnGridPage {},
                                label: "BurnGrid", // BurnGrid
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" } } }
                            }
                            SidebarItem {
                                to: CoreRoute::ApiManagement {},
                                label: "访问凭证", // API KEY
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" } } }
                            }
                        }

                        // Section 2: Application Center
                        div { class: "flex flex-col gap-1",
                            div { class: "px-6 text-[11px] font-semibold text-base-content/40 uppercase tracking-widest mb-2 mt-2", "应用中心" }

                            SidebarItem {
                                to: CoreRoute::PlaygroundPage {},
                                label: "演练场", // Playground
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" } } }
                            }
                        }

                        // Section 3: Operations
                        div { class: "flex flex-col gap-1",
                            div { class: "px-6 text-[11px] font-semibold text-base-content/40 uppercase tracking-widest mb-2 mt-2", "运营中心" }

                            SidebarItem {
                                to: CoreRoute::ServiceMonitor {},
                                label: "风控雷达", // Risk Radar
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" } } }
                            }
                             SidebarItem {
                                to: CoreRoute::LogPage {},
                                label: "日志审查", // Log Review
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2m-6 9l2 2 4-4" } } }
                            }
                             SidebarItem {
                                to: CoreRoute::UserPage {},
                                label: "用户管理", // Client List
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" } } }
                            }
                            SidebarItem {
                                to: CoreRoute::BillingPage {},
                                label: "财务中心", // Billing
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3 10h18M7 15h1m4 0h1m-7 4h12a3 3 0 003-3V8a3 3 0 00-3-3H6a3 3 0 00-3 3v8a3 3 0 003 3z" } } }
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
            }
        }
    }
}
