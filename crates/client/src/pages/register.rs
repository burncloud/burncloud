use crate::app::Route;
use crate::components::logo::Logo;
use burncloud_client_shared::auth_service::AuthService;
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

    let logo_margin = if cfg!(feature = "liveview") {
        "mb-6"
    } else if cfg!(feature = "desktop") {
        "mb-10"
    } else {
        "mb-6"
    };

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
        // Container: Aurora Canvas
        div { class: "h-full w-full min-h-screen overflow-hidden bg-[#F5F5F7] text-[#1D1D1F] relative selection:bg-[#0071E3] selection:text-white font-sans flex items-center justify-center py-12",

            // ========== BACKGROUND: Liquid Light Field ==========
            div { class: "absolute inset-0 pointer-events-none overflow-hidden",
                // Layer 1: Primary Aurora Blob - shifted for variety
                div { class: "absolute top-[-15%] right-[-15%] w-[900px] h-[900px] bg-gradient-to-l from-[#5856D6]/15 via-[#AF52DE]/12 to-[#007AFF]/10 rounded-full blur-[100px] animate-aurora animate-morph" }

                // Layer 2: Secondary Flow
                div { class: "absolute bottom-[-20%] left-[-10%] w-[800px] h-[800px] bg-gradient-to-r from-[#34C759]/12 via-[#30B0C7]/10 to-transparent rounded-full blur-[80px] animate-aurora [animation-delay:5s] [animation-duration:22s]" }

                // Layer 3: Accent Orb
                div { class: "absolute top-[30%] left-[15%] w-[350px] h-[350px] bg-gradient-to-br from-[#FF9500]/10 to-[#FF2D55]/8 rounded-full blur-[60px] animate-float [animation-delay:3s]" }

                // Grid pattern overlay - reduced opacity for cleaner background
                div {
                    class: "absolute inset-0 opacity-[0.015]",
                    style: "background-image: radial-gradient(circle at 1px 1px, #1D1D1F 0.5px, transparent 0); background-size: 48px 48px;"
                }
            }

            // ========== REGISTER CARD: Glass Morphism ==========
            div { class: "relative z-10 w-full max-w-[400px] mx-4 animate-in",

                // Glass Card - refined aesthetics with sophisticated shadow
                div { class: "backdrop-blur-xl bg-white/75 rounded-[24px] shadow-[0_8px_32px_-4px_rgba(0,0,0,0.08),0_24px_56px_-8px_rgba(88,86,214,0.12)] border border-white/60 p-8 relative overflow-hidden",

                    // Glossy reflection
                    div { class: "absolute top-0 left-0 w-56 h-56 bg-gradient-to-br from-white/80 to-transparent opacity-60 pointer-events-none rounded-full blur-2xl -translate-y-1/2 -translate-x-1/2" }

                    // Logo & Header
                    div { class: "text-center mb-8 relative z-10",
                        // Logo with gradient ring
                        div { class: "relative inline-flex items-center justify-center w-16 h-16 {logo_margin}",
                            // Gradient ring
                            div { class: "absolute inset-0 rounded-[20px] bg-gradient-to-br from-[#007AFF] via-[#5856D6] to-[#AF52DE] p-[2px]",
                                div { class: "w-full h-full rounded-[18px] bg-white" }
                            }
                            // Icon
                            div { class: "absolute inset-0 flex items-center justify-center overflow-hidden",
                                Logo { class: "w-7 h-7 text-[#5856D6] fill-current" }
                            }
                        }

                        // Header Slogan
                        div { class: "flex flex-col items-center justify-center space-y-1 mb-4",
                            h1 { class: "text-2xl font-semibold tracking-tight text-[#1D1D1F]",
                                "One Interface."
                            }
                            h1 { class: "text-2xl font-semibold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-[#5856D6] to-[#AF52DE]",
                                "Every Intelligence."
                            }
                        }
                        p { class: "text-[15px] text-[#6E6E73] font-medium",
                            "开启本地优先的 AI 体验"
                        }
                    }

                    // Form
                    div { class: "space-y-4 relative z-10",
                        // Username Input
                        div { class: "group",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider",
                                "用户名"
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#5856D6] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" }
                                    }
                                }
                                input {
                                    class: "w-full h-13 pl-12 pr-4 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#5856D6] focus:ring-4 focus:ring-[#5856D6]/10 focus:bg-white",
                                    r#type: "text",
                                    value: "{username}",
                                    placeholder: "设置您的唯一标识",
                                    oninput: move |e| username.set(e.value())
                                }
                            }
                        }

                        // Email Input (Optional)
                        div { class: "group",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider",
                                "邮箱"
                                span { class: "ml-2 text-[11px] text-[#C7C7CC] normal-case tracking-normal", "(可选)" }
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#5856D6] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M21.75 6.75v10.5a2.25 2.25 0 01-2.25 2.25h-15a2.25 2.25 0 01-2.25-2.25V6.75m19.5 0A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25m19.5 0v.243a2.25 2.25 0 01-1.07 1.916l-7.5 4.615a2.25 2.25 0 01-2.36 0L3.32 8.91a2.25 2.25 0 01-1.07-1.916V6.75" }
                                    }
                                }
                                input {
                                    class: "w-full h-13 pl-12 pr-4 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#5856D6] focus:ring-4 focus:ring-[#5856D6]/10 focus:bg-white",
                                    r#type: "email",
                                    value: "{email}",
                                    placeholder: "用于接收通知",
                                    oninput: move |e| email.set(e.value())
                                }
                            }
                        }

                        // Password Input
                        div { class: "group",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider",
                                "密码"
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#5856D6] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" }
                                    }
                                }
                                input {
                                    class: "w-full h-13 pl-12 pr-4 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#5856D6] focus:ring-4 focus:ring-[#5856D6]/10 focus:bg-white",
                                    r#type: "password",
                                    value: "{password}",
                                    placeholder: "设置强密码",
                                    oninput: move |e| password.set(e.value())
                                }
                            }
                        }

                        // Confirm Password Input
                        div { class: "group",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider",
                                "确认密码"
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#5856D6] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                                    }
                                }
                                input {
                                    class: "w-full h-13 pl-12 pr-4 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#5856D6] focus:ring-4 focus:ring-[#5856D6]/10 focus:bg-white",
                                    r#type: "password",
                                    value: "{confirm_password}",
                                    placeholder: "再次输入密码",
                                    oninput: move |e| confirm_password.set(e.value())
                                }
                            }
                        }

                        // Register Button - Primary Action with high visual weight
                        button {
                            class: "group relative w-full h-14 mt-6 text-[17px] font-semibold text-white transition-all duration-300 bg-[#1D1D1F] rounded-2xl hover:bg-[#2C2C2E] shadow-[0_4px_14px_-2px_rgba(29,29,31,0.25),0_12px_32px_-4px_rgba(29,29,31,0.15)] hover:shadow-[0_8px_24px_-4px_rgba(29,29,31,0.35),0_16px_40px_-8px_rgba(29,29,31,0.2)] hover:scale-[1.015] hover:-translate-y-0.5 active:scale-[0.985] active:translate-y-0 disabled:opacity-60 disabled:cursor-not-allowed disabled:hover:scale-100 disabled:hover:translate-y-0 overflow-hidden",
                            disabled: loading(),
                            onclick: handle_register,

                            span { class: "relative z-10 flex items-center justify-center gap-2",
                                if loading() {
                                    // Loading spinner
                                    svg { class: "w-5 h-5 animate-spin", fill: "none", view_box: "0 0 24 24",
                                        circle { class: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", stroke_width: "4" }
                                        path { class: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" }
                                    }
                                    "注册中..."
                                } else {
                                    "创建账号"
                                    svg { class: "w-5 h-5 transition-transform duration-300 group-hover:translate-x-1", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M13 7l5 5m0 0l-5 5m5-5H6" }
                                    }
                                }
                            }
                        }
                    }

                    // Footer Link
                    div { class: "text-center mt-8 relative z-10",
                        span { class: "text-[15px] text-[#86868B]", "已有账号？" }
                        Link {
                            to: Route::LoginPage {},
                            class: "text-[15px] font-medium text-[#5856D6] hover:text-[#6E6AE8] transition-colors ml-1",
                            "返回登录"
                        }
                    }
                }

                // Bottom branding
                div { class: "text-center mt-8",
                    span { class: "text-[13px] font-medium text-[#86868B]/60 tracking-wider",
                        "BurnCloud"
                    }
                }
            }
        }
    }
}
