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

                        // Section 1: Core Assets (Was Intelligence)
                        div { class: "flex flex-col gap-1",
                            div { class: "px-3 text-[10px] font-bold text-base-content/40 uppercase tracking-widest mb-1", "核心资产" }

                            SidebarItem {
                                to: CoreRoute::Dashboard {},
                                label: "控制台", // Dashboard
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6zM14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6zM4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2zM14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" } } }
                            }
                            SidebarItem {
                                to: CoreRoute::ModelManagement {},
                                label: "账号矩阵", // Account Matrix
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10" } } }
                            }
                            SidebarItem {
                                to: CoreRoute::DeployConfig {},
                                label: "配额管理", // Quota Manager
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M11 3.055A9.001 9.001 0 1020.945 13H11V3.055z" } path { stroke_linecap: "round", stroke_linejoin: "round", d: "M20.488 9H15V3.512A9.025 9.025 0 0120.488 9z" } } }
                            }
                        }

                        // Section 2: Operations (Was Network)
                        div { class: "flex flex-col gap-1",
                            div { class: "px-3 text-[10px] font-bold text-base-content/40 uppercase tracking-widest mb-1 mt-2", "运营中心" }

                            SidebarItem {
                                to: CoreRoute::ChannelPage {},
                                label: "财务总览", // Financials
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" } } }
                            }
                            SidebarItem {
                                to: CoreRoute::ApiManagement {},
                                label: "API 网关", // API Gateway
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M8 9l3 3-3 3m5 0h3M5 20h14a2 2 0 002-2V6a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z" } } }
                            }
                            SidebarItem {
                                to: CoreRoute::ServiceMonitor {},
                                label: "风控雷达", // Risk Radar
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z" } } }
                            }
                             SidebarItem {
                                to: CoreRoute::UserPage {},
                                label: "客户列表", // Client List
                                icon: rsx! { svg { fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2", path { stroke_linecap: "round", stroke_linejoin: "round", d: "M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" } } }
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
