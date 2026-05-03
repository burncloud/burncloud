// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use crate::app::Route;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::components::logo::Logo;
use burncloud_client_shared::i18n::{t, use_i18n};
use burncloud_client_shared::use_toast;
use burncloud_client_shared::utils::storage::ClientState;
use burncloud_client_shared::{use_auth, CurrentUser};
use dioxus::prelude::*;

#[component]
pub fn LoginPage() -> Element {
    let i18n = use_i18n();
    let lang = i18n.language;
    let state = ClientState::load();
    let last_username = state.last_username.unwrap_or_default();

    let mut email = use_signal(move || last_username);
    let mut pw = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut login_error = use_signal(|| None::<String>);
    let toast = use_toast();
    let navigator = use_navigator();
    let auth = use_auth();

    let handle_login = move |_| {
        let u = email.read().clone();
        let p = pw.read().clone();
        if u.is_empty() || p.is_empty() {
            login_error.set(Some(t(*lang.read(), "login.error.fill_required").to_string()));
            return;
        }
        loading.set(true);
        login_error.set(None);
        spawn(async move {
            match AuthService::login(&u, &p).await {
                Ok(response) => {
                    loading.set(false);
                    let new_state = ClientState {
                        last_username: Some(u.clone()),
                        auth_token: Some(response.token.clone()),
                        user_info: Some(
                            serde_json::to_string(&CurrentUser {
                                id: response.id.clone(),
                                username: response.username.clone(),
                                roles: response.roles.clone(),
                            })
                            .unwrap_or_default(),
                        ),
                        theme: None,
                    };
                    new_state.save();
                    let user = CurrentUser {
                        id: response.id,
                        username: response.username,
                        roles: response.roles,
                    };
                    auth.set_auth(response.token, user);
                    toast.success(t(*lang.read(), "login.success"));
                    navigator.push(Route::Dashboard {});
                }
                Err(e) => {
                    loading.set(false);
                    eprintln!("LoginPage: Login error: {}", e);
                    login_error.set(Some(t(*lang.read(), "login.error.invalid_credentials").to_string()));
                }
            }
        });
    };

    rsx! {
        div { class: "login",

            // --- LEFT: BRAND PANEL (50%) ---
            aside { class: "login-brand",
                // Logo + brand
                div { class: "login-brand-header",
                    Logo { class: "login-brand-logo" }
                    div {
                        div { class: "login-brand-name", "BurnCloud" }
                        div { class: "login-brand-sublabel", "Enterprise" }
                    }
                }

                // Center content
                div {
                    div { class: "login-brand-eyebrow", "The Next-Gen AI Gateway" }
                    h1 { class: "login-brand-headline",
                        "Upgrade the"
                        br {}
                        "engine."
                    }
                    p { class: "login-brand-subhead",
                        "Rust-native LLM aggregation. MB-level footprint, smart load balancing, OpenAI-compatible API. One binary, every model."
                    }
                }

                // Version
                div { class: "login-brand-version",
                    "v0.3.1 \u{00b7} build 2026.04.27 \u{00b7} burncloud.io"
                }
            }

            // --- RIGHT: FORM PANEL (50%) ---
            main { class: "login-form",
                div { class: "mb-xxxl",
                    h2 { class: "login-form-title", {t(*lang.read(), "login.form.title")} }
                    div { class: "login-form-subtitle", {t(*lang.read(), "login.form.subtitle")} }
                }

                div { class: "flex flex-col gap-xl",
                    // Email field
                    div {
                        label { class: "login-input-label", {t(*lang.read(), "login.form.email_label")} }
                        div { class: "login-input",
                            input {
                                r#type: "email",
                                placeholder: "you@burncloud.com",
                                value: "{email}",
                                oninput: move |e: Event<FormData>| email.set(e.value()),
                            }
                        }
                    }

                    // Password field
                    div {
                        div { class: "flex items-center justify-between mb-sm",
                            label { class: "login-input-label m-0", {t(*lang.read(), "login.form.password_label")} }
                            a { class: "login-forgot-link", {t(*lang.read(), "login.form.forgot_password")} }
                        }
                        div { class: "login-input",
                            input {
                                r#type: "password",
                                placeholder: "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}",
                                value: "{pw}",
                                oninput: move |e: Event<FormData>| pw.set(e.value()),
                            }
                        }
                    }

                    // Error message
                    if let Some(err) = login_error() {
                        div { class: "login-error-text", "{err}" }
                    }

                    // Login button
                    button {
                        class: "landing-btn landing-btn-dark bc-btn-block bc-btn-lg",
                        disabled: loading(),
                        onclick: handle_login,
                        if loading() {
                            {t(*lang.read(), "login.form.logging_in")}
                        } else {
                            {t(*lang.read(), "login.form.login")}
                        }
                    }

                    // Divider
                    div { class: "login-divider",
                        div { class: "login-divider-line" }
                        span { class: "login-divider-text", {t(*lang.read(), "login.form.or")} }
                        div { class: "login-divider-line" }
                    }

                    // OAuth buttons
                    div { class: "login-social-grid",
                        button { class: "landing-btn login-social-btn",
                            span { class: "login-social-letter font-bold mr-sm", "G" } " Google"
                        }
                        button { class: "landing-btn login-social-btn",
                            svg { class: "login-social-icon", width: "14", height: "14", view_box: "0 0 24 24", fill: "currentColor",
                                path { d: "M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.1.79-.25.79-.56v-2.05c-3.2.7-3.87-1.36-3.87-1.36-.52-1.33-1.27-1.69-1.27-1.69-1.04-.71.08-.69.08-.69 1.15.08 1.76 1.18 1.76 1.18 1.02 1.75 2.68 1.24 3.34.95.1-.74.4-1.24.73-1.53-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.18-3.09-.12-.29-.51-1.46.11-3.04 0 0 .96-.31 3.15 1.18.91-.25 1.89-.38 2.86-.38.97 0 1.95.13 2.86.38 2.18-1.49 3.14-1.18 3.14-1.18.62 1.58.23 2.75.11 3.04.74.8 1.18 1.83 1.18 3.09 0 4.42-2.69 5.4-5.25 5.68.41.36.78 1.05.78 2.12v3.14c0 .31.21.66.79.55C20.21 21.39 23.5 17.07 23.5 12 23.5 5.65 18.35.5 12 .5z" }
                            }
                            "GitHub"
                        }
                    }

                    // Switch link
                    div { class: "login-footer",
                        {t(*lang.read(), "login.form.no_account")}
                        Link { to: Route::RegisterPage {}, class: "login-footer-link", {t(*lang.read(), "login.form.free_register")} }
                    }
                }
            }
        }
    }
}
