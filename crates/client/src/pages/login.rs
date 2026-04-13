// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use crate::app::Route;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::components::{
    logo::Logo, BCButton, ButtonSize, ButtonVariant, FormMode, SchemaForm,
};
use burncloud_client_shared::schema::login_schema;
use burncloud_client_shared::use_toast;
use burncloud_client_shared::utils::storage::ClientState;
use burncloud_client_shared::{use_auth, CurrentUser};
use dioxus::prelude::*;

#[component]
pub fn LoginPage() -> Element {
    // Load persisted state
    let state = ClientState::load();
    let last_username = state.last_username.unwrap_or_default();

    let form_data = use_signal(move || {
        serde_json::json!({
            "username": last_username,
            "password": ""
        })
    });
    let mut loading = use_signal(|| false);
    let mut login_error = use_signal(|| None::<String>);
    let toast = use_toast();
    let navigator = use_navigator();
    let auth = use_auth();

    let schema = login_schema();

    let mut handle_login = move |value: serde_json::Value| {
        loading.set(true);
        login_error.set(None);

        let u = value["username"].as_str().unwrap_or("").to_string();
        let p = value["password"].as_str().unwrap_or("").to_string();

        spawn(async move {
            match AuthService::login(&u, &p).await {
                Ok(response) => {
                    loading.set(false);

                    let new_state = ClientState {
                        last_username: Some(u.clone()),
                        last_password: None,
                        auth_token: Some(response.token.clone()),
                        user_info: Some(
                            serde_json::to_string(&CurrentUser {
                                id: response.id.clone(),
                                username: response.username.clone(),
                                roles: response.roles.clone(),
                            })
                            .unwrap_or_default(),
                        ),
                    };
                    new_state.save();

                    let user = CurrentUser {
                        id: response.id,
                        username: response.username,
                        roles: response.roles,
                    };
                    auth.set_auth(response.token, user);
                    toast.success("登录成功");
                    navigator.push(Route::Dashboard {});
                }
                Err(e) => {
                    loading.set(false);
                    eprintln!("LoginPage: Login error: {}", e);
                    login_error.set(Some("用户名或密码错误".to_string()));
                }
            }
        });
    };

    rsx! {
        div { class: "login-shell",

            // ========== LEFT: BRAND (40%) ==========
            aside { class: "login-brand",
                div { class: "login-brand-grid" }
                div { class: "login-brand-inner",
                    Logo { class: Some("login-brand-logo".to_string()) }
                    div { style: "margin-top: auto;",
                        h1 { class: "login-brand-headline",
                            "Unleash"
                            br {}
                            "Intelligence."
                        }
                        p { class: "login-brand-subhead",
                            "Seamlessly connecting your local nodes with the power of the second brain."
                        }
                    }
                    div { class: "login-brand-eyebrow",
                        div { class: "login-brand-eyebrow-line" }
                        "Intelligence Reimagined"
                    }
                }
            }

            // ========== RIGHT: FORM (60%) ==========
            main { class: "login-pane",
                div { class: "login-card",

                    // Header
                    header { class: "login-card-header",
                        h2 { class: "login-card-title", "登录" }
                        p { class: "login-card-subtitle", "欢迎回来，请登录您的账号" }
                    }

                    // Schema-driven form
                    form {
                        onsubmit: move |e: FormEvent| {
                            e.stop_propagation();
                            let data = form_data.read().clone();
                            handle_login(data);
                        },

                        SchemaForm {
                            schema: schema.clone(),
                            data: form_data,
                            mode: FormMode::Create,
                            show_actions: false,
                            class: "login-form",
                        }

                        if let Some(err) = login_error() {
                            div { class: "bc-input-error-row",
                                div { class: "bc-input-error-dot" }
                                span { class: "bc-input-error-text", "{err}" }
                            }
                        }

                        BCButton {
                            r#type: Some("submit".to_string()),
                            variant: ButtonVariant::Black,
                            size: ButtonSize::Large,
                            class: "bc-btn-block bc-btn-press",
                            loading: loading(),
                            children: rsx! { "登录" }
                        }
                    }

                    // Social Login
                    div { class: "login-social-section",
                        div { class: "login-divider",
                            div { class: "login-divider-line" }
                            span { class: "login-divider-text", "或者使用以下方式" }
                            div { class: "login-divider-line" }
                        }

                        div { class: "login-social-grid",
                            BCButton {
                                variant: ButtonVariant::Social,
                                size: ButtonSize::Large,
                                class: "bc-btn-block",
                                children: rsx! {
                                    svg { class: "bc-btn-icon", width: "20", height: "20", view_box: "0 0 24 24", fill: "currentColor",
                                        path { d: "M12.48 10.92v3.28h7.84c-.24 1.84-.908 3.152-1.928 4.176-1.288 1.288-3.312 2.688-6.912 2.688-5.552 0-9.92-4.48-9.92-10.032s4.368-10.032 9.92-10.032c3.008 0 5.232 1.192 6.848 2.72l2.312-2.312C18.424 1.296 15.84 0 12.48 0 6.448 0 1.552 4.936 1.552 11s4.896 11 10.928 11c3.272 0 5.752-1.072 7.712-3.12 2.032-2.032 2.672-4.904 2.672-7.232 0-.688-.048-1.344-.144-1.952h-8.24z" }
                                    }
                                    "Google"
                                }
                            }
                            BCButton {
                                variant: ButtonVariant::Social,
                                size: ButtonSize::Large,
                                class: "bc-btn-block",
                                children: rsx! {
                                    svg { class: "bc-btn-icon", width: "20", height: "20", view_box: "0 0 24 24", fill: "currentColor",
                                        path { d: "M12 .297c-6.63 0-12 5.373-12 12 0 5.303 3.438 9.8 8.205 11.385.6.113.82-.258.82-.577 0-.285-.01-1.04-.015-2.04-3.338.724-4.042-1.61-4.042-1.61C4.422 18.07 3.633 17.7 3.633 17.7c-1.087-.744.084-.729.084-.729 1.205.084 1.838 1.236 1.838 1.236 1.07 1.835 2.809 1.305 3.495.998.108-.776.417-1.305.76-1.605-2.665-.3-5.466-1.332-5.466-5.93 0-1.31.465-2.38 1.235-3.22-.135-.303-.54-1.523.105-3.176 0 0 1.005-.322 3.3 1.23.96-.267 1.98-.399 3-.405 1.02.006 2.04.138 3 .405 2.28-1.552 3.285-1.23 3.285-1.23.645 1.653.24 2.873.12 3.176.765.84 1.23 1.91 1.23 3.22 0 4.61-2.805 5.625-5.475 5.92.42.36.81 1.096.81 2.22 0 1.606-.015 2.896-.015 3.286 0 .315.21.69.825.57C20.565 22.092 24 17.592 24 12.297c0-6.627-5.373-12-12-12" }
                                    }
                                    "GitHub"
                                }
                            }
                        }
                    }

                    // Footer Link
                    div { class: "login-footer",
                        Link {
                            to: Route::RegisterPage {},
                            "还没有账号? 立即注册"
                        }
                    }
                }
            }
        }
    }
}
