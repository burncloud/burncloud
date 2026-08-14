use dioxus::prelude::*;
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::{
    app::Route,
    backend::{server_root, use_auth, AuthData, AuthService, ClientState, CurrentUser},
    components::{Badge, Logo},
};

#[derive(Debug, Clone, Deserialize)]
struct SetupStatus {
    setup_required: bool,
    setup_code_required: bool,
    public_registration_open: bool,
}

#[derive(Debug, Deserialize)]
struct ApiEnvelope<T> {
    success: bool,
    data: Option<T>,
    message: Option<String>,
}

async fn decode_public_envelope<T: DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, String> {
    let status = response.status();
    let text = response.text().await.map_err(|error| error.to_string())?;
    let envelope: ApiEnvelope<T> = serde_json::from_str(&text)
        .map_err(|error| format!("Invalid API response ({status}): {error}; body={text}"))?;
    if status.is_success() && envelope.success {
        envelope
            .data
            .ok_or_else(|| "API response did not include data".to_string())
    } else {
        Err(envelope
            .message
            .unwrap_or_else(|| format!("API request failed: {status}")))
    }
}

async fn load_setup_status() -> Result<SetupStatus, String> {
    let response = reqwest::Client::new()
        .get(format!("{}/api/auth/setup", server_root()))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    decode_public_envelope(response).await
}

async fn register_account(
    username: &str,
    password: &str,
    email: Option<&str>,
    setup_code: Option<&str>,
) -> Result<AuthData, String> {
    let response = reqwest::Client::new()
        .post(format!("{}/api/auth/register", server_root()))
        .json(&serde_json::json!({
            "username": username,
            "password": password,
            "email": email,
            "bootstrap_token": setup_code,
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?;
    decode_public_envelope(response).await
}

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
                        p { "Use your BurnCloud username and password to access the connected environment." }
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
                                                    recovery_status.set("Password recovery request accepted by the BurnCloud server.".to_string());
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

                    p { class: "tiny subtle", style: "text-align:center", "BurnCloud stores the authenticated session locally so console API calls can use your JWT." }
                }
            }

            AuthFooter { alternate: "register" }
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
    let mut setup_code = use_signal(String::new);
    let mut terms = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut is_error = use_signal(|| false);
    let mut setup = use_signal(|| None::<SetupStatus>);
    let mut setup_loading = use_signal(|| true);
    let mut setup_error = use_signal(String::new);

    use_effect(move || {
        spawn(async move {
            match load_setup_status().await {
                Ok(value) => setup.set(Some(value)),
                Err(error) => setup_error.set(error),
            }
            setup_loading.set(false);
        });
    });

    let setup_snapshot = setup();
    let first_admin = setup_snapshot
        .as_ref()
        .map(|value| value.setup_required)
        .unwrap_or(false);
    let setup_code_required = setup_snapshot
        .as_ref()
        .map(|value| value.setup_code_required)
        .unwrap_or(false);
    let public_registration_open = setup_snapshot
        .as_ref()
        .map(|value| value.public_registration_open)
        .unwrap_or(false);
    let registration_available = first_admin || public_registration_open;

    rsx! {
        div { class: "auth-page",
            header { class: "auth-header",
                Link { to: Route::Home {}, class: "brand-link",
                    Logo {}
                    span { class: "brand-name", "BurnCloud" }
                }
                if !first_admin {
                    div { class: "auth-header-note",
                        span { "Already have an account?" }
                        Link { to: Route::Login {}, class: "strong", "Sign in" }
                    }
                } else {
                    div { class: "auth-header-note",
                        span { "First-time setup" }
                    }
                }
            }

            main { class: "auth-main",
                div { class: "auth-wrap",
                    if setup_loading() {
                        div { class: "card auth-card", style: "text-align:center",
                            strong { "Preparing BurnCloud…" }
                            p { class: "small muted", "Checking first-time setup state." }
                        }
                    } else if !setup_error().is_empty() {
                        div { class: "card auth-card",
                            Badge { text: "Setup unavailable", tone: "danger" }
                            h1 { "Cannot load setup state" }
                            p { "BurnCloud could not verify whether this environment has already been initialized." }
                            div { class: "terminal auth-status auth-status-error", "{setup_error}" }
                            Link { to: Route::Login {}, class: "button button-secondary", "Back to sign in" }
                        }
                    } else if !registration_available {
                        div { class: "card auth-card",
                            Badge { text: "Registration closed", tone: "brand" }
                            h1 { "BurnCloud is already initialized" }
                            p { "Public account creation is disabled on this environment. Ask an administrator to create an account for you." }
                            Link { to: Route::Login {}, class: "button button-primary", "Sign in" }
                        }
                    } else {
                        div { class: "auth-intro",
                            if first_admin {
                                Badge { text: "First-time Setup", tone: "success" }
                                h1 { "Create administrator" }
                                if setup_code_required {
                                    p { "Create the first administrator. Because this server is exposed beyond localhost, enter the one-time setup code BurnCloud printed at startup." }
                                } else {
                                    p { "Create the first administrator and start using BurnCloud. No setup code or environment configuration is required." }
                                }
                            } else {
                                Badge { text: "BurnCloud Account", tone: "success" }
                                h1 { "Create your account" }
                                p { "Create the identity you will use to sign in to this BurnCloud environment." }
                            }
                        }

                        div { class: "card auth-card",
                            form {
                                class: "auth-form",
                                onsubmit: move |event| {
                                    event.prevent_default();
                                    let user_name = username().trim().to_string();
                                    let account_email = email().trim().to_string();
                                    let user_password = password();
                                    let one_time_code = setup_code().trim().to_string();
                                    let current_setup = setup();
                                    let creating_first_admin = current_setup
                                        .as_ref()
                                        .map(|value| value.setup_required)
                                        .unwrap_or(false);
                                    let code_required = current_setup
                                        .as_ref()
                                        .map(|value| value.setup_code_required)
                                        .unwrap_or(false);

                                    if !creating_first_admin && !terms() {
                                        is_error.set(true);
                                        status.set("Accept the Terms of Service and Privacy Policy to continue.".to_string());
                                        return;
                                    }
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
                                    if code_required && one_time_code.is_empty() {
                                        is_error.set(true);
                                        status.set("Enter the one-time setup code shown by BurnCloud at startup.".to_string());
                                        return;
                                    }

                                    loading.set(true);
                                    is_error.set(false);
                                    status.set(if creating_first_admin {
                                        "Creating administrator…".to_string()
                                    } else {
                                        "Creating your BurnCloud account…".to_string()
                                    });
                                    let nav = navigator.clone();
                                    spawn(async move {
                                        let email_arg = if account_email.is_empty() {
                                            None
                                        } else {
                                            Some(account_email.as_str())
                                        };
                                        let code_arg = if code_required {
                                            Some(one_time_code.as_str())
                                        } else {
                                            None
                                        };
                                        match register_account(
                                            &user_name,
                                            &user_password,
                                            email_arg,
                                            code_arg,
                                        )
                                        .await
                                        {
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
                                        placeholder: if first_admin { "Choose the administrator username" } else { "Choose a BurnCloud username" },
                                        disabled: loading(),
                                        oninput: move |event| username.set(event.value()),
                                    }
                                    if first_admin {
                                        span { class: "tiny subtle", "This account will become the first BurnCloud administrator." }
                                    } else {
                                        span { class: "tiny subtle", "This is the identity used on the Sign In page." }
                                    }
                                }

                                div { class: "field",
                                    label { "Email (recommended)" }
                                    input {
                                        class: "input",
                                        r#type: "email",
                                        autocomplete: "email",
                                        value: "{email}",
                                        placeholder: "name@company.com",
                                        disabled: loading(),
                                        oninput: move |event| email.set(event.value()),
                                    }
                                    span { class: "tiny subtle", "Add an email if you want to use the server's password-recovery flow." }
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

                                if setup_code_required {
                                    div { class: "field",
                                        label { "One-time setup code" }
                                        input {
                                            class: "input",
                                            r#type: "password",
                                            autocomplete: "one-time-code",
                                            required: true,
                                            value: "{setup_code}",
                                            placeholder: "Paste the code printed by BurnCloud",
                                            disabled: loading(),
                                            oninput: move |event| setup_code.set(event.value()),
                                        }
                                        span { class: "tiny subtle", "This field appears only because BurnCloud is bound to a non-local address. The code stops working after the administrator is created." }
                                    }
                                }

                                if !first_admin {
                                    label { class: "small row gap-2", style: "align-items:flex-start",
                                        input {
                                            r#type: "checkbox",
                                            checked: terms(),
                                            disabled: loading(),
                                            onchange: move |_| terms.set(!terms()),
                                        }
                                        span { "I agree to BurnCloud's Terms of Service and Privacy Policy." }
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
                                    if loading() {
                                        if first_admin { "Creating administrator…" } else { "Creating account…" }
                                    } else if first_admin {
                                        "Create Administrator & Start"
                                    } else {
                                        "Create Account"
                                    }
                                }
                            }
                        }

                        if first_admin {
                            div { class: "product-note", "BurnCloud permanently closes first-admin setup after this account is created. You can manage additional users from the Console." }
                        } else {
                            div { class: "product-note", "Registration shows only fields the current BurnCloud backend can persist. Billing plans, company metadata, and passkeys appear only when corresponding server capabilities exist." }
                        }
                    }
                }
            }

            if !first_admin {
                AuthFooter { alternate: "login" }
            }
        }
    }
}

#[component]
fn AuthFooter(alternate: &'static str) -> Element {
    rsx! {
        footer { class: "public-footer",
            div { class: "row", style: "justify-content:center",
                Link { to: Route::Home {}, "Home" }
                span { "•" }
                if alternate == "register" {
                    Link { to: Route::Register {}, "Register" }
                } else {
                    Link { to: Route::Login {}, "Sign In" }
                }
                span { "•" }
                a { href: "#privacy", "Privacy Policy" }
                span { "•" }
                a { href: "#terms", "Terms of Service" }
            }
        }
    }
}
