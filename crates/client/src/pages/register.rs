use crate::app::Route;
use burncloud_client_shared::auth_service::AuthService;
use dioxus::prelude::*;

#[component]
pub fn RegisterPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut confirm_password = use_signal(|| "".to_string());
    let mut email = use_signal(|| "".to_string());
    let mut error_msg = use_signal(|| "".to_string());
    let mut success_msg = use_signal(|| "".to_string());
    let navigator = use_navigator();

    let handle_register = move |_| {
        spawn(async move {
            error_msg.set("".to_string());

            if password() != confirm_password() {
                error_msg.set("两次输入的密码不一致".to_string());
                return;
            }

            let email_val = email();
            let email_opt = if email_val.is_empty() {
                None
            } else {
                Some(email_val.as_str())
            };

            match AuthService::register(&username(), &password(), email_opt).await {
                Ok(_) => {
                    success_msg.set("注册成功，正在跳转登录...".to_string());
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    navigator.push(Route::LoginPage {});
                }
                Err(e) => {
                    error_msg.set(e);
                }
            }
        });
    };

    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                h2 { class: "text-title font-bold text-center mb-xl", "注册账号" }

                if !error_msg().is_empty() {
                    div { class: "alert alert-error mb-lg", "{error_msg}" }
                }
                if !success_msg().is_empty() {
                    div { class: "alert alert-success mb-lg", "{success_msg}" }
                }

                div { class: "form-group",
                    label { "用户名" }
                    input {
                        r#type: "text",
                        class: "form-control",
                        value: "{username}",
                        oninput: move |e| username.set(e.value())
                    }
                }

                div { class: "form-group",
                    label { "邮箱 (可选)" }
                    input {
                        r#type: "email",
                        class: "form-control",
                        value: "{email}",
                        oninput: move |e| email.set(e.value())
                    }
                }

                div { class: "form-group",
                    label { "密码" }
                    input {
                        r#type: "password",
                        class: "form-control",
                        value: "{password}",
                        oninput: move |e| password.set(e.value())
                    }
                }

                div { class: "form-group",
                    label { "确认密码" }
                    input {
                        r#type: "password",
                        class: "form-control",
                        value: "{confirm_password}",
                        oninput: move |e| confirm_password.set(e.value())
                    }
                }

                button {
                    class: "btn btn-primary w-full mt-lg",
                    onclick: handle_register,
                    "注册"
                }

                div { class: "text-center mt-lg",
                    Link { to: Route::LoginPage {}, "已有账号？去登录" }
                }
            }
        }
    }
}
