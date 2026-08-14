use dioxus::prelude::*;
use serde::Deserialize;

use crate::{
    app::Route,
    backend::{server_root, use_auth},
    functional_layout::FunctionalConsoleLayout,
};

#[derive(Debug, Deserialize)]
struct SetupStatus {
    setup_required: bool,
}

#[derive(Debug, Deserialize)]
struct SetupEnvelope {
    success: bool,
    data: Option<SetupStatus>,
}

async fn first_run_setup_required() -> bool {
    let url = format!("{}/api/auth/setup", server_root());
    let Ok(response) = reqwest::Client::new().get(url).send().await else {
        return false;
    };
    let Ok(envelope) = response.json::<SetupEnvelope>().await else {
        return false;
    };
    envelope.success
        && envelope
            .data
            .map(|status| status.setup_required)
            .unwrap_or(false)
}

#[component]
pub fn AuthGate() -> Element {
    let auth = use_auth();
    let navigator = use_navigator();
    let authenticated = auth.is_authenticated();
    let mut redirect_started = use_signal(|| false);

    use_effect(move || {
        if authenticated || redirect_started() {
            return;
        }

        redirect_started.set(true);
        let nav = navigator.clone();
        spawn(async move {
            if first_run_setup_required().await {
                nav.replace(Route::Register {});
            } else {
                nav.replace(Route::Login {});
            }
        });
    });

    if !authenticated {
        return rsx! {
            div { class: "auth-page",
                main { class: "auth-main",
                    div { class: "card card-pad stack", style: "max-width:420px;margin:auto;text-align:center",
                        strong { "Starting BurnCloud" }
                        span { class: "small muted", "Checking whether this environment needs first-time administrator setup…" }
                    }
                }
            }
        };
    }

    rsx! { FunctionalConsoleLayout {} }
}
