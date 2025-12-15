use crate::app::Route;
use crate::components::logo::Logo;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

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
                    navigator.push(Route::Dashboard {});
                }
                Err(e) => {
                    loading.set(false);
                    println!("LoginPage: Login error: {}", e);
                    toast.error(&e);
                }
            }
        });
    };

    rsx! {
        // Container: Aurora Canvas
        div { class: "h-full w-full min-h-screen overflow-hidden bg-[#F5F5F7] text-[#1D1D1F] relative selection:bg-[#0071E3] selection:text-white font-sans flex items-center justify-center",

            // ========== BACKGROUND: Liquid Light Field ==========
            div { class: "absolute inset-0 pointer-events-none overflow-hidden",
                // Layer 1: Primary Aurora Blob
                div { class: "absolute top-[-20%] left-[-10%] w-[800px] h-[800px] bg-gradient-to-r from-[#FF2D55]/12 via-[#AF52DE]/10 to-[#007AFF]/12 rounded-full blur-[100px] animate-aurora animate-morph" }

                // Layer 2: Secondary Flow
                div { class: "absolute bottom-[-15%] right-[-10%] w-[700px] h-[700px] bg-gradient-to-l from-[#30B0C7]/15 via-[#5856D6]/12 to-transparent rounded-full blur-[80px] animate-aurora [animation-delay:7s] [animation-duration:25s]" }

                // Layer 3: Accent Orb
                div { class: "absolute top-[20%] right-[20%] w-[300px] h-[300px] bg-gradient-to-br from-[#5AC8FA]/15 to-[#007AFF]/8 rounded-full blur-[60px] animate-float [animation-delay:2s]" }

                // Grid pattern overlay
                div {
                    class: "absolute inset-0 opacity-[0.02]",
                    style: "background-image: radial-gradient(circle at 1px 1px, #1D1D1F 1px, transparent 0); background-size: 40px 40px;"
                }
            }

            // ========== LOGIN CARD: Glass Morphism ==========
            div { class: "relative z-10 w-full max-w-[420px] mx-4 animate-slide-up",

                // Glass Card
                div { class: "backdrop-blur-xl bg-white/70 rounded-[32px] shadow-[0_30px_60px_-12px_rgba(0,0,0,0.12)] border border-white/50 p-10 relative overflow-hidden",

                    // Glossy reflection
                    div { class: "absolute top-0 right-0 w-48 h-48 bg-gradient-to-br from-white/80 to-transparent opacity-60 pointer-events-none rounded-full blur-2xl -translate-y-1/2 translate-x-1/2" }

                    // Logo & Header
                    div { class: "text-center mb-10 relative z-10",
                        // Logo
                        div { class: "inline-flex items-center justify-center mb-6 transition-transform duration-500 hover:scale-110 hover:rotate-6",
                            Logo { class: "w-16 h-16 fill-current" }
                        }

                        // Header Slogan
                        div { class: "flex flex-col items-center justify-center space-y-1 mb-4",
                            h1 { class: "text-2xl font-semibold tracking-tight text-[#1D1D1F]",
                                "One Interface."
                            }
                            h1 { class: "text-2xl font-semibold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-[#007AFF] to-[#5856D6]",
                                "Every Intelligence."
                            }
                        }
                        p { class: "text-[15px] text-[#86868B]",
                            "登录以连接您的本地算力节点"
                        }
                    }

                    // Form
                    div { class: "space-y-5 relative z-10",
                        // Username Input
                        div { class: "group",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider",
                                "用户名"
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#007AFF] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" }
                                    }
                                }
                                input {
                                    class: "w-full h-14 pl-12 pr-4 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#007AFF] focus:ring-4 focus:ring-[#007AFF]/10 focus:bg-white",
                                    r#type: "text",
                                    value: "{username}",
                                    placeholder: "请输入用户名",
                                    oninput: move |e| username.set(e.value())
                                }
                            }
                        }

                        // Password Input
                        div { class: "group",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider",
                                "密码"
                            }
                            div { class: "relative",
                                div { class: "absolute left-4 top-1/2 -translate-y-1/2 text-[#86868B] group-focus-within:text-[#007AFF] transition-colors",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" }
                                    }
                                }
                                input {
                                    class: "w-full h-14 pl-12 pr-4 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#007AFF] focus:ring-4 focus:ring-[#007AFF]/10 focus:bg-white",
                                    r#type: "password",
                                    value: "{password}",
                                    placeholder: "请输入密码",
                                    oninput: move |e| password.set(e.value())
                                }
                            }
                        }

                        // Login Button - Gradient CTA
                        button {
                            class: "group relative w-full h-14 mt-8 text-[17px] font-semibold text-white transition-all duration-500 bg-gradient-to-r from-[#0071E3] to-[#5856D6] rounded-2xl hover:from-[#0077ED] hover:to-[#6E6AE8] shadow-[0_10px_30px_-5px_rgba(0,113,227,0.4)] hover:shadow-[0_20px_40px_-5px_rgba(0,113,227,0.5)] hover:scale-[1.02] active:scale-[0.98] disabled:opacity-60 disabled:cursor-not-allowed disabled:hover:scale-100 overflow-hidden",
                            disabled: loading(),
                            onclick: handle_login,

                            // Shimmer effect
                            div { class: "absolute inset-0 animate-shimmer opacity-0 group-hover:opacity-100 transition-opacity duration-300" }

                            span { class: "relative z-10 flex items-center justify-center gap-2",
                                if loading() {
                                    // Loading spinner
                                    svg { class: "w-5 h-5 animate-spin", fill: "none", view_box: "0 0 24 24",
                                        circle { class: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", stroke_width: "4" }
                                        path { class: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" }
                                    }
                                    "登录中..."
                                } else {
                                    "登录"
                                    svg { class: "w-5 h-5 transition-transform duration-300 group-hover:translate-x-1", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M9 5l7 7-7 7" }
                                    }
                                }
                            }
                        }
                    }

                    // Footer Link
                    div { class: "text-center mt-8 relative z-10",
                        span { class: "text-[15px] text-[#86868B]", "还没有账号？" }
                        Link {
                            to: Route::RegisterPage {},
                            class: "text-[15px] font-medium text-[#007AFF] hover:text-[#0077ED] transition-colors ml-1",
                            "立即注册"
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
