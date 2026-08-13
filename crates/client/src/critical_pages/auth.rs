use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{use_auth, AuthService, ClientState, CurrentUser},
    components::{Badge, Logo},
};

#[component]
pub fn Login() -> Element {
    let navigator = use_navigator();
    let auth = use_auth();
    let was_authenticated = auth.is_authenticated();
    let last_username = ClientState::load().last_username.unwrap_or_default();

    let mut username = use_signal(move || last_username);
    let mut password = use_signal(String::new);
    let mut recovery_open = use_signal(|| false);
    let mut recovery_email = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut recovery_loading = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut is_error = use_signal(|| false);
    let mut recovery_status = use_signal(String::new);
    let mut recovery_error = use_signal(|| false);

    use_effect(move || {
        if was_authenticated {
            auth.clear();
        }
    });

    rsx! {
        div { class: "auth-page",
            header { class: "auth-header",
                Link { to: Route::Home {}, class: "brand-link",
                    Logo {}
                    span { class: "brand-name", "BurnCloud" }
                }
                div { class: "auth-header-note",
                    span { "Don't have an account?" }
                    Link { to: Route::Register {}, class: "strong", "Create account" }
                }
            }

            main { class: "auth-main",
                div { class: "auth-wrap",
                    div { class: "auth-intro",
                        Badge { text: "BurnCloud Console", tone: "brand" }
                        h1 { "Sign in" }
                        p { "Manage providers, traffic, customers, access, and billing in this BurnCloud environment." }
                    }

                    div { class: "card auth-card",
                        form {
                            class: "auth-form",
                            onsubmit: move |event| {
                                event.prevent_default();
                                let user_name = username().trim().to_string();
                                let user_password = password();
                                if user_name.is_empty() || user_password.is_empty() {
                                    is_error.set(true);
                                    status.set("Username and password are required.".to_string());
                                    return;
                                }
                                loading.set(true);
                                is_error.set(false);
                                status.set("Signing in…".to_string());
                                let nav = navigator.clone();
                                spawn(async move {
                                    match AuthService::login(&user_name, &user_password).await {
                                        Ok(response) => {
                                            let user = CurrentUser {
                                                id: response.id,
                                                username: response.username,
                                                roles: response.roles,
                                            };
                                            auth.set(response.token, user, true);
                                            loading.set(false);
                                            status.set(String::new());
                                            nav.replace(Route::Overview {});
                                        }
                                        Err(error) => {
                                            loading.set(false);
                                            is_error.set(true);
                                            status.set(format!("Sign in failed: {error}"));
                                        }
                                    }
                                });
                            },
                            div { class: "field",
                                label { "Username" }
                                input {
                                    class: "input",
                                    r#type: "text",
                                    autocomplete: "username",
                                    required: true,
                                    value: "{username}",
                                    placeholder: "Your BurnCloud username",
                                    disabled: loading(),
                                    oninput: move |event| username.set(event.value()),
                                }
                            }

                            div { class: "field",
                                div { class: "row between",
                                    label { "Password" }
                                    button {
                                        r#type: "button",
                                        class: "button button-ghost button-sm",
                                        disabled: loading(),
                                        onclick: move |_| {
                                            recovery_open.set(!recovery_open());
                                            recovery_status.set(String::new());
                                            recovery_error.set(false);
                                        },
                                        "Forgot password?"
                                    }
                                }
                                input {
                                    class: "input",
                                    r#type: "password",
                                    autocomplete: "current-password",
                                    required: true,
                                    value: "{password}",
                                    placeholder: "Enter your password",
                                    disabled: loading(),
                                    oninput: move |event| password.set(event.value()),
                                }
                            }

                            if !status().is_empty() {
                                div { class: if is_error() { "terminal auth-status auth-status-error" } else { "terminal auth-status" }, "{status}" }
                            }

                            button {
                                r#type: "submit",
                                class: "button button-primary button-lg",
                                style: "width:100%",
                                disabled: loading(),
                                if loading() { "Signing in…" } else { "Sign in to Console" }
                            }
                        }

                        if recovery_open() {
                            div { class: "form-section", style: "margin-top:18px",
                                div { class: "form-section-head",
                                    strong { "Password recovery" }
                                    small { "Enter the email stored on your BurnCloud account. Recovery is separate from the username you use to sign in." }
                                }
                                div { class: "field",
                                    label { "Account email" }
                                    input {
                                        class: "input",
                                        r#type: "email",
                                        autocomplete: "email",
                                        value: "{recovery_email}",
                                        placeholder: "name@company.com",
                                        disabled: recovery_loading(),
                                        oninput: move |event| recovery_email.set(event.value()),
                                    }
                                }
                                if !recovery_status().is_empty() {
                                    div { class: if recovery_error() { "terminal auth-status auth-status-error" } else { "terminal auth-status" }, "{recovery_status}" }
                                }
                                button {
                                    r#type: "button",
                                    class: "button button-secondary",
                                    disabled: recovery_loading(),
                                    onclick: move |_| {
                                        let email = recovery_email().trim().to_string();
                                        if email.is_empty() || !email.contains('@') {
                                            recovery_error.set(true);
                                            recovery_status.set("Enter the email address attached to the account.".to_string());
                                            return;
                                        }
                                        recovery_loading.set(true);
                                        recovery_error.set(false);
                                        recovery_status.set("Requesting password recovery…".to_string());
                                        spawn(async move {
                                            match AuthService::forgot_password(&email).await {
                                                Ok(()) => {
                                                    recovery_loading.set(false);
                                                    recovery_error.set(false);
                                                    recovery_status.set("Password recovery request accepted.".to_string());
                                                }
                                                Err(error) => {
                                                    recovery_loading.set(false);
                                                    recovery_error.set(true);
                                                    recovery_status.set(format!("Password recovery failed: {error}"));
                                                }
                                            }
                                        });
                                    },
                                    if recovery_loading() { "Requesting…" } else { "Request Password Recovery" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn Register() -> Element {
    let navigator = use_navigator();
    let auth = use_auth();
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut is_error = use_signal(|| false);

    rsx! {
        div { class: "auth-page",
            header { class: "auth-header",
                Link { to: Route::Home {}, class: "brand-link",
                    Logo {}
                    span { class: "brand-name", "BurnCloud" }
                }
                div { class: "auth-header-note",
                    span { "Already have an account?" }
                    Link { to: Route::Login {}, class: "strong", "Sign in" }
                }
            }

            main { class: "auth-main",
                div { class: "auth-wrap",
                    div { class: "auth-intro",
                        Badge { text: "BurnCloud Account", tone: "success" }
                        h1 { "Create your account" }
                        p { "Create an account for this BurnCloud environment, then complete provider and API access setup from the console." }
                    }

                    div { class: "card auth-card",
                        form {
                            class: "auth-form",
                            onsubmit: move |event| {
                                event.prevent_default();
                                let user_name = username().trim().to_string();
                                let account_email = email().trim().to_string();
                                let user_password = password();
                                if user_name.is_empty() {
                                    is_error.set(true);
                                    status.set("Username is required.".to_string());
                                    return;
                                }
                                if user_password.len() < 8 {
                                    is_error.set(true);
                                    status.set("Password must contain at least 8 characters.".to_string());
                                    return;
                                }
                                loading.set(true);
                                is_error.set(false);
                                status.set("Creating your BurnCloud account…".to_string());
                                let nav = navigator.clone();
                                spawn(async move {
                                    let email_arg = if account_email.is_empty() { None } else { Some(account_email.as_str()) };
                                    match AuthService::register(&user_name, &user_password, email_arg).await {
                                        Ok(response) => {
                                            let user = CurrentUser {
                                                id: response.id,
                                                username: response.username,
                                                roles: response.roles,
                                            };
                                            auth.set(response.token, user, true);
                                            loading.set(false);
                                            status.set(String::new());
                                            nav.replace(Route::Overview {});
                                        }
                                        Err(error) => {
                                            loading.set(false);
                                            is_error.set(true);
                                            status.set(format!("Account creation failed: {error}"));
                                        }
                                    }
                                });
                            },
                            div { class: "field",
                                label { "Username" }
                                input {
                                    class: "input",
                                    r#type: "text",
                                    autocomplete: "username",
                                    required: true,
                                    value: "{username}",
                                    placeholder: "Choose a BurnCloud username",
                                    disabled: loading(),
                                    oninput: move |event| username.set(event.value()),
                                }
                            }

                            div { class: "field",
                                label { "Email" }
                                input {
                                    class: "input",
                                    r#type: "email",
                                    autocomplete: "email",
                                    value: "{email}",
                                    placeholder: "name@company.com",
                                    disabled: loading(),
                                    oninput: move |event| email.set(event.value()),
                                }
                                span { class: "tiny subtle", "Recommended so password recovery can identify this account." }
                            }

                            div { class: "field",
                                label { "Password" }
                                input {
                                    class: "input",
                                    r#type: "password",
                                    autocomplete: "new-password",
                                    required: true,
                                    value: "{password}",
                                    placeholder: "At least 8 characters",
                                    disabled: loading(),
                                    oninput: move |event| password.set(event.value()),
                                }
                            }

                            if !status().is_empty() {
                                div { class: if is_error() { "terminal auth-status auth-status-error" } else { "terminal auth-status" }, "{status}" }
                            }

                            button {
                                r#type: "submit",
                                class: "button button-primary button-lg",
                                style: "width:100%",
                                disabled: loading(),
                                if loading() { "Creating account…" } else { "Create Account" }
                            }
                        }
                    }
                }
            }
        }
    }
}
