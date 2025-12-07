use dioxus::prelude::*;
use burncloud_client_shared::auth_service::AuthService;
use crate::app::Route;

#[component]
pub fn LoginPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut error_msg = use_signal(|| "".to_string());
    let navigator = use_navigator();

    let handle_login = move |_| {
        spawn(async move {
            error_msg.set("".to_string());
            match AuthService::login(&username(), &password()).await {
                Ok(_) => {
                    // Redirect to Dashboard
                    navigator.push(Route::Dashboard {});
                },
                Err(e) => {
                    error_msg.set(e);
                }
            }
        });
    };

    rsx! {
        div { class: "auth-container",
            div { class: "auth-card",
                h2 { class: "text-title font-bold text-center mb-xl", "登录 BurnCloud" }
                
                if !error_msg().is_empty() {
                    div { class: "alert alert-error mb-lg", "{error_msg}" }
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
                    label { "密码" }
                    input { 
                        r#type: "password", 
                        class: "form-control",
                        value: "{password}",
                        oninput: move |e| password.set(e.value())
                    }
                }

                button { 
                    class: "btn btn-primary w-full mt-lg",
                    onclick: handle_login,
                    "登录" 
                }

                div { class: "text-center mt-lg",
                    Link { to: Route::RegisterPage {}, "注册新账号" }
                }
            }
        }
    }
}
