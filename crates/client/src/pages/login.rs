use dioxus::prelude::*;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::use_toast;
use burncloud_client_shared::components::{BCButton, BCInput, BCCard};
use crate::app::Route;

#[component]
pub fn LoginPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut loading = use_signal(|| false);
    let toast = use_toast();
    let navigator = use_navigator();

    let handle_login = move |_| {
        loading.set(true);
        spawn(async move {
            match AuthService::login(&username(), &password()).await {
                Ok(_) => {
                    loading.set(false);
                    toast.success("登录成功");
                    // Redirect to Dashboard
                    navigator.push(Route::Dashboard {});
                },
                Err(e) => {
                    loading.set(false);
                    println!("LoginPage: Login error: {}", e);
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
                    h2 { class: "text-title font-bold", "登录 BurnCloud" }
                }
                
                BCInput {
                    label: Some("用户名".to_string()),
                    value: "{username}",
                    placeholder: "请输入用户名".to_string(),
                    oninput: move |e: FormEvent| username.set(e.value())
                }

                BCInput {
                    label: Some("密码".to_string()),
                    r#type: "password".to_string(),
                    value: "{password}",
                    placeholder: "请输入密码".to_string(),
                    oninput: move |e: FormEvent| password.set(e.value())
                }

                div { class: "mt-lg",
                    BCButton { 
                        class: "w-full",
                        loading: loading(),
                        onclick: handle_login,
                        "登录" 
                    }
                }

                div { class: "text-center mt-lg",
                    Link { to: Route::RegisterPage {}, class: "text-decoration-none", "注册新账号" }
                }
            }
        }
    }
}