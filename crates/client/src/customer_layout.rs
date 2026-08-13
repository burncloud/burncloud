use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::use_auth,
    components::{Icon, Logo},
};

fn initials(name: &str) -> String {
    let words: Vec<&str> = name
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_')
        .filter(|word| !word.is_empty())
        .collect();
    if words.len() >= 2 {
        let first = words[0].chars().next().unwrap_or('B');
        let second = words[1].chars().next().unwrap_or('C');
        format!("{}{}", first.to_ascii_uppercase(), second.to_ascii_uppercase())
    } else {
        name.chars().take(2).collect::<String>().to_ascii_uppercase()
    }
}

#[component]
pub fn CustomerConsoleLayout() -> Element {
    let auth = use_auth();
    let navigator = use_navigator();
    let user = auth.user();
    let username = user
        .as_ref()
        .map(|value| value.username.clone())
        .unwrap_or_else(|| "BurnCloud".to_string());
    let roles = user
        .as_ref()
        .map(|value| value.roles.join(", "))
        .unwrap_or_default();
    let avatar = initials(&username);

    rsx! {
        div { class: "console-shell",
            aside { class: "sidebar",
                div { class: "sidebar-brand",
                    Link { to: Route::Billing {}, class: "brand-link",
                        Logo {}
                        span { class: "brand-name", "BurnCloud" }
                    }
                }
                nav { class: "sidebar-nav",
                    div { class: "nav-group",
                        h4 { class: "nav-group-title", "Account" }
                        div { class: "nav-items",
                            Link { to: Route::Billing {}, class: "nav-item", active_class: "active",
                                Icon { name: "billing" }
                                span { "Billing & Usage" }
                            }
                        }
                    }
                    div { class: "product-note", style: "margin:12px",
                        "Customer access currently exposes only account-scoped billing. Provider inventory, global logs, customer administration, API-key administration, guardrails, and system settings remain operator-only in the product UI."
                    }
                    div { class: "public-links",
                        h4 { class: "nav-group-title", "External" }
                        div { class: "nav-items",
                            Link { to: Route::Home {}, class: "nav-item",
                                Icon { name: "globe" }
                                span { "Landing Page" }
                            }
                            button {
                                class: "nav-item",
                                style: "width:100%;text-align:left;border:0;background:transparent",
                                onclick: move |_| {
                                    auth.clear();
                                    navigator.replace(Route::Login {});
                                },
                                Icon { name: "logout" }
                                span { "Sign Out" }
                            }
                        }
                    }
                }
            }
            div { class: "console-main",
                header { class: "topbar",
                    h1 { class: "topbar-title", "Account Billing" }
                    div { class: "topbar-right",
                        div { class: "top-actions",
                            div { class: "profile-link", title: "Signed-in customer account",
                                div { class: "avatar", "{avatar}" }
                                div { class: "two-line",
                                    span { class: "profile-name", "{username}" }
                                    if !roles.is_empty() { small { class: "subtle", "{roles}" } }
                                }
                            }
                        }
                    }
                }
                main { class: "content-scroll", Outlet::<Route> {} }
            }
        }
    }
}
