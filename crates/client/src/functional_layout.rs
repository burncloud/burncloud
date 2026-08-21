use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{use_auth, CurrentUser},
    components::{Icon, Logo},
};

fn page_title(route: &Route) -> &'static str {
    match route {
        Route::Overview {} | Route::BuyerOverview {} | Route::Dashboard {} => "Overview",
        Route::BuyerPlayground {} => "Playground",
        Route::BuyerMarketplace {} => "Marketplace",
        Route::BuyerAPIKeys {} => "API Keys",
        Route::BuyerUsage {} => "Usage",
        Route::BuyerBilling {} => "Billing",
        Route::BuyerLogs {} => "Logs",
        Route::SupplierWorkspace {} => "Supplier",
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

fn search_route(query: &str, buyer_workspace: bool) -> Option<Route> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return None;
    }

    if buyer_workspace {
        let pages = [
            ("overview buyer consumer console", Route::BuyerOverview {}),
            ("playground chat test request", Route::BuyerPlayground {}),
            ("marketplace models catalog", Route::BuyerMarketplace {}),
            ("api keys credentials access", Route::BuyerAPIKeys {}),
            ("usage tokens spend funding", Route::BuyerUsage {}),
            ("billing funding requests", Route::BuyerBilling {}),
            ("logs requests history", Route::BuyerLogs {}),
        ];
        return pages
            .into_iter()
            .find(|(keywords, _)| {
                keywords
                    .split_whitespace()
                    .any(|word| word.starts_with(&q) || q.contains(word))
            })
            .map(|(_, route)| route);
    }

    let pages = [
        (
            "overview dashboard health status home console",
            Route::Overview {},
        ),
        (
            "providers provider channels channel upstream supply",
            Route::Providers {},
        ),
        ("models model catalog", Route::Models {}),
        (
            "routes routing groups priority weight traffic",
            Route::Routes {},
        ),
        (
            "playground chat completion inference test",
            Route::Playground {},
        ),
        (
            "logs requests router log observability errors",
            Route::Logs {},
        ),
        (
            "evaluation metrics latency success performance",
            Route::Evaluation {},
        ),
        ("billing cost usage spend finance", Route::Billing {}),
        ("api keys key tokens token access", Route::APIKeys {}),
        (
            "customers users user accounts account balance",
            Route::Customers {},
        ),
        (
            "guardrails security filters risk circuit breaker",
            Route::Guardrails {},
        ),
        ("team members roles operators admins", Route::Team {}),
        ("settings cache runtime server system", Route::Settings {}),
    ];

    pages
        .into_iter()
        .find(|(keywords, _)| {
            keywords
                .split_whitespace()
                .any(|word| word.starts_with(&q) || q.contains(word))
        })
        .map(|(_, route)| route)
}

fn is_buyer_workspace_route(route: &Route) -> bool {
    matches!(
        route,
        Route::BuyerOverview {}
            | Route::BuyerPlayground {}
            | Route::BuyerMarketplace {}
            | Route::BuyerAPIKeys {}
            | Route::BuyerUsage {}
            | Route::BuyerBilling {}
            | Route::BuyerLogs {}
    )
}

fn has_role(user: Option<&CurrentUser>, expected: &str) -> bool {
    user.is_some_and(|user| {
        user.roles
            .iter()
            .any(|role| role.eq_ignore_ascii_case(expected))
    })
}

fn has_buyer_access(user: Option<&CurrentUser>) -> bool {
    user.is_some_and(|user| {
        user.roles
            .iter()
            .any(|role| role.eq_ignore_ascii_case("buyer") || role.eq_ignore_ascii_case("user"))
    })
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
        name.chars()
            .take(2)
            .collect::<String>()
            .to_ascii_uppercase()
    }
}

#[component]
pub fn FunctionalConsoleLayout() -> Element {
    let current = use_route::<Route>();
    let title = page_title(&current);
    let buyer_route = is_buyer_workspace_route(&current);
    let supplier_route = matches!(current, Route::SupplierWorkspace {});
    let auth = use_auth();
    let navigator = use_navigator();
    let search_navigator = navigator.clone();
    let user = auth.user();
    if (buyer_route && !has_buyer_access(user.as_ref()))
        || (supplier_route && !has_role(user.as_ref(), "supplier"))
    {
        return rsx! {
            div { class: "auth-page",
                main { class: "auth-main",
                    div { class: "card card-pad stack", style: "max-width:480px;margin:auto",
                        strong { "Workspace access required" }
                        span { class: "small muted", "This account does not hold the role required for this workspace." }
                    }
                }
            }
        };
    }
    let username = user
        .as_ref()
        .map(|u| u.username.clone())
        .unwrap_or_else(|| "BurnCloud".to_string());
    let avatar = initials(&username);
    let roles = user
        .as_ref()
        .map(|u| u.roles.join(", "))
        .unwrap_or_default();
    let mut search = use_signal(String::new);
    let mut search_status = use_signal(String::new);

    rsx! {
        div { class:"console-shell",
            aside { class:"sidebar",
                div { class:"sidebar-brand",
                    Link { to:if buyer_route { Route::BuyerOverview {} } else if supplier_route { Route::SupplierWorkspace {} } else { Route::Overview {} }, class:"brand-link", Logo {} span { class:"brand-name", "BurnCloud" } }
                }
                nav { class:"sidebar-nav",
                    div { class:"buyer-workspace-label",
                        span { "Workspace" }
                        strong { if buyer_route { "Buyer" } else if supplier_route { "Supplier" } else { "Admin" } }
                        div { class:"workspace-switcher", aria_label:"Switch workspace",
                            if has_buyer_access(user.as_ref()) { Link { to:Route::BuyerOverview {}, "Buyer" } }
                            if has_role(user.as_ref(), "supplier") { Link { to:Route::SupplierWorkspace {}, "Supplier" } }
                            if has_role(user.as_ref(), "admin") { Link { to:Route::Overview {}, "Admin" } }
                        }
                    }
                    if buyer_route {
                        NavGroup { title:"Buyer",
                            NavItem { to:Route::BuyerOverview {}, icon:"overview", label:"Overview" }
                            NavItem { to:Route::BuyerPlayground {}, icon:"terminal", label:"Playground" }
                            NavItem { to:Route::BuyerMarketplace {}, icon:"models", label:"Marketplace" }
                            NavItem { to:Route::BuyerAPIKeys {}, icon:"key", label:"API Keys" }
                            NavItem { to:Route::BuyerUsage {}, icon:"chart", label:"Usage" }
                            NavItem { to:Route::BuyerBilling {}, icon:"billing", label:"Billing" }
                            NavItem { to:Route::BuyerLogs {}, icon:"logs", label:"Logs" }
                        }
                    } else if supplier_route {
                        NavGroup { title:"Supplier",
                            NavItem { to:Route::SupplierWorkspace {}, icon:"overview", label:"Overview" }
                        }
                    } else {
                        NavGroup { title:"Workspace",
                            NavItem { to:Route::Overview {}, icon:"overview", label:"Overview" }
                        }
                    }
                    if !buyer_route && !supplier_route {
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
                                        if let Some(route) = search_route(&query, buyer_route) {
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
                            if !buyer_route && !supplier_route {
                                div { class:"env-chip", title:"BurnCloud server is configured; live health is shown on Overview", span { class:"green-dot" } "Server Configured" }
                                Link { to:Route::Logs {}, class:"icon-button", title:"Open request logs", Icon { name:"bell" } }
                                Link { to:Route::Settings {}, class:"icon-button", title:"System settings", Icon { name:"help" } }
                            }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn user(roles: &[&str]) -> CurrentUser {
        CurrentUser {
            id: "user-1".to_string(),
            username: "buyer".to_string(),
            roles: roles.iter().map(|role| (*role).to_string()).collect(),
        }
    }

    #[test]
    fn buyer_access_accepts_explicit_and_legacy_buyer_roles() {
        assert!(has_buyer_access(Some(&user(&["buyer"]))));
        assert!(has_buyer_access(Some(&user(&["BUYER"]))));
        assert!(has_buyer_access(Some(&user(&["user"]))));
        assert!(has_buyer_access(Some(&user(&["admin", "buyer"]))));
        assert!(!has_buyer_access(Some(&user(&["admin"]))));
        assert!(!has_buyer_access(None));
    }

    #[test]
    fn buyer_workspace_contains_only_buyer_namespaced_routes() {
        assert!(is_buyer_workspace_route(&Route::BuyerOverview {}));
        assert!(is_buyer_workspace_route(&Route::BuyerPlayground {}));
        assert!(is_buyer_workspace_route(&Route::BuyerMarketplace {}));
        assert!(is_buyer_workspace_route(&Route::BuyerAPIKeys {}));
        assert!(is_buyer_workspace_route(&Route::BuyerUsage {}));
        assert!(is_buyer_workspace_route(&Route::BuyerBilling {}));
        assert!(is_buyer_workspace_route(&Route::BuyerLogs {}));
        assert!(!is_buyer_workspace_route(&Route::APIKeys {}));
        assert!(!is_buyer_workspace_route(&Route::Billing {}));
    }

    #[test]
    fn workspace_switcher_roles_are_exact_and_case_insensitive() {
        let mixed = user(&["buyer", "SUPPLIER", "admin"]);
        assert!(has_buyer_access(Some(&mixed)));
        assert!(has_role(Some(&mixed), "supplier"));
        assert!(has_role(Some(&mixed), "ADMIN"));
        assert!(!has_role(Some(&mixed), "operator"));
    }
}
