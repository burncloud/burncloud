use dioxus::prelude::*;

use crate::{
    app::Route,
    components::{Badge, Icon, Logo},
};

#[component]
pub fn Login() -> Element {
    let navigator = use_navigator();
    let password_nav = navigator.clone();
    let passkey_nav = navigator.clone();
    let demo_nav = navigator.clone();
    let mut passkey = use_signal(|| false);
    let mut email = use_signal(|| "wei@burncloud.io".to_string());
    let mut password = use_signal(|| "••••••••••••".to_string());
    let mut remember = use_signal(|| true);
    let mut status = use_signal(String::new);

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
                        Badge { text: "TPM Hardware Attested Portal", tone: "brand" }
                        h1 { "Sign in to BurnCloud" }
                        p { "Access your silicon route Gateway console and cryptographic receipts." }
                    }

                    div { class: "card auth-card",
                        div { class: "auth-tabs",
                            button {
                                r#type: "button",
                                class: if !passkey() { "auth-tab active" } else { "auth-tab" },
                                onclick: move |_| {
                                    passkey.set(false);
                                    status.set(String::new());
                                },
                                "Password & 2FA"
                            }
                            button {
                                r#type: "button",
                                class: if passkey() { "auth-tab active" } else { "auth-tab" },
                                onclick: move |_| {
                                    passkey.set(true);
                                    status.set(String::new());
                                },
                                "◉ Passkey / Enclave"
                            }
                        }

                        if !passkey() {
                            div { class: "auth-form",
                                div { class: "field",
                                    label { "Work Email Address" }
                                    div { class: "auth-input-wrap",
                                        span { class: "auth-input-icon", "@" }
                                        input {
                                            class: "input auth-input-with-icon",
                                            r#type: "email",
                                            required: true,
                                            value: "{email}",
                                            placeholder: "name@company.com",
                                            oninput: move |evt| email.set(evt.value()),
                                        }
                                    }
                                }

                                div { class: "field",
                                    div { class: "row between",
                                        label { "Password" }
                                        button {
                                            r#type: "button",
                                            class: "button button-ghost button-sm",
                                            onclick: move |_| {
                                                status.set("A password reset token has been issued to your registered hardware key.".to_string());
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
                                            placeholder: "••••••••••••",
                                            oninput: move |evt| password.set(evt.value()),
                                        }
                                    }
                                }

                                div { class: "check-row",
                                    label { class: "row gap-2",
                                        input {
                                            r#type: "checkbox",
                                            checked: remember(),
                                            onclick: move |_| remember.set(!remember()),
                                        }
                                        "Remember this session"
                                    }
                                    span { class: "mono tiny subtle", "256-bit TLS" }
                                }

                                if !status().is_empty() {
                                    div { class: "terminal auth-status", "{status}" }
                                }

                                button {
                                    r#type: "button",
                                    class: "button button-primary",
                                    style: "width:100%",
                                    onclick: move |_| {
                                        if email().trim().is_empty() || password().trim().is_empty() {
                                            status.set("Email and password are required.".to_string());
                                        } else {
                                            password_nav.push(Route::Overview {});
                                        }
                                    },
                                    "Sign in to Console →"
                                }
                            }
                        } else {
                            div { class: "auth-form",
                                div { style: "text-align:center",
                                    div { class: "metric-icon tone-purple", style: "margin:0 auto;width:64px;height:64px",
                                        Icon { name: "lock" }
                                    }
                                    h3 { "Touch ID / YubiKey Authentication" }
                                    p { class: "small muted", "Authenticate using your physical hardware key or biometric enclave bound to your account." }
                                }
                                if !status().is_empty() {
                                    div { class: "terminal auth-status", "{status}" }
                                }
                                button {
                                    r#type: "button",
                                    class: "button button-primary",
                                    style: "width:100%",
                                    onclick: move |_| {
                                        passkey_nav.push(Route::Overview {});
                                    },
                                    "◉ Prompt Passkey Challenge"
                                }
                            }
                        }

                        div { class: "auth-demo",
                            span { class: "section-label", "Or Quick Test Drive" }
                            button {
                                r#type: "button",
                                class: "button button-secondary",
                                style: "width:100%;margin-top:12px",
                                onclick: move |_| {
                                    demo_nav.push(Route::Overview {});
                                },
                                "✨ One-Click Instant Demo Login"
                            }
                        }
                    }

                    p { class: "tiny subtle mono", style: "text-align:center", "BurnCloud Security Enclave • Hardware Proof Protocol v2.4" }
                }
            }

            AuthFooter { alternate: "register" }
        }
    }
}

#[component]
pub fn Register() -> Element {
    let navigator = use_navigator();
    let submit_nav = navigator.clone();
    let demo_nav = navigator.clone();
    let mut tier = use_signal(|| 1usize);
    let mut full_name = use_signal(|| "Wei Huang".to_string());
    let mut company = use_signal(|| "BurnCloud AI Labs".to_string());
    let mut email = use_signal(|| "wei@burncloud.io".to_string());
    let mut password = use_signal(|| "••••••••••••".to_string());
    let mut terms = use_signal(|| true);
    let mut status = use_signal(String::new);

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
                        Badge { text: "Includes $5 Free Token Credits • Pay-As-You-Go", tone: "success" }
                        h1 { "Deploy Your Silicon Gateway" }
                        p { "Get instant access to hardware-bound LLM routing, smart fallbacks, and verifiable cryptographic receipts." }
                    }

                    div { class: "card auth-card",
                        div { class: "auth-form",
                            div { class: "field",
                                label { "Select Initial Account Type" }
                                div { class: "tier-grid",
                                    TierButton { index: 0, current: tier(), name: "Free Sandbox", price: "$0 Free", detail: "2M Test Tokens", on_select: move |index| tier.set(index) }
                                    TierButton { index: 1, current: tier(), name: "Pay-As-You-Go", price: "Pay Per Token", detail: "$5 Free Credits", popular: true, on_select: move |index| tier.set(index) }
                                    TierButton { index: 2, current: tier(), name: "Enterprise", price: "Volume Rate", detail: "Post-paid Invoice", on_select: move |index| tier.set(index) }
                                }
                            }

                            div { class: "auth-form-grid",
                                div { class: "field",
                                    label { "Full Name" }
                                    input {
                                        class: "input",
                                        r#type: "text",
                                        required: true,
                                        value: "{full_name}",
                                        placeholder: "Jane Doe",
                                        oninput: move |evt| full_name.set(evt.value()),
                                    }
                                }
                                div { class: "field",
                                    label { "Company / Team Name" }
                                    input {
                                        class: "input",
                                        r#type: "text",
                                        required: true,
                                        value: "{company}",
                                        placeholder: "Acme Corp",
                                        oninput: move |evt| company.set(evt.value()),
                                    }
                                }
                            }

                            div { class: "field",
                                label { "Work Email Address" }
                                input {
                                    class: "input",
                                    r#type: "email",
                                    required: true,
                                    value: "{email}",
                                    placeholder: "name@company.com",
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
                                    oninput: move |evt| password.set(evt.value()),
                                }
                            }

                            label { class: "small row gap-2", style: "align-items:flex-start",
                                input {
                                    r#type: "checkbox",
                                    checked: terms(),
                                    onclick: move |_| terms.set(!terms()),
                                }
                                span { "I agree to BurnCloud's Terms of Service and Privacy Policy, including hardware attestation logging." }
                            }

                            if !status().is_empty() {
                                div { class: "terminal auth-status", "{status}" }
                            }

                            button {
                                r#type: "button",
                                class: "button button-primary button-lg",
                                style: "width:100%",
                                onclick: move |_| {
                                    if !terms() {
                                        status.set("Please accept the Terms of Service to proceed.".to_string());
                                    } else if full_name().trim().is_empty()
                                        || company().trim().is_empty()
                                        || email().trim().is_empty()
                                        || password().trim().is_empty()
                                    {
                                        status.set("Complete all required account fields.".to_string());
                                    } else {
                                        submit_nav.push(Route::Overview {});
                                    }
                                },
                                "Create Account & Open Console →"
                            }
                        }

                        div { class: "auth-demo",
                            button {
                                r#type: "button",
                                class: "button button-secondary",
                                style: "width:100%",
                                onclick: move |_| {
                                    demo_nav.push(Route::Overview {});
                                },
                                "⚡ Instant Demo Registration"
                            }
                        }
                    }

                    p { class: "tiny subtle mono", style: "text-align:center", "BurnCloud Gateway • Silicon Attested Multi-Cloud Infrastructure" }
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
