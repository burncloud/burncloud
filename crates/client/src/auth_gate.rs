use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::use_auth,
    functional_layout::FunctionalConsoleLayout,
    workbench::WorkbenchLayout,
};

#[component]
pub fn AuthGate() -> Element {
    let auth = use_auth();
    let navigator = use_navigator();
    let authenticated = auth.is_authenticated();

    use_effect(move || {
        if !authenticated {
            navigator.replace(Route::Login {});
        }
    });

    let current = use_route::<Route>();

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

    if matches!(current, Route::Overview {}) {
        rsx! { WorkbenchLayout {} }
    } else {
        rsx! { FunctionalConsoleLayout {} }
    }
}
