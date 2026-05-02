// UI registration — HTTP response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::auth_context::{use_auth, CurrentUser};
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::components::logo::Logo;
use burncloud_client_shared::i18n::{t, use_i18n};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;
use std::time::Duration;

#[component]
pub fn RegisterPage() -> Element {
    let i18n = use_i18n();
    let lang = i18n.language;
    let mut email = use_signal(String::new);
    let mut pw = use_signal(String::new);
    let mut pw2 = use_signal(String::new);
    let mut name = use_signal(String::new);
    let mut org = use_signal(String::new);
    let mut agreed = use_signal(|| false);
    let mut loading = use_signal(|| false);
    let mut shake_form = use_signal(|| false);
    let toast = use_toast();
    let navigator = use_navigator();
    let auth = use_auth();

    let handle_register = move |_| {
        let uname = name.read().clone();
        let pwd = pw.read().clone();
        let confirm = pw2.read().clone();
        let mail = email.read().clone();

        if uname.is_empty() || pwd.is_empty() || confirm.is_empty() || mail.is_empty() {
            shake_form.set(true);
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                shake_form.set(false);
            });
            toast.error(t(*lang.read(), "register.error.fill_required"));
            return;
        }

        if pwd != confirm {
            shake_form.set(true);
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                shake_form.set(false);
            });
            toast.error(t(*lang.read(), "register.error.password_mismatch"));
            return;
        }

        let email_opt = if mail.is_empty() { None } else { Some(mail) };
        loading.set(true);
        spawn(async move {
            match AuthService::register(&uname, &pwd, email_opt.as_deref()).await {
                Ok(login_response) => {
                    auth.set_auth(
                        login_response.token.clone(),
                        CurrentUser {
                            id: login_response.id.clone(),
                            username: login_response.username.clone(),
                            roles: login_response.roles.clone(),
                        },
                    );
                    toast.success(t(*lang.read(), "register.success"));
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    loading.set(false);
                    navigator.push("/console/dashboard");
                }
                Err(e) => {
                    loading.set(false);
                    shake_form.set(true);
                    spawn(async move {
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        shake_form.set(false);
                    });
                    toast.error(&e);
                }
            }
        });
    };

    // Password strength calculation
    let pw_len = pw.read().len();
    let strength_bars: [(bool, &str); 4] = [
        (pw_len >= 3, if pw_len >= 12 { "var(--bc-success)" } else if pw_len >= 9 { "var(--bc-warning)" } else { "var(--bc-danger)" }),
        (pw_len >= 6, if pw_len >= 12 { "var(--bc-success)" } else if pw_len >= 9 { "var(--bc-warning)" } else { "var(--bc-danger)" }),
        (pw_len >= 9, if pw_len >= 12 { "var(--bc-success)" } else if pw_len >= 9 { "var(--bc-warning)" } else { "var(--bc-danger)" }),
        (pw_len >= 12, "var(--bc-success)"),
    ];

    let form_class = if shake_form() { "animate-shake" } else { "" };

    rsx! {
        div { class: "login",

            // --- LEFT: BRAND PANEL (50%) ---
            aside { class: "login-brand",
                // Logo + brand
                div { style: "display:flex; align-items:center; gap:12px;",
                    Logo { class: "login-brand-logo" }
                    div {
                        div { style: "font-size:17px; font-weight:600; line-height:1;", "BurnCloud" }
                        div { style: "font-size:11px; font-weight:500; color:rgba(255,255,255,0.4); letter-spacing:0.18em; text-transform:uppercase; margin-top:4px;", "Enterprise" }
                    }
                }

                // Center content
                div {
                    div { class: "login-brand-eyebrow", "Start in 60 seconds" }
                    h1 { class: "login-brand-headline",
                        "One binary."
                        br {}
                        "Every model."
                    }
                    p { class: "login-brand-subhead",
                        {t(*lang.read(), "register.brand.subhead")}
                    }

                    // Benefit rows
                    div { style: "display:flex; flex-direction:column; gap:12px; margin-top:32px;",
                        div { class: "login-benefit",
                            span { class: "login-benefit-check", "\u{2713}" }
                            div { style: "display:flex; flex-direction:column;",
                                span { class: "login-benefit-key", {t(*lang.read(), "register.brand.benefit_free_key")} }
                                span { class: "login-benefit-val", {t(*lang.read(), "register.brand.benefit_free_val")} }
                            }
                        }
                        div { class: "login-benefit",
                            span { class: "login-benefit-check", "\u{2713}" }
                            div { style: "display:flex; flex-direction:column;",
                                span { class: "login-benefit-key", {t(*lang.read(), "register.brand.benefit_unified_key")} }
                                span { class: "login-benefit-val", "OpenAI \u{00b7} Anthropic \u{00b7} Gemini \u{00b7} Azure \u{00b7} Qwen" }
                            }
                        }
                        div { class: "login-benefit",
                            span { class: "login-benefit-check", "\u{2713}" }
                            div { style: "display:flex; flex-direction:column;",
                                span { class: "login-benefit-key", {t(*lang.read(), "register.brand.benefit_instant_key")} }
                                span { class: "login-benefit-val", {t(*lang.read(), "register.brand.benefit_instant_val")} }
                            }
                        }
                    }
                }

                // Version
                div { class: "login-brand-version",
                    "v0.3.1 \u{00b7} build 2026.04.27 \u{00b7} burncloud.io"
                }
            }

            // --- RIGHT: FORM PANEL (50%) ---
            main { class: "login-form",
                div { class: "{form_class}", style: "display:flex; flex-direction:column; gap:18px; width:100%;",

                    // Header
                    div { style: "margin-bottom:14px;",
                        h2 { class: "login-form-title", {t(*lang.read(), "register.form.title")} }
                        div { class: "login-form-subtitle", {t(*lang.read(), "register.form.subtitle")} }
                    }

                    // Name + Org (2-column)
                    div { style: "display:grid; grid-template-columns:1fr 1fr; gap:12px;",
                        div {
                            label { class: "login-input-label", {t(*lang.read(), "register.form.name_label")} }
                            div { class: "login-input",
                                input {
                                    r#type: "text",
                                    placeholder: t(*lang.read(), "register.form.name_placeholder"),
                                    value: "{name}",
                                    oninput: move |e: Event<FormData>| name.set(e.value()),
                                }
                            }
                        }
                        div {
                            label { class: "login-input-label", {t(*lang.read(), "register.form.org_label")} " " span { style: "color:var(--bc-text-tertiary); font-weight:400;", {t(*lang.read(), "register.form.org_optional")} } }
                            div { class: "login-input",
                                input {
                                    r#type: "text",
                                    placeholder: "Acme Inc.",
                                    value: "{org}",
                                    oninput: move |e: Event<FormData>| org.set(e.value()),
                                }
                            }
                        }
                    }

                    // Email
                    div {
                        label { class: "login-input-label", {t(*lang.read(), "register.form.email_label")} }
                        div { class: "login-input",
                            input {
                                r#type: "email",
                                placeholder: "you@burncloud.com",
                                value: "{email}",
                                oninput: move |e: Event<FormData>| email.set(e.value()),
                            }
                        }
                    }

                    // Password
                    div {
                        label { class: "login-input-label", {t(*lang.read(), "register.form.password_label")} }
                        div { class: "login-input",
                            input {
                                r#type: "password",
                                placeholder: t(*lang.read(), "register.form.password_placeholder"),
                                value: "{pw}",
                                oninput: move |e: Event<FormData>| pw.set(e.value()),
                            }
                        }
                        // Strength meter
                        if pw_len > 0 {
                            div { class: "pw-meter",
                                for (i, (active, color)) in strength_bars.iter().enumerate() {
                                    div {
                                        key: "{i}",
                                        class: "pw-meter-bar",
                                        style: if *active { format!("background:{}", color) } else { String::new() },
                                    }
                                }
                            }
                        }
                    }

                    // Confirm password
                    div {
                        label { class: "login-input-label", {t(*lang.read(), "register.form.confirm_password_label")} }
                        div { class: "login-input",
                            input {
                                r#type: "password",
                                placeholder: t(*lang.read(), "register.form.confirm_password_placeholder"),
                                value: "{pw2}",
                                oninput: move |e: Event<FormData>| pw2.set(e.value()),
                            }
                        }
                    }

                    // Terms checkbox
                    label { style: "display:flex; align-items:flex-start; gap:10px; font-size:12px; color:var(--bc-text-secondary); line-height:1.5; cursor:pointer; user-select:none;",
                        input {
                            r#type: "checkbox",
                            checked: agreed(),
                            onchange: move |_| agreed.set(!agreed()),
                            style: "margin-top:2px; accent-color:#000;",
                        }
                        span {
                            {t(*lang.read(), "register.form.terms_prefix")}
                            a { style: "color:var(--bc-primary); text-decoration:none;", {t(*lang.read(), "register.form.terms_tos")} }
                            {t(*lang.read(), "register.form.terms_and")}
                            a { style: "color:var(--bc-primary); text-decoration:none;", {t(*lang.read(), "register.form.terms_privacy")} }
                            {t(*lang.read(), "register.form.terms_suffix")}
                        }
                    }

                    // Register button
                    button {
                        class: "landing-btn landing-btn-dark",
                        style: "width:100%; height:48px; font-size:15px; border-radius:12px; margin-top:4px;",
                        disabled: loading(),
                        onclick: handle_register,
                        if loading() {
                            {t(*lang.read(), "register.form.registering")}
                        } else {
                            {t(*lang.read(), "register.form.create_account")}
                        }
                    }

                    // Divider
                    div { class: "login-divider",
                        div { class: "login-divider-line" }
                        span { class: "login-divider-text", {t(*lang.read(), "register.form.or")} }
                        div { class: "login-divider-line" }
                    }

                    // OAuth buttons
                    div { class: "login-social-grid",
                        button { class: "landing-btn", style: "height:42px; font-size:13px; background:transparent; color:var(--bc-text-primary); border:1px solid var(--bc-border); border-radius:12px; width:100%;",
                            span { style: "font-weight:700; margin-right:6px;", "G" } " Google"
                        }
                        button { class: "landing-btn", style: "height:42px; font-size:13px; background:transparent; color:var(--bc-text-primary); border:1px solid var(--bc-border); border-radius:12px; width:100%;",
                            svg { width: "14", height: "14", view_box: "0 0 24 24", fill: "currentColor", style: "margin-right:6px;",
                                path { d: "M12 .5C5.65.5.5 5.65.5 12c0 5.08 3.29 9.39 7.86 10.91.58.1.79-.25.79-.56v-2.05c-3.2.7-3.87-1.36-3.87-1.36-.52-1.33-1.27-1.69-1.27-1.69-1.04-.71.08-.69.08-.69 1.15.08 1.76 1.18 1.76 1.18 1.02 1.75 2.68 1.24 3.34.95.1-.74.4-1.24.73-1.53-2.55-.29-5.24-1.28-5.24-5.69 0-1.26.45-2.29 1.18-3.09-.12-.29-.51-1.46.11-3.04 0 0 .96-.31 3.15 1.18.91-.25 1.89-.38 2.86-.38.97 0 1.95.13 2.86.38 2.18-1.49 3.14-1.18 3.14-1.18.62 1.58.23 2.75.11 3.04.74.8 1.18 1.83 1.18 3.09 0 4.42-2.69 5.4-5.25 5.68.41.36.78 1.05.78 2.12v3.14c0 .31.21.66.79.55C20.21 21.39 23.5 17.07 23.5 12 23.5 5.65 18.35.5 12 .5z" }
                            }
                            "GitHub"
                        }
                    }

                    // Switch link
                    div { class: "login-footer",
                        {t(*lang.read(), "register.form.has_account")}
                        Link { to: "/login", style: "color:var(--bc-primary); text-decoration:none; font-weight:500; cursor:pointer;", {t(*lang.read(), "register.form.login_now")} }
                    }
                }
            }
        }
    }
}
