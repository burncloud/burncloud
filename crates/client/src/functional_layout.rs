use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::use_auth,
    components::{Icon, Logo},
};

fn page_title(route: &Route) -> &'static str {
    match route {
        Route::Overview {} | Route::Dashboard {} => "Overview",
        Route::BuyerHome {} | Route::BuyerOverview {} => "Buyer Overview",
        Route::Playground {} => "Playground",
        Route::Routes {} => "Routes",
        Route::Models {} => "Models",
        Route::Providers {} => "Providers",
        Route::APIKeys {} => "API Keys",
        Route::Customers {} | Route::Users {} => "Customers",
        Route::Guardrails {} => "Guardrails",
        Route::Logs {} => "Logs",
        Route::Evaluation {} => "Evaluation",
        Route::Billing {} => "Billing",
        Route::Team {} => "Team",
        Route::Settings {} => "Settings",
        Route::Home {} | Route::Landing {} => "Home",
        Route::Login {} => "Sign In",
        Route::Register {} => "Register",
    }
}

fn search_route(query: &str) -> Option<Route> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return None;
    }

    let pages = [
        ("overview dashboard health status home console", Route::Overview {}),
        ("providers provider channels channel upstream supply", Route::Providers {}),
        ("models model catalog", Route::Models {}),
        ("routes routing groups priority weight traffic", Route::Routes {}),
        ("playground chat completion inference test", Route::Playground {}),
        ("logs requests router log observability errors", Route::Logs {}),
        ("evaluation metrics latency success performance", Route::Evaluation {}),
        ("billing cost usage spend finance", Route::Billing {}),
        ("api keys key tokens token access", Route::APIKeys {}),
        ("customers users user accounts account balance", Route::Customers {}),
        ("guardrails security filters risk circuit breaker", Route::Guardrails {}),
        ("team members roles operators admins", Route::Team {}),
        ("settings cache runtime server system", Route::Settings {}),
    ];

    pages
        .into_iter()
        .find(|(keywords, _)| keywords.split_whitespace().any(|word| word.starts_with(&q) || q.contains(word)))
        .map(|(_, route)| route)
}

fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|w| !w.is_empty())
        .collect();
    if words.len() >= 2 {
        let a = words[0].chars().next().unwrap_or('B');
        let b = words[1].chars().next().unwrap_or('C');
        format!("{}{}", a.to_ascii_uppercase(), b.to_ascii_uppercase())
    } else {
        name.chars().take(2).collect::<String>().to_ascii_uppercase()
    }
}

#[component]
pub fn FunctionalConsoleLayout() -> Element {
    let current = use_route::<Route>();
    let title = page_title(&current);
    let auth = use_auth();
    let navigator = use_navigator();
    let search_navigator = navigator.clone();
    let user = auth.user();
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "BurnCloud".to_string());
    let avatar = initials(&username);
    let roles = user.as_ref().map(|u| u.roles.join(", ")).unwrap_or_default();
    let mut search = use_signal(String::new);
    let mut search_status = use_signal(String::new);

    rsx! {
        div { class:"console-shell",
            aside { class:"sidebar",
                div { class:"sidebar-brand",
                    Link { to:Route::Overview {}, class:"brand-link", Logo {} span { class:"brand-name", "BurnCloud" } }
                }
                nav { class:"sidebar-nav",
                    NavGroup { title:"Workspace",
                        NavItem { to:Route::Overview {}, icon:"overview", label:"Overview" }
                    }
                    NavGroup { title:"Traffic Setup",
                        NavItem { to:Route::Providers {}, icon:"providers", label:"Providers" }
                        NavItem { to:Route::Models {}, icon:"models", label:"Models" }
                        NavItem { to:Route::Routes {}, icon:"routes", label:"Routes" }
                        NavItem { to:Route::Playground {}, icon:"terminal", label:"Playground" }
                    }
                    NavGroup { title:"Observe",
                        NavItem { to:Route::Logs {}, icon:"logs", label:"Logs" }
                        NavItem { to:Route::Evaluation {}, icon:"chart", label:"Evaluation" }
                        NavItem { to:Route::Billing {}, icon:"billing", label:"Billing" }
                    }
                    NavGroup { title:"Access & Customers",
                        NavItem { to:Route::APIKeys {}, icon:"key", label:"API Keys" }
                        NavItem { to:Route::Customers {}, icon:"users", label:"Customers" }
                    }
                    NavGroup { title:"Security & System",
                        NavItem { to:Route::Guardrails {}, icon:"shield", label:"Guardrails" }
                        NavItem { to:Route::Team {}, icon:"users", label:"Team" }
                        NavItem { to:Route::Settings {}, icon:"settings", label:"Settings" }
                    }
                    div { class:"public-links",
                        h4 { class:"nav-group-title", "External" }
                        div { class:"nav-items",
                            NavItem { to:Route::Home {}, icon:"globe", label:"Landing Page" }
                            button {
                                class:"nav-item",
                                style:"width:100%;text-align:left",
                                onclick:move |_| {
                                    auth.clear();
                                    navigator.replace(Route::Login {});
                                },
                                Icon { name:"logout" }
                                span { "Sign Out" }
                            }
                        }
                    }
                }
            }
            div { class:"console-main",
                header { class:"topbar",
                    h1 { class:"topbar-title", "{title}" }
                    div { class:"topbar-right",
                        div { class:"global-search", title: if search_status().is_empty() { "Jump to a console page" } else { "{search_status}" },
                            Icon { name:"search" }
                            input {
                                r#type:"text",
                                placeholder:"Jump to a page…",
                                value:"{search}",
                                oninput:move |evt| {
                                    search.set(evt.value());
                                    search_status.set(String::new());
                                },
                                onkeydown:move |evt| {
                                    if evt.key() == Key::Enter {
                                        let query = search();
                                        if let Some(route) = search_route(&query) {
                                            search_navigator.replace(route);
                                            search.set(String::new());
                                            search_status.set(String::new());
                                        } else {
                                            search_status.set("No console page matched that query.".to_string());
                                        }
                                    }
                                }
                            }
                        }
                        div { class:"top-actions",
                            Link { to:Route::Home {}, class:"tiny-link", Icon { name:"globe" } span { "Landing Page" } }
                            div { class:"env-chip", title:"BurnCloud server is configured; live health is shown on Overview", span { class:"green-dot" } "Server Configured" }
                            Link { to:Route::Logs {}, class:"icon-button", title:"Open request logs", Icon { name:"bell" } }
                            Link { to:Route::Settings {}, class:"icon-button", title:"System settings", Icon { name:"help" } }
                            div { class:"top-divider" }
                            div { class:"profile-link",
                                div { class:"avatar", "{avatar}" }
                                div { class:"two-line",
                                    span { class:"profile-name", "{username}" }
                                    if !roles.is_empty() { small { class:"subtle", "{roles}" } }
                                }
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
    rsx! { Link { to:to, class:"nav-item", active_class:"active", Icon { name:icon } span { "{label}" } } }
}
