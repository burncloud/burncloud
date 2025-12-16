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

    let label_class = "block text-[13px] font-medium text-[#7B7B80] mb-1.5 uppercase tracking-wider transition-all duration-200 group-focus-within:text-[#5856D6]";
    let input_class = "w-full h-13 pl-12 pr-4 rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#B0B0B5] bg-white/70 backdrop-blur-xl border border-white/40 shadow-[0_12px_40px_-22px_rgba(88,86,214,0.25),inset_0_1px_0_rgba(255,255,255,0.7)] transition-all duration-300 focus:outline-none focus:border-transparent focus:ring-2 focus:ring-[#FF7A45]/50 focus:shadow-[0_18px_50px_-24px_rgba(255,122,69,0.35),0_14px_36px_-26px_rgba(88,86,214,0.45)] focus:bg-white/80";

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
        div { class: "h-full w-full min-h-screen overflow-hidden bg-gradient-to-br from-[#100c1a] via-[#0f1626] to-[#1a0f16] text-[#1D1D1F] relative selection:bg-[#FF7A45] selection:text-white font-sans flex items-center justify-center py-12",

            // ========== BACKGROUND: Liquid Light Field ==========
            div { class: "absolute inset-0 pointer-events-none overflow-hidden",
                // Layer 1: Primary Aurora Blob - shifted for variety
                div { class: "absolute top-[-15%] right-[-15%] w-[900px] h-[900px] bg-gradient-to-l from-[#FF7A45]/14 via-[#AF52DE]/12 to-[#007AFF]/10 rounded-full blur-[110px] animate-aurora animate-morph" }

                // Layer 2: Secondary Flow
                div { class: "absolute bottom-[-20%] left-[-10%] w-[800px] h-[800px] bg-gradient-to-r from-[#FF4D79]/12 via-[#30B0C7]/10 to-transparent rounded-full blur-[90px] animate-aurora [animation-delay:5s] [animation-duration:22s]" }

                // Layer 3: Accent Orb
                div { class: "absolute top-[30%] left-[15%] w-[350px] h-[350px] bg-gradient-to-br from-[#FF9500]/12 to-[#FF2D55]/10 rounded-full blur-[70px] animate-float [animation-delay:3s]" }

                // Grid pattern overlay - reduced opacity for cleaner background
                div {
                    class: "absolute inset-0 opacity-[0.015]",
                    style: "background-image: radial-gradient(circle at 1px 1px, #1D1D1F 0.5px, transparent 0); background-size: 48px 48px;"
                }
            }

            // ========== REGISTER CARD: Glass Morphism ==========
            div { class: "relative z-10 w-full max-w-[360px] md:max-w-[420px] mx-4 animate-in",

                // Glass Card - refined aesthetics with sophisticated shadow
                div { class: "backdrop-blur-2xl bg-white/75 rounded-[24px] shadow-[0_8px_32px_-4px_rgba(0,0,0,0.08),0_24px_56px_-8px_rgba(88,86,214,0.14)] border border-white/60 p-8 relative overflow-hidden",

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
                                "Ignite."
                            }
                            h1 { class: "text-2xl font-semibold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-[#FF7A45] via-[#FF5E8A] to-[#7B61FF]",
                                "Your Gateway to Intelligence"
                            }
                        }
                        p { class: "text-[15px] text-[#6E6E73] font-medium",
                            "点燃本地优先的 AI 体验"
                        }
                    }

                    // Form
                    div { class: "space-y-3.5 relative z-10",
                        // Username Input
                        div { class: "group",
                            label { class: "{label_class}",
                                "用户名"
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#5856D6] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" }
                                    }
                                }
                                input {
                                    class: "{input_class}",
                                    r#type: "text",
                                    value: "{username}",
                                    placeholder: "设置您的唯一标识",
                                    oninput: move |e| username.set(e.value())
                                }
                            }
                        }

                        // Email Input (Optional)
                        div { class: "group",
                            label { class: "{label_class}",
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
                                    class: "{input_class}",
                                    r#type: "email",
                                    value: "{email}",
                                    placeholder: "保持联系，获取最新火花",
                                    oninput: move |e| email.set(e.value())
                                }
                            }
                        }

                        // Password Input
                        div { class: "group",
                            label { class: "{label_class}",
                                "密码"
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#5856D6] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" }
                                    }
                                }
                                input {
                                    class: "{input_class}",
                                    r#type: "password",
                                    value: "{password}",
                                    placeholder: "设置强密码",
                                    oninput: move |e| password.set(e.value())
                                }
                            }
                        }

                        // Confirm Password Input
                        div { class: "group",
                            label { class: "{label_class}",
                                "确认密码"
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#5856D6] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                                    }
                                }
                                input {
                                    class: "{input_class}",
                                    r#type: "password",
                                    value: "{confirm_password}",
                                    placeholder: "再次输入密码",
                                    oninput: move |e| confirm_password.set(e.value())
                                }
                            }
                        }

                        // Register Button - Primary Action with high visual weight
                        button {
                            class: "group relative w-full h-14 mt-6 text-[17px] font-semibold text-white transition-all duration-300 bg-gradient-to-r from-[#FF7A45] via-[#FF4D79] to-[#7B61FF] rounded-2xl shadow-[0_16px_48px_-20px_rgba(255,74,117,0.65),0_18px_52px_-26px_rgba(123,97,255,0.45)] hover:shadow-[0_18px_52px_-18px_rgba(255,122,69,0.7),0_24px_60px_-26px_rgba(123,97,255,0.55)] hover:scale-[1.02] hover:-translate-y-0.5 active:scale-[0.99] active:translate-y-0 disabled:opacity-60 disabled:cursor-not-allowed disabled:hover:scale-100 disabled:hover:translate-y-0 overflow-hidden backdrop-blur-lg border border-white/10",
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
                    span { class: "text-[13px] font-medium text-white/70 tracking-wider",
                        "BurnCloud"
                    }
                }
            }
        }
    }
}
