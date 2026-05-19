use crate::app::Route;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::i18n::{t, use_i18n};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn ForgotPasswordPage() -> Element {
    let i18n = use_i18n();
    let lang = i18n.language;
    let mut email = use_signal(String::new);
    let mut loading = use_signal(|| false);
    let mut success = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    let toast = use_toast();
    let navigator = use_navigator();

    let handle_submit = move |_| {
        let mail = email.read().clone();
        if mail.is_empty() {
            error_msg.set(Some(
                t(*lang.read(), "forgot_password.error.email_required").to_string(),
            ));
            return;
        }
        error_msg.set(None);
        loading.set(true);
        spawn(async move {
            match AuthService::forgot_password(&mail).await {
                Ok(_) => {
                    loading.set(false);
                    success.set(true);
                    toast.success(t(*lang.read(), "forgot_password.success"));
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
                    h2 { class: "login-form-title", {t(*lang.read(), "forgot_password.title")} }
                    div { class: "login-form-subtitle", {t(*lang.read(), "forgot_password.subtitle")} }
                }

                if success() {
                    div { class: "login-success-text", {t(*lang.read(), "forgot_password.success")} }
                } else {
                    div { class: "flex flex-col gap-xl",
                        div {
                            label { class: "login-input-label", {t(*lang.read(), "forgot_password.email_label")} }
                            div { class: "login-input",
                                input {
                                    r#type: "email",
                                    placeholder: t(*lang.read(), "forgot_password.email_placeholder"),
                                    value: "{email}",
                                    oninput: move |e: Event<FormData>| email.set(e.value()),
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
                                {t(*lang.read(), "forgot_password.sending")}
                            } else {
                                {t(*lang.read(), "forgot_password.submit")}
                            }
                        }
                    }
                }

                div { class: "login-footer",
                    a {
                        class: "bc-link-semibold",
                        onclick: move |_| { navigator.push(Route::LoginPage {}); },
                        {t(*lang.read(), "forgot_password.back_to_login")}
                    }
                }
            }
        }
    }
}
