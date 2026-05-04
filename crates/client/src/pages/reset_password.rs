use crate::app::Route;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::i18n::{t, use_i18n};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn ResetPasswordPage(token: Option<String>) -> Element {
    let i18n = use_i18n();
    let lang = i18n.language;
    let mut new_password = use_signal(String::new);
    let mut confirm_password = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut success = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let toast = use_toast();
    let navigator = use_navigator();

    let token_value = match token {
        Some(ref t) if !t.is_empty() => t.clone(),
        _ => String::new(),
    };

    let handle_submit = move |_| {
        let pw = new_password.read().clone();
        let cpw = confirm_password.read().clone();
        if pw.is_empty() || cpw.is_empty() {
            error_msg.set(Some(t(*lang.read(), "reset_password.error.password_required").to_string()));
            return;
        }
        if pw != cpw {
            error_msg.set(Some(t(*lang.read(), "reset_password.error.password_mismatch").to_string()));
            return;
        }
        if token_value.is_empty() {
            error_msg.set(Some(t(*lang.read(), "reset_password.error.invalid_token").to_string()));
            return;
        }
        error_msg.set(None);
        loading.set(true);
        let tv = token_value.clone();
        spawn(async move {
            match AuthService::reset_password(&tv, &pw).await {
                Ok(_) => {
                    loading.set(false);
                    success.set(true);
                    toast.success(t(*lang.read(), "reset_password.success"));
                }
                Err(e) => {
                    loading.set(false);
                    error_msg.set(Some(e));
                }
            }
        });
    };

    rsx! {
        div { class: "login",
            aside { class: "login-brand",
                div { class: "login-brand-header",
                    div {
                        div { class: "login-brand-name", "BurnCloud" }
                        div { class: "login-brand-sublabel", "Enterprise" }
                    }
                }
            }

            main { class: "login-form",
                div { class: "mb-xxxl",
                    h2 { class: "login-form-title", {t(*lang.read(), "reset_password.title")} }
                    div { class: "login-form-subtitle", {t(*lang.read(), "reset_password.subtitle")} }
                }

                if token.as_ref().is_none_or(|t| t.is_empty()) {
                    div { class: "login-error-text", {t(*lang.read(), "reset_password.error.invalid_token")} }
                } else if success() {
                    div { class: "login-success-text", {t(*lang.read(), "reset_password.success")} }
                } else {
                    div { class: "flex flex-col gap-xl",
                        div {
                            label { class: "login-input-label", {t(*lang.read(), "reset_password.new_password_label")} }
                            div { class: "login-input",
                                input {
                                    r#type: "password",
                                    placeholder: t(*lang.read(), "reset_password.new_password_placeholder"),
                                    value: "{new_password}",
                                    oninput: move |e: Event<FormData>| new_password.set(e.value()),
                                }
                            }
                        }

                        div {
                            label { class: "login-input-label", {t(*lang.read(), "reset_password.confirm_password_label")} }
                            div { class: "login-input",
                                input {
                                    r#type: "password",
                                    placeholder: t(*lang.read(), "reset_password.confirm_password_placeholder"),
                                    value: "{confirm_password}",
                                    oninput: move |e: Event<FormData>| confirm_password.set(e.value()),
                                }
                            }
                        }

                        if let Some(err) = error_msg() {
                            div { class: "login-error-text", "{err}" }
                        }

                        button {
                            class: "landing-btn landing-btn-dark bc-btn-block bc-btn-lg",
                            disabled: loading(),
                            onclick: handle_submit,
                            if loading() {
                                {t(*lang.read(), "reset_password.resetting")}
                            } else {
                                {t(*lang.read(), "reset_password.submit")}
                            }
                        }
                    }
                }

                div { class: "login-footer",
                    a {
                        class: "bc-link-semibold",
                        onclick: move |_| { navigator.push(Route::LoginPage {}); },
                        {t(*lang.read(), "reset_password.back_to_login")}
                    }
                }
            }
        }
    }
}
