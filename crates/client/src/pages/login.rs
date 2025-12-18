use crate::app::Route;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::components::logo::Logo;
use burncloud_client_shared::use_toast;
use burncloud_client_shared::{use_auth, CurrentUser};
use dioxus::prelude::*;

#[component]
pub fn LoginPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut loading = use_signal(|| false);
    let mut login_error = use_signal(|| None::<String>);
    let toast = use_toast();
    let navigator = use_navigator();
    let auth = use_auth();

    let logo_margin = if cfg!(feature = "liveview") {
        "mb-6"
    } else if cfg!(feature = "desktop") {
        "mb-10"
    } else {
        "mb-6"
    };

    let handle_login = move |_| {
        loading.set(true);
        login_error.set(None);
        spawn(async move {
            match AuthService::login(&username(), &password()).await {
                Ok(response) => {
                    loading.set(false);
                    // Store auth state in context
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
                    println!("LoginPage: Login error: {}", e);
                    // toast.error(&e); // Use inline error instead
                    login_error.set(Some("用户名或密码错误".to_string()));
                }
            }
        });
    };

    rsx! {
        // Container: Aurora Canvas
        div { class: "h-full w-full min-h-screen overflow-hidden bg-[#F5F5F7] text-[#1D1D1F] relative selection:bg-[#0071E3] selection:text-white font-sans flex items-center justify-center py-12",

            // ========== BACKGROUND: Liquid Light Field ==========
            div { class: "absolute inset-0 pointer-events-none overflow-hidden",
                // Draggable Region
                div { class: "absolute top-0 left-0 w-full h-16 z-50 cursor-default", style: "-webkit-app-region: drag;" }

                // Layer 1: Primary Aurora Blob - slower
                div { class: "absolute top-[-15%] right-[-15%] w-[900px] h-[900px] bg-gradient-to-l from-[#5856D6]/15 via-[#AF52DE]/12 to-[#007AFF]/10 rounded-full blur-[100px] animate-aurora animate-morph [animation-duration:30s]" }

                // Layer 2: Secondary Flow - slower
                div { class: "absolute bottom-[-20%] left-[-10%] w-[800px] h-[800px] bg-gradient-to-r from-[#34C759]/12 via-[#30B0C7]/10 to-transparent rounded-full blur-[80px] animate-aurora [animation-delay:5s] [animation-duration:40s]" }

                // Layer 3: Accent Orb - slower
                div { class: "absolute top-[30%] left-[15%] w-[350px] h-[350px] bg-gradient-to-br from-[#FF9500]/10 to-[#FF2D55]/8 rounded-full blur-[60px] animate-float [animation-delay:3s] [animation-duration:20s]" }

                // Noise Texture (Removed quotes from url() to avoid string escaping issues)
                div {
                    class: "absolute inset-0 opacity-[0.03] mix-blend-overlay",
                    style: "background-image: url(data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E);"
                }
            }

            // ========== LOGIN CONTAINER (Transparent) ==========
            div { class: "relative z-10 w-full max-w-[400px] mx-4 animate-in",

                div { class: "p-8 relative",

                    // Logo & Header
                    div { class: "text-center mb-8 relative z-10",
                        // Logo (Force Field)
                        div { class: "relative inline-flex items-center justify-center w-24 h-24 {logo_margin}",
                            div {
                                class: "w-full h-full rounded-full bg-white/20 border border-white/30 backdrop-blur-sm shadow-[0_8px_30px_-6px_rgba(88,86,214,0.12)] flex items-center justify-center",
                                Logo { class: "w-10 h-10 text-[#5856D6] fill-current translate-y-0.5" }
                            }
                        }

                        // Header Slogan
                        div { class: "flex flex-col items-center justify-center space-y-2 mb-6",
                            h1 { class: "text-2xl font-semibold tracking-tight text-[#1D1D1F]",
                                "Unleash Intelligence."
                            }
                            h1 { class: "text-2xl font-semibold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-[#5856D6] to-[#AF52DE]",
                                "Your Second Brain."
                            }
                        }
                        p { class: "text-[15px] text-[#1D1D1F]/60 font-semibold tracking-wide",
                            "登录以连接您的本地算力节点"
                        }
                    }

                    // Form
                    div { class: "space-y-4 relative z-10",
                        // Username Input
                        div { class: "group relative",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider ml-1", "用户名" }
                            div {
                                class: "relative flex items-center w-full h-12 bg-white/90 shadow-sm rounded-xl transition-all duration-200 focus-within:shadow-[0_0_0_2px_rgba(88,86,214,0.3)] focus-within:bg-white hover:bg-white",
                                div { class: "pl-4 pr-1 text-[#86868B] group-focus-within:text-[#007AFF] group-focus-within:scale-110 transition-all duration-300 flex-shrink-0 origin-center",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" }
                                    }
                                }
                                input {
                                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-[#007AFF] px-2 text-[15px] text-[#1D1D1F] placeholder-[#86868B]",
                                    r#type: "text",
                                    value: "{username}",
                                    placeholder: "请输入用户名",
                                    autofocus: true,
                                    oninput: move |e| {
                                        username.set(e.value());
                                        login_error.set(None);
                                    }
                                }
                            }
                        }

                        // Password Input
                        div { class: "group relative",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider ml-1", "密码" }
                            div {
                                class: if login_error().is_some() {
                                    "relative flex items-center w-full h-12 bg-white/90 shadow-sm rounded-xl transition-all duration-200 shadow-[0_0_0_2px_rgba(255,59,48,0.3)] bg-white"
                                } else {
                                    "relative flex items-center w-full h-12 bg-white/90 shadow-sm rounded-xl transition-all duration-200 focus-within:shadow-[0_0_0_2px_rgba(88,86,214,0.3)] focus-within:bg-white hover:bg-white"
                                },
                                div { class: "pl-4 pr-1 text-[#86868B] group-focus-within:text-[#007AFF] group-focus-within:scale-110 transition-all duration-300 flex-shrink-0 origin-center",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" }
                                    }
                                }
                                input {
                                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-[#007AFF] px-2 text-[15px] text-[#1D1D1F] placeholder-[#86868B]",
                                    r#type: "password",
                                    value: "{password}",
                                    placeholder: "请输入密码",
                                    oninput: move |e| {
                                        password.set(e.value());
                                        login_error.set(None);
                                    }
                                }
                            }
                            // Floating Error Message
                            div {
                                class: if login_error().is_some() {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-100 z-20 pointer-events-none"
                                } else {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-0 z-20 pointer-events-none"
                                },
                                div { class: "w-1.5 h-1.5 rounded-full bg-[#FF3B30] shadow-[0_0_8px_rgba(255,59,48,0.8)] translate-y-[0.5px]" }
                                span { class: "text-[12px] text-[#FF3B30] font-medium opacity-90",
                                    "{login_error().unwrap_or_default()}"
                                }
                            }
                        }

                        // Login Button
                        button {
                            class: "group relative w-full h-12 mt-6 text-[16px] font-medium text-white transition-all duration-300 bg-gradient-to-r from-[#007AFF] to-[#5856D6] rounded-2xl shadow-[0_10px_30px_-10px_rgba(0,122,255,0.5)] hover:shadow-[0_20px_40px_-10px_rgba(0,122,255,0.6)] hover:scale-[1.015] hover:-translate-y-0.5 active:scale-[0.985] active:brightness-90 active:translate-y-0 disabled:opacity-60 disabled:cursor-not-allowed disabled:hover:scale-100 disabled:hover:translate-y-0 overflow-hidden",
                            disabled: loading(),
                            onclick: handle_login,

                            span { class: "relative z-10 flex items-center justify-center gap-2",
                                if loading() {
                                    svg { class: "w-5 h-5 animate-spin", fill: "none", view_box: "0 0 24 24",
                                        circle { class: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", stroke_width: "4" }
                                        path { class: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" }
                                    }
                                    "登录中..."
                                } else {
                                    "登录"
                                    svg { class: "w-5 h-5 transition-transform duration-300 group-hover:translate-x-1", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M13 7l5 5m0 0l-5 5m5-5H6" }
                                    }
                                }
                            }
                        }
                    }

                    // Footer Link
                    div { class: "text-center mt-8 relative z-10",
                        Link {
                            to: Route::RegisterPage {},
                            class: "text-[15px] font-medium text-[#86868B] hover:text-[#1D1D1F] transition-colors",
                            "还没有账号? 立即注册"
                        }
                    }
                }
            }
        }
    }
}
