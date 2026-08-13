use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::use_auth,
    customer_layout::CustomerConsoleLayout,
    functional_layout::FunctionalConsoleLayout,
    role_access::is_staff_roles,
};

#[component]
pub fn AuthGate() -> Element {
    let auth = use_auth();
    let navigator = use_navigator();
    let current = use_route::<Route>();
    let authenticated = auth.is_authenticated();
    let user = auth.user();
    let staff = user.as_ref().is_some_and(|value| is_staff_roles(&value.roles));
    let customer_allowed = matches!(current, Route::Billing {});

    use_effect(move || {
        if !authenticated {
            navigator.replace(Route::Login {});
        } else if !staff && !customer_allowed {
            navigator.replace(Route::Billing {});
        }
    });

    if !authenticated {
        return rsx! {
            div { class: "auth-page",
                main { class: "auth-main",
                    div { class: "card card-pad stack", style: "max-width:420px;margin:auto;text-align:center",
                        strong { "Authentication required" }
                        span { class: "small muted", "Redirecting to BurnCloud sign in…" }
                    }
                }
            }
        };
    }

    if staff {
        return rsx! { FunctionalConsoleLayout {} };
    }

    if !customer_allowed {
        return rsx! {
            div { class: "auth-page",
                main { class: "auth-main",
                    div { class: "card card-pad stack", style: "max-width:520px;margin:auto;text-align:center",
                        strong { "Opening your account view" }
                        span { class: "small muted", "This session does not have an operator role. Redirecting to account-scoped billing…" }
                    }
                }
            }
        };
    }

    rsx! { CustomerConsoleLayout {} }
}
