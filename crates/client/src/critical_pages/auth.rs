use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{use_auth, AuthService, ClientState, CurrentUser},
    components::{Badge, Icon, Logo},
};

#[component]
pub fn Login() -> Element {
    let navigator = use_navigator();
    let auth = use_auth();
    let was_authenticated = auth.is_authenticated();
    let last_username = ClientState::load().last_username.unwrap_or_default();

    let mut username = use_signal(move || last_username);
    let mut password = use_signal(String::new);
    let mut passkey = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut status = use_signal(String::new);
    let mut is_error = use_signal(|| false);

    use_effect(move || {
        if was_authenticated {
            auth.clear();
        }
    });

    let submit_nav = navigator.clone();
    let forgot_username = username;
    let mut forgot_status = status;
    let mut forgot_error = is_error;

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
                        Badge { text: "BurnCloud Secure Console", tone: "brand" }
                        h1 { "Sign in to BurnCloud" }
                        p { "Authenticate against the local BurnCloud server and resume your protected console session." }
                    }

                    div { class: "card auth-card",
                        div { class: "auth-tabs",
                            button {
                                r#type: "button",
                                class: if !passkey() { "auth-tab active" } else { "auth-tab" },
                                onclick: move |_| {
                                    passkey.set(false);
                                    status.set(String::new());
                                    is_error.set(false);
                                },
                                "Password"
                            }
                            button {
                                r#type: "button",
                                class: if passkey() { "auth-tab active" } else { "auth-tab" },
                                onclick: move |_| {
                                    passkey.set(true);
                                    status.set("Passkey authentication is not exposed by the current BurnCloud server API yet.".to_string());
                                    is_error.set(false);
                                },
                                "◉ Passkey"
                            }
                        }

                        if !passkey() {
                            div { class: "auth-form",
                                div { class: "field",
                                    label { "Username" }
                                    div { class: "auth-input-wrap",
                                        span { class: "auth-input-icon", "@" }
                                        input {
                                            class: "input auth-input-with-icon",
                                            r#type: "text",
                                            required: true,
                                            value: "{username}",
                                            placeholder: "your BurnCloud username",
                                            disabled: loading(),
                                            oninput: move |evt| username.set(evt.value()),
                                        }
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
                                                let account = forgot_username().trim().to_string();
                                                if account.is_empty() || !account.contains('@') {
                                                    forgot_error.set(true);
                                                    forgot_status.set("Password reset requires the account email address. Enter the email in Username if your account uses email as its username, or use the registered email from the recovery flow.".to_string());
                                                    return;
                                                }
                                                forgot_error.set(false);
                                                forgot_status.set("Requesting password reset…".to_string());
                                                spawn(async move {
                                                    match AuthService::forgot_password(&account).await {
                                                        Ok(()) => {
                                                            forgot_error.set(false);
                                                            forgot_status.set("Password reset request accepted by the BurnCloud server.".to_string());
                                                        }
                                                        Err(error) => {
                                                            forgot_error.set(true);
                                                            forgot_status.set(format!("Password reset failed: {error}"));
                                                        }
                                                    }
                                                });
                                            },
                                            "Forgot?"
                                        }
                                    }
                                    div { class: "auth-input-wrap",
                                        span { class: "auth-input-icon", "•" }
                                        input {
                                            class: "input auth-input-with-icon",
                                            r#type: "password",
                                            required: true,
                                            value: "{password}",
                                            placeholder: "Enter password",
                                            disabled: loading(),
                                            oninput: move |evt| password.set(evt.value()),
                                            onkeydown: move |evt| {
                                                if evt.key() == Key::Enter && !loading() {
                                                    let u = username().trim().to_string();
                                                    let p = password();
                                                    if u.is_empty() || p.is_empty() {
                                                        is_error.set(true);
                                                        status.set("Username and password are required.".to_string());
                                                        return;
                                                    }
                                                    loading.set(true);
                                                    is_error.set(false);
                                                    status.set("Authenticating with BurnCloud…".to_string());
                                                    let nav = submit_nav.clone();
                                                    spawn(async move {
                                                        match AuthService::login(&u, &p).await {
                                                            Ok(response) => {
                                                                let user = CurrentUser { id: response.id, username: response.username, roles: response.roles };
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
                                                }
                                            },
                                        }
                                    }
                                }

                                if !status().is_empty() {
                                    div { class: if is_error() { "terminal auth-status auth-status-error" } else { "terminal auth-status" }, "{status}" }
                                }

                                button {
                                    r#type: "button",
                                    class: "button button-primary",
                                    style: "width:100%",
                                    disabled: loading(),
                                    onclick: move |_| {
                                        let u = username().trim().to_string();
                                        let p = password();
                                        if u.is_empty() || p.is_empty() {
                                            is_error.set(true);
                                            status.set("Username and password are required.".to_string());
                                            return;
                                        }
                                        loading.set(true);
                                        is_error.set(false);
                                        status.set("Authenticating with BurnCloud…".to_string());
                                        let nav = navigator.clone();
                                        spawn(async move {
                                            match AuthService::login(&u, &p).await {
                                                Ok(response) => {
                                                    let user = CurrentUser { id: response.id, username: response.username, roles: response.roles };
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
                                    if loading() { "Signing in…" } else { "Sign in to Console →" }
                                }
                            }
                        } else {
                            div { class: "auth-form",
                                div { style: "text-align:center",
                                    div { class: "metric-icon tone-purple", style: "margin:0 auto;width:64px;height:64px",
                                        Icon { name: "lock" }
                                    }
                                    h3 { "Passkey authentication" }
                                    p { class: "small muted", "The current BurnCloud backend exposes password/JWT authentication, OAuth URL generation and password recovery, but no passkey challenge endpoint." }
                                }
                                div { class: "terminal auth-status", "Backend support required before this control can be enabled." }
                                button { r#type: "button", class: "button button-secondary", style: "width:100%", disabled: true, "Passkey challenge unavailable" }
                            }
                        }
                    }

                    p { class: "tiny subtle mono", style: "text-align:center", "Session token is stored locally for authenticated console API calls." }
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
    let mut tier = use_signal(|| 1usize);
    let mut username = use_signal(String::new);
    let mut email = use_signal(String::new);
    let mut password = use_signal(String::new);
    let mut company = use_signal(String::new);
    let mut terms = use_signal(|| false);
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
                div { class: "auth-wrap register",
                    div { class: "auth-intro",
                        Badge { text: "Create a real BurnCloud account", tone: "success" }
                        h1 { "Open Your Gateway Console" }
                        p { "This form now creates the account through /api/auth/register and uses the returned JWT immediately." }
                    }

                    div { class: "card auth-card",
                        div { class: "auth-form",
                            div { class: "field",
                                label { "Onboarding Account Preference" }
                                p { class: "tiny subtle", "Visual preference only: the current backend does not persist billing tiers during registration." }
                                div { class: "tier-grid",
                                    TierButton { index: 0, current: tier(), name: "Free Sandbox", price: "$0 Free", detail: "Onboarding preference", on_select: move |index| tier.set(index) }
                                    TierButton { index: 1, current: tier(), name: "Pay-As-You-Go", price: "Usage Based", detail: "Onboarding preference", popular: true, on_select: move |index| tier.set(index) }
                                    TierButton { index: 2, current: tier(), name: "Enterprise", price: "Volume", detail: "Onboarding preference", on_select: move |index| tier.set(index) }
                                }
                            }

                            div { class: "auth-form-grid",
                                div { class: "field",
                                    label { "Username" }
                                    input {
                                        class: "input",
                                        r#type: "text",
                                        required: true,
                                        value: "{username}",
                                        placeholder: "burncloud-admin",
                                        disabled: loading(),
                                        oninput: move |evt| username.set(evt.value()),
                                    }
                                }
                                div { class: "field",
                                    label { "Company / Team" }
                                    input {
                                        class: "input",
                                        r#type: "text",
                                        value: "{company}",
                                        placeholder: "Optional UI note",
                                        disabled: loading(),
                                        oninput: move |evt| company.set(evt.value()),
                                    }
                                    span { class: "tiny subtle", "Not persisted by the current registration API." }
                                }
                            }

                            div { class: "field",
                                label { "Email" }
                                input {
                                    class: "input",
                                    r#type: "email",
                                    value: "{email}",
                                    placeholder: "name@company.com",
                                    disabled: loading(),
                                    oninput: move |evt| email.set(evt.value()),
                                }
                            }

                            div { class: "field",
                                label { "Password" }
                                input {
                                    class: "input",
                                    r#type: "password",
                                    required: true,
                                    value: "{password}",
                                    placeholder: "At least 8 characters",
                                    disabled: loading(),
                                    oninput: move |evt| password.set(evt.value()),
                                }
                            }

                            label { class: "small row gap-2", style: "align-items:flex-start",
                                input {
                                    r#type: "checkbox",
                                    checked: terms(),
                                    disabled: loading(),
                                    onclick: move |_| terms.set(!terms()),
                                }
                                span { "I agree to BurnCloud's Terms of Service and Privacy Policy." }
                            }

                            if !status().is_empty() {
                                div { class: if is_error() { "terminal auth-status auth-status-error" } else { "terminal auth-status" }, "{status}" }
                            }

                            button {
                                r#type: "button",
                                class: "button button-primary button-lg",
                                style: "width:100%",
                                disabled: loading(),
                                onclick: move |_| {
                                    let u = username().trim().to_string();
                                    let e = email().trim().to_string();
                                    let p = password();
                                    if !terms() {
                                        is_error.set(true);
                                        status.set("Please accept the Terms of Service to proceed.".to_string());
                                        return;
                                    }
                                    if u.is_empty() || p.is_empty() {
                                        is_error.set(true);
                                        status.set("Username and password are required.".to_string());
                                        return;
                                    }
                                    if p.len() < 8 {
                                        is_error.set(true);
                                        status.set("Password must contain at least 8 characters.".to_string());
                                        return;
                                    }
                                    loading.set(true);
                                    is_error.set(false);
                                    status.set("Creating account on BurnCloud…".to_string());
                                    let nav = navigator.clone();
                                    spawn(async move {
                                        let email_arg = if e.is_empty() { None } else { Some(e.as_str()) };
                                        match AuthService::register(&u, &p, email_arg).await {
                                            Ok(response) => {
                                                let user = CurrentUser { id: response.id, username: response.username, roles: response.roles };
                                                auth.set(response.token, user, true);
                                                loading.set(false);
                                                status.set(String::new());
                                                nav.replace(Route::Overview {});
                                            }
                                            Err(error) => {
                                                loading.set(false);
                                                is_error.set(true);
                                                status.set(format!("Registration failed: {error}"));
                                            }
                                        }
                                    });
                                },
                                if loading() { "Creating account…" } else { "Create Account & Open Console →" }
                            }
                        }
                    }

                    p { class: "tiny subtle mono", style: "text-align:center", "Registration persists only fields supported by the current BurnCloud backend." }
                }
            }

            AuthFooter { alternate: "login" }
        }
    }
}

#[component]
fn TierButton(
    index: usize,
    current: usize,
    name: &'static str,
    price: &'static str,
    detail: &'static str,
    #[props(default)] popular: bool,
    on_select: EventHandler<usize>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: if current == index { "tier active" } else { "tier" },
            onclick: move |_| on_select.call(index),
            if popular { span { class: "popular", "POPULAR" } }
            strong { "{name}" }
            span { class: "price", "{price}" }
            small { "{detail}" }
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
