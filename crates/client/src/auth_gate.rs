use dioxus::prelude::*;

use crate::{app::Route, backend::use_auth};

#[component]
pub fn AuthGate() -> Element {
    let auth = use_auth();
    let navigator = use_navigator();

    if !auth.is_authenticated() {
        use_effect(move || {
            navigator.replace(Route::Login {});
        });
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

    rsx! { Outlet::<Route> {} }
}
