use dioxus::prelude::*;
use crate::app::Route;

#[component]
pub fn Logo(#[props(default = "brand-logo".to_string())] class: String) -> Element {
    rsx! {
        svg {
            class: "{class}",
            view_box: "0 0 24 24",
            fill: "none",
            xmlns: "http://www.w3.org/2000/svg",
            defs {
                linearGradient { id: "burnCloudGrad", x1: "0", y1: "0", x2: "0", y2: "1",
                    stop { offset: "0%", stop_color: "#f7b52c" }
                    stop { offset: "100%", stop_color: "#e95513" }
                }
            }
            path {
                d: "M17.8 10.1q-.6-.9-1.4-1.9S14.6 6.1 14.9 3c0 0-6.9 2.7-7 8.2 0 0-1-1.6-.8-4.6 0 0-2.2 2.1-2.5 5.5-2.1.7-3.8 2.5-3.8 4.3 0 2.5 2.7 4.6 5.9 4.6-2.4-.4-4.2-2-4.2-4 0-1.4.8-2.5 2-3.3q.1 1.1.5 2.4s1.2 3.8 5.4 4.8c1.2.3 2.5.2 3.7-.3 1.3-.6 2.8-1.8 2.8-4.5 0 0 .1-2.7-1.5-4.1 0 0 2.1 5-1.8 6.5-1.3.5-2.6.5-3.9 0-1.7-.7-3.8-2.5-3.5-7.2 0 0 1 3.4 3.2 4.7 0 0-2-5.8 3.9-9.8 0 0 .5 2.1 1.9 3.3.4.4 4 3.2 3.3 8 .7-.9 1.3-3.1.7-4.8 0 0-.1-.4-.4-.9 1.5.3 2.7 1.5 2.8 4.2.1 2.3-1.6 4.2-3.8 5 3-.4 5.4-2.7 5.4-5.6 0-2.8-2.2-5.1-5.4-5.3z",
                fill: "url(#burnCloudGrad)"
            }
        }
    }
}

#[component]
pub fn Icon(name: &'static str) -> Element {
    let common = rsx! {};
    let _ = common;
    match name {
        "overview" => rsx! { svg { class: "nav-icon", view_box: "0 0 24 24", fill: "none", stroke: "currentColor", stroke_width: "2", rect { x:"3", y:"3", width:"7", height:"7", rx:"1" } rect { x:"14", y:"3", width:"7", height:"7", rx:"1" } rect { x:"3", y:"14", width:"7", height:"7", rx:"1" } rect { x:"14", y:"14", width:"7", height:"7", rx:"1" } } },
        "terminal" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", rect{x:"3",y:"4",width:"18",height:"16",rx:"2"} path{d:"m7 9 3 3-3 3"} path{d:"M13 15h4"} } },
        "routes" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", circle{cx:"6",cy:"6",r:"2"} circle{cx:"18",cy:"6",r:"2"} circle{cx:"12",cy:"18",r:"2"} path{d:"M8 6h8M7.4 7.7l3.3 8.4M16.6 7.7l-3.3 8.4"} } },
        "models" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", rect{x:"5",y:"5",width:"14",height:"14",rx:"2"} path{d:"M9 9h6v6H9zM9 1v4M15 1v4M9 19v4M15 19v4M1 9h4M19 9h4M1 15h4M19 15h4"} } },
        "providers" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", rect{x:"3",y:"4",width:"18",height:"6",rx:"2"} rect{x:"3",y:"14",width:"18",height:"6",rx:"2"} path{d:"M7 7h.01M7 17h.01"} } },
        "key" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", circle{cx:"8",cy:"15",r:"4"} path{d:"m11 12 8-8M15 8l3 3M17 6l3 3"} } },
        "users" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"} circle{cx:"9",cy:"7",r:"4"} path{d:"M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"} } },
        "shield" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"} path{d:"m9 12 2 2 4-4"} } },
        "logs" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M6 2h9l3 3v17H6z"} path{d:"M9 9h6M9 13h6M9 17h4"} } },
        "chart" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M3 3v18h18"} path{d:"m7 16 4-5 4 3 5-7"} } },
        "billing" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", rect{x:"2",y:"5",width:"20",height:"14",rx:"2"} path{d:"M2 10h20"} } },
        "settings" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", circle{cx:"12",cy:"12",r:"3"} path{d:"M19.4 15a1.7 1.7 0 0 0 .34 1.88l.06.06-2.83 2.83-.06-.06a1.7 1.7 0 0 0-1.88-.34 1.7 1.7 0 0 0-1.03 1.56V21h-4v-.09A1.7 1.7 0 0 0 9 19.36a1.7 1.7 0 0 0-1.88.34l-.06.06-2.83-2.83.06-.06A1.7 1.7 0 0 0 4.63 15 1.7 1.7 0 0 0 3.07 14H3v-4h.09A1.7 1.7 0 0 0 4.64 9a1.7 1.7 0 0 0-.34-1.88l-.06-.06 2.83-2.83.06.06A1.7 1.7 0 0 0 9 4.63h.01A1.7 1.7 0 0 0 10 3.07V3h4v.09A1.7 1.7 0 0 0 15 4.64a1.7 1.7 0 0 0 1.88-.34l.06-.06 2.83 2.83-.06.06A1.7 1.7 0 0 0 19.37 9v.01A1.7 1.7 0 0 0 20.93 10H21v4h-.09A1.7 1.7 0 0 0 19.4 15z"} } },
        "globe" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", circle{cx:"12",cy:"12",r:"9"} path{d:"M3 12h18M12 3a15 15 0 0 1 0 18M12 3a15 15 0 0 0 0 18"} } },
        "logout" => rsx! { svg { class:"nav-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M10 17l5-5-5-5M15 12H3M21 19V5a2 2 0 0 0-2-2h-5"} } },
        "search" => rsx! { svg { class:"search-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", circle{cx:"11",cy:"11",r:"7"} path{d:"m20 20-3.5-3.5"} } },
        "bell" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", width:"19", height:"19", path{d:"M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9M10 21h4"} } },
        "help" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", width:"19", height:"19", circle{cx:"12",cy:"12",r:"9"} path{d:"M9.1 9a3 3 0 1 1 5.8 1c-.6 1-1.9 1.3-2.4 2.3-.2.4-.3.8-.3 1.2M12 17h.01"} } },
        "activity" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M3 12h4l2-7 4 14 2-7h6"} } },
        "dollar" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", circle{cx:"12",cy:"12",r:"9"} path{d:"M16 8h-6a2 2 0 0 0 0 4h4a2 2 0 0 1 0 4H8M12 6v12"} } },
        "server" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", rect{x:"3",y:"4",width:"18",height:"6",rx:"2"} rect{x:"3",y:"14",width:"18",height:"6",rx:"2"} path{d:"M7 7h.01M7 17h.01"} } },
        "spark" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"m12 3 1.6 4.4L18 9l-4.4 1.6L12 15l-1.6-4.4L6 9l4.4-1.6L12 3zM19 15l.8 2.2L22 18l-2.2.8L19 21l-.8-2.2L16 18l2.2-.8L19 15z"} } },
        "plus" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M12 5v14M5 12h14"} } },
        "play" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"m8 5 11 7-11 7V5z"} } },
        "download" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M12 3v12M7 10l5 5 5-5M5 21h14"} } },
        "wifi" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", path{d:"M5 12.5a11 11 0 0 1 14 0M8 16a6 6 0 0 1 8 0M11 19.5a1.5 1.5 0 0 1 2 0"} } },
        "lock" => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", rect{x:"4",y:"10",width:"16",height:"11",rx:"2"} path{d:"M8 10V7a4 4 0 0 1 8 0v3"} } },
        _ => rsx! { svg { view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"2", circle{cx:"12",cy:"12",r:"8"} } },
    }
}

fn page_title(route: &Route) -> &'static str {
    match route {
        Route::Overview {} => "Overview",
        Route::Playground {} => "Playground",
        Route::Routes {} => "Routes",
        Route::Models {} => "Models",
        Route::Providers {} => "Providers",
        Route::APIKeys {} => "API Keys",
        Route::Customers {} => "Customers",
        Route::Guardrails {} => "Guardrails",
        Route::Logs {} => "Logs",
        Route::Evaluation {} => "Evaluation",
        Route::Billing {} => "Billing",
        Route::Team {} => "Team",
        Route::Settings {} => "Settings",
        _ => "BurnCloud",
    }
}

#[component]
pub fn ConsoleLayout() -> Element {
    let current = use_route::<Route>();
    let title = page_title(&current);
    rsx! {
        div { class:"console-shell",
            aside { class:"sidebar",
                div { class:"sidebar-brand",
                    Link { to:Route::Overview {}, class:"brand-link",
                        Logo {}
                        span { class:"brand-name", "BurnCloud" }
                    }
                }
                nav { class:"sidebar-nav",
                    NavGroup { title:"Core Platform",
                        NavItem { to:Route::Overview {}, icon:"overview", label:"Overview" }
                        NavItem { to:Route::Playground {}, icon:"terminal", label:"Playground" }
                        NavItem { to:Route::Routes {}, icon:"routes", label:"Routes" }
                        NavItem { to:Route::Models {}, icon:"models", label:"Models" }
                        NavItem { to:Route::Providers {}, icon:"providers", label:"Providers" }
                    }
                    NavGroup { title:"Access & Control",
                        NavItem { to:Route::APIKeys {}, icon:"key", label:"API Keys" }
                        NavItem { to:Route::Customers {}, icon:"users", label:"Customers" }
                        NavItem { to:Route::Guardrails {}, icon:"shield", label:"Guardrails" }
                    }
                    NavGroup { title:"Ops & Finance",
                        NavItem { to:Route::Logs {}, icon:"logs", label:"Logs" }
                        NavItem { to:Route::Evaluation {}, icon:"chart", label:"Evaluation" }
                        NavItem { to:Route::Billing {}, icon:"billing", label:"Billing" }
                    }
                    NavGroup { title:"System",
                        NavItem { to:Route::Team {}, icon:"users", label:"Team" }
                        NavItem { to:Route::Settings {}, icon:"settings", label:"Settings" }
                    }
                    div { class:"public-links",
                        h4 { class:"nav-group-title", "Public Portal" }
                        div { class:"nav-items",
                            NavItem { to:Route::Home {}, icon:"globe", label:"Landing Page" }
                            NavItem { to:Route::Login {}, icon:"logout", label:"Sign Out / Login" }
                        }
                    }
                }
            }
            div { class:"console-main",
                header { class:"topbar",
                    h1 { class:"topbar-title", "{title}" }
                    div { class:"topbar-right",
                        div { class:"global-search",
                            Icon { name:"search" }
                            input { r#type:"text", placeholder:"Search routes, keys, logs..." }
                        }
                        div { class:"top-actions",
                            Link { to:Route::Home {}, class:"tiny-link", Icon { name:"globe" } span { "Visit Landing Page" } }
                            button { class:"env-chip", span { class:"green-dot" } "Production" span { class:"subtle", "⌄" } }
                            button { class:"icon-button", Icon { name:"bell" } span { class:"notification-dot" } }
                            button { class:"icon-button", Icon { name:"help" } }
                            div { class:"top-divider" }
                            Link { to:Route::Login {}, class:"profile-link",
                                div { class:"avatar", "WH" }
                                span { class:"profile-name", "BurnCloud AI" }
                            }
                        }
                    }
                }
                main { class:"content-scroll", Outlet::<Route> {} }
            }
        }
    }
}

#[component]
fn NavGroup(title: &'static str, children: Element) -> Element {
    rsx! { div { class:"nav-group", h4 { class:"nav-group-title", "{title}" } div { class:"nav-items", {children} } } }
}

#[component]
fn NavItem(to: Route, icon: &'static str, label: &'static str) -> Element {
    rsx! {
        Link { to:to, class:"nav-item", active_class:"active",
            Icon { name:icon }
            span { "{label}" }
        }
    }
}

#[component]
pub fn MetricCard(label: &'static str, value: &'static str, note: &'static str, icon: &'static str, tone: &'static str) -> Element {
    rsx! {
        div { class:"card metric card-hover",
            div { class:"metric-copy",
                span { class:"metric-label", "{label}" }
                span { class:"metric-value", "{value}" }
                if !note.is_empty() { span { class:"metric-note mono", "{note}" } }
            }
            div { class:"metric-icon {tone}", Icon { name:icon } }
        }
    }
}

#[component]
pub fn Badge(text: &'static str, #[props(default = "neutral".to_string())] tone: String) -> Element {
    rsx! { span { class:"badge badge-{tone}", "{text}" } }
}

#[component]
pub fn Drawer(title: &'static str, open: bool, on_close: EventHandler<MouseEvent>, children: Element) -> Element {
    if !open { return rsx! {}; }
    rsx! {
        div { class:"drawer-backdrop", onclick:move |evt| on_close.call(evt) }
        aside { class:"drawer",
            div { class:"drawer-head", h2 { "{title}" } button { class:"close-button", onclick:move |evt| on_close.call(evt), "×" } }
            div { class:"drawer-body", {children} }
        }
    }
}
