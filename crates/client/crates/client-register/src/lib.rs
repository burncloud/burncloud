// UI registration — HTTP response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::auth_context::{use_auth, CurrentUser};
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::components::logo::Logo;
use burncloud_client_shared::components::{FormMode, SchemaForm};
use burncloud_client_shared::schema::register_schema;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;
use std::time::Duration;

#[component]
pub fn RegisterPage() -> Element {
    let schema = register_schema();
    let mut form_data = use_signal(|| serde_json::Value::Object(serde_json::Map::new()));
    let mut loading = use_signal(|| false);
    let mut shake_form = use_signal(|| false);
    let toast = use_toast();
    let navigator = use_navigator();
    let auth = use_auth();

    let handle_submit = move |data: serde_json::Value| {
        form_data.set(data);
    };

    let handle_register = move |_: Event<MouseData>| {
        let current = form_data.read().clone();
        let uname = current["username"].as_str().unwrap_or("").to_string();
        let pwd = current["password"].as_str().unwrap_or("").to_string();
        let confirm = current["confirm_password"].as_str().unwrap_or("").to_string();
        let mail = current["email"].as_str().unwrap_or("").to_string();

        // Validate required fields
        if uname.is_empty() || pwd.is_empty() || confirm.is_empty() {
            shake_form.set(true);
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                shake_form.set(false);
            });
            toast.error("请填写所有必填字段");
            return;
        }

        // Password match
        if pwd != confirm {
            shake_form.set(true);
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                shake_form.set(false);
            });
            toast.error("两次输入的密码不一致");
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
                    toast.success("注册成功！欢迎使用 BurnCloud");
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

    let form_data_for_keydown = form_data.clone();
    let handle_keydown = move |e: Event<KeyboardData>| {
        if e.key() != Key::Enter || loading() {
            return;
        }
        let current = form_data_for_keydown.read().clone();
        let uname = current["username"].as_str().unwrap_or("").to_string();
        let pwd = current["password"].as_str().unwrap_or("").to_string();
        let confirm = current["confirm_password"].as_str().unwrap_or("").to_string();
        let mail = current["email"].as_str().unwrap_or("").to_string();

        if uname.is_empty() || pwd.is_empty() || confirm.is_empty() || pwd != confirm {
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
                    toast.success("注册成功！欢迎使用 BurnCloud");
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

    rsx! {
        // Container: Aurora Canvas
        div { class: "h-full w-full min-h-screen overflow-hidden bg-[var(--bc-bg-canvas)] text-primary relative selection:bg-[var(--bc-primary)] selection:text-white font-sans flex items-center justify-center p-xxxl",

            // ========== BACKGROUND: Liquid Light Field ==========
            div { class: "absolute inset-0 pointer-events-none overflow-hidden",
                div { class: "absolute top-0 left-0 w-full h-16 z-50 cursor-default", style: "-webkit-app-region: drag;" }
                div { class: "absolute top-[-15%] right-[-15%] w-[900px] h-[900px] bg-gradient-to-l from-[var(--bc-primary-dark)]/15 via-[#AF52DE]/12 to-[var(--bc-primary)]/10 rounded-full blur-[100px] animate-aurora animate-morph [animation-duration:30s]" }
                div { class: "absolute bottom-[-20%] left-[-10%] w-[800px] h-[800px] bg-gradient-to-r from-[var(--bc-success)]/12 via-[#30B0C7]/10 to-transparent rounded-full blur-[80px] animate-aurora [animation-delay:5s] [animation-duration:40s]" }
                div { class: "absolute top-[30%] left-[15%] w-[350px] h-[350px] bg-gradient-to-br from-[var(--bc-warning)]/10 to-[#FF2D55]/8 rounded-full blur-[60px] animate-float [animation-delay:3s] [animation-duration:20s]" }
                div {
                    class: "absolute inset-0 opacity-[0.03] mix-blend-overlay",
                    style: "background-image: url(\"data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E\");"
                }
            }

            // ========== REGISTER CARD ==========
            div { class: "relative z-10 w-full max-w-[400px] mx-4 animate-in",
                div { class: "p-8 relative",

                    // Logo & Header
                    div { class: "text-center mb-lg relative z-10",
                        div { class: "relative inline-flex items-center justify-center w-24 h-24 mb-10",
                            div {
                                class: "w-full h-full rounded-full bg-white/20 border border-white/30 backdrop-blur-sm shadow-[0_8px_30px_-6px_rgba(88,86,214,0.12)] flex items-center justify-center",
                                Logo { class: "w-10 h-10 text-[var(--bc-primary-dark)] fill-current translate-y-0.5" }
                            }
                        }
                        div { class: "flex flex-col items-center justify-center space-y-2 mb-lg",
                            h1 { class: "text-large-title font-semibold tracking-tight text-primary", "Unleash Intelligence." }
                            h1 { class: "text-large-title font-semibold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-[var(--bc-primary-dark)] to-[#AF52DE]", "Your Second Brain." }
                        }
                        p { class: "text-subtitle text-tertiary font-semibold tracking-wide", "开启本地优先的 AI 体验" }
                    }

                    // Schema-driven form
                    div {
                        class: if shake_form() {
                            "relative z-10 animate-shake"
                        } else {
                            "relative z-10"
                        },
                        onkeydown: handle_keydown,

                        SchemaForm {
                            schema: schema.clone(),
                            data: form_data,
                            mode: FormMode::Create,
                            show_actions: false,
                            on_submit: handle_submit,
                        }

                        // Register Button
                        button {
                            class: "bc-btn-gradient group relative mt-lg w-full",
                            disabled: loading(),
                            onclick: handle_register,
                            span { class: "relative z-10 flex items-center justify-center gap-sm",
                                if loading() {
                                    svg { class: "w-5 h-5 animate-spin", fill: "none", view_box: "0 0 24 24",
                                        circle { class: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", stroke_width: "4" }
                                        path { class: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" }
                                    }
                                    "注册中..."
                                } else {
                                    "开始体验"
                                    svg { class: "w-5 h-5 transition-transform duration-300 group-hover:translate-x-1", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M13 7l5 5m0 0l-5 5m5-5H6" }
                                    }
                                }
                            }
                        }
                    }

                    // Footer Link
                    div { class: "text-center mt-xxl relative z-10",
                        Link {
                            to: "/login",
                            class: "text-subtitle font-medium text-secondary hover:text-primary transition-colors",
                            "已有账号？返回登录"
                        }
                    }
                }
            }
        }
    }
}
