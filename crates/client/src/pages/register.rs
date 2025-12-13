use crate::app::Route;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::components::{BCButton, BCCard, BCInput};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn RegisterPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut confirm_password = use_signal(|| "".to_string());
    let mut email = use_signal(|| "".to_string());
    let mut loading = use_signal(|| false);
    
    let toast = use_toast();
    let navigator = use_navigator();

    let handle_register = move |_| {
        loading.set(true);
        spawn(async move {
            if password() != confirm_password() {
                toast.error("两次输入的密码不一致");
                loading.set(false);
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
                    toast.success("注册成功，正在跳转登录...");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    loading.set(false);
                    navigator.push(Route::LoginPage {});
                }
                Err(e) => {
                    loading.set(false);
                    toast.error(&e);
                }
            }
        });
    };

    rsx! {
        div { class: "auth-container",
            BCCard {
                class: "auth-card",
                div { class: "text-center mb-xl",
                    h2 { class: "text-title font-bold", "加入 BurnCloud" }
                    p { class: "text-sm text-base-content/60 mt-2", "连接您的本地算力专家" }
                }

                BCInput {
                    label: Some("用户名".to_string()),
                    value: "{username}",
                    placeholder: "设置您的唯一标识".to_string(),
                    oninput: move |e: FormEvent| username.set(e.value())
                }

                BCInput {
                    label: Some("邮箱 (可选)".to_string()),
                    value: "{email}",
                    placeholder: "用于接收 BurnGrid 通知".to_string(),
                    r#type: "email".to_string(),
                    oninput: move |e: FormEvent| email.set(e.value())
                }

                BCInput {
                    label: Some("密码".to_string()),
                    r#type: "password".to_string(),
                    value: "{password}",
                    placeholder: "设置强密码".to_string(),
                    oninput: move |e: FormEvent| password.set(e.value())
                }

                BCInput {
                    label: Some("确认密码".to_string()),
                    r#type: "password".to_string(),
                    value: "{confirm_password}",
                    placeholder: "再次输入密码".to_string(),
                    oninput: move |e: FormEvent| confirm_password.set(e.value())
                }

                div { class: "mt-lg",
                    BCButton {
                        class: "w-full",
                        loading: loading(),
                        onclick: handle_register,
                        "立即注册"
                    }
                }

                div { class: "text-center mt-lg",
                    Link { to: Route::LoginPage {}, class: "text-sm link link-hover opacity-70", "已有账号？返回登录" }
                }
            }
        }
    }
}
