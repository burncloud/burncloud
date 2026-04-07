use crate::app::Route;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::components::bc_button::{BCButton, ButtonVariant};
use burncloud_client_shared::components::logo::Logo;
use burncloud_client_shared::use_toast;
use burncloud_client_shared::utils::storage::ClientState;
use burncloud_client_shared::{use_auth, CurrentUser};
use dioxus::prelude::*;

#[component]
pub fn LoginPage() -> Element {
    // Load persisted state
    let state = ClientState::load();
    let mut username = use_signal(|| state.last_username.unwrap_or_default());
    let mut password = use_signal(|| String::new());
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

        // Capture for async and save
        let u = username();
        let p = password();

        spawn(async move {
            match AuthService::login(&u, &p).await {
                Ok(response) => {
                    loading.set(false);

                    // Save credentials and token on success
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
                    eprintln!("LoginPage: Login error: {}", e);
                    // toast.error(&e); // Use inline error instead
                    login_error.set(Some("用户名或密码错误".to_string()));
                }
            }
        });
    };

    rsx! {
        // Container: Aurora Canvas
        div { class: "h-full w-full min-h-screen overflow-hidden bg-[var(--bc-bg-canvas)] text-[var(--bc-text-primary)] relative selection:bg-[var(--bc-primary)] selection:text-[var(--bc-text-on-accent)] font-sans flex items-center justify-center py-12",

            // ========== BACKGROUND: Liquid Light Field ==========
            div { class: "absolute inset-0 pointer-events-none overflow-hidden",
                // Draggable Region
                div { class: "absolute top-0 left-0 w-full h-16 z-50 cursor-default", style: "-webkit-app-region: drag;" }

                // Layer 1: Primary Aurora Blob - slower
                div { class: "absolute top-[-15%] right-[-15%] w-[900px] h-[900px] bg-gradient-to-l from-[var(--bc-primary-dark)]/15 via-[#AF52DE]/12 to-[var(--bc-primary)]/10 rounded-full blur-[100px] animate-aurora animate-morph [animation-duration:30s]" }

                // Layer 2: Secondary Flow - slower
                div { class: "absolute bottom-[-20%] left-[-10%] w-[800px] h-[800px] bg-gradient-to-r from-[var(--bc-success)]/12 via-[#30B0C7]/10 to-transparent rounded-full blur-[80px] animate-aurora [animation-delay:5s] [animation-duration:40s]" }

                // Layer 3: Accent Orb - slower
                div { class: "absolute top-[30%] left-[15%] w-[350px] h-[350px] bg-gradient-to-br from-[var(--bc-warning)]/10 to-[var(--bc-danger)]/8 rounded-full blur-[60px] animate-float [animation-delay:3s] [animation-duration:20s]" }

                // Noise Texture (Removed quotes from url() to avoid string escaping issues)
                div {
                    class: "absolute inset-0 opacity-[0.03] mix-blend-overlay",
                    style: "background-image: url(data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E);"
                }
            }

            // ========== LOGIN CONTAINER (Transparent) ==========
            div { class: "relative z-10 w-full mx-4 animate-in login-card-container",

                div { class: "p-8 relative bg-white/10 backdrop-blur-2xl border border-white/20 rounded-[var(--bc-radius-xl)] shadow-[0_8px_32px_rgba(0,0,0,0.08)]",

                    // Logo & Header
                    div { class: "text-center mb-8 relative z-10",
                        // Logo (Force Field)
                        div { class: "relative inline-flex items-center justify-center w-20 h-20 {logo_margin}",
                            div {
                                class: "w-full h-full rounded-full bg-white/20 border border-white/30 backdrop-blur-sm shadow-[0_8px_30px_-6px_rgba(88,86,214,0.12)] flex items-center justify-center",
                                Logo { class: "w-9 h-9 text-[var(--bc-primary-dark)] fill-current translate-y-0.5" }
                            }
                        }

                        // Header Slogan
                        div { class: "flex flex-col items-center justify-center space-y-2 mb-6",
                            h1 { class: "text-2xl font-semibold tracking-tight text-[var(--bc-text-primary)]",
                                "Unleash Intelligence."
                            }
                            p { class: "text-2xl font-semibold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-[var(--bc-primary-dark)] to-[#AF52DE]",
                                "Your Second Brain."
                            }
                        }
                        p { class: "text-[15px] text-[var(--bc-text-primary)]/60 font-semibold tracking-wide",
                            "登录以连接您的本地算力节点"
                        }
                    }

                    // Form
                    div { class: "space-y-4 relative z-10",
                        // Username Input
                        div { class: "group relative",
                            label { class: "block text-caption font-medium text-secondary mb-sm uppercase tracking-wider ml-1", "用户名" }
                            div {
                                class: "relative flex items-center w-full h-12 bg-[var(--bc-bg-input)] shadow-[var(--bc-shadow-xs)] rounded-[var(--bc-radius-md)] transition-all duration-200 focus-within:shadow-[0_0_0_2px_rgba(88,86,214,0.3)] focus-within:bg-[var(--bc-bg-card-solid)] hover:bg-[var(--bc-bg-card-solid)]",
                                div { class: "pl-lg pr-sm text-secondary group-focus-within:text-[var(--bc-primary)] group-focus-within:scale-110 transition-all duration-300 flex-shrink-0 origin-center",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" }
                                    }
                                }
                                input {
                                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-[var(--bc-primary)] px-2 text-body text-primary placeholder-text-tertiary",
                                    r#type: "text",
                                    value: "{username}",
                                    placeholder: "请输入用户名",
                                    autofocus: true,
                                    oninput: move |e: FormEvent| {
                                        username.set(e.value());
                                        login_error.set(None);
                                    }
                                }
                            }
                        }

                        // Password Input
                        div { class: "group relative",
                            label { class: "block text-caption font-medium text-secondary mb-sm uppercase tracking-wider ml-1", "密码" }
                            div {
                                class: if login_error().is_some() {
                                    "relative flex items-center w-full h-12 bg-[var(--bc-bg-input)] shadow-[var(--bc-shadow-xs)] rounded-[var(--bc-radius-md)] transition-all duration-200 shadow-[0_0_0_2px_rgba(255,59,48,0.3)] bg-[var(--bc-bg-card-solid)]"
                                } else {
                                    "relative flex items-center w-full h-12 bg-[var(--bc-bg-input)] shadow-[var(--bc-shadow-xs)] rounded-[var(--bc-radius-md)] transition-all duration-200 focus-within:shadow-[0_0_0_2px_rgba(88,86,214,0.3)] focus-within:bg-[var(--bc-bg-card-solid)] hover:bg-[var(--bc-bg-card-solid)]"
                                },
                                div { class: "pl-lg pr-sm text-secondary group-focus-within:text-[var(--bc-primary)] group-focus-within:scale-110 transition-all duration-300 flex-shrink-0 origin-center",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" }
                                    }
                                }
                                input {
                                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-[var(--bc-primary)] px-2 text-body text-primary placeholder-text-tertiary",
                                    r#type: "password",
                                    value: "{password}",
                                    placeholder: "请输入密码",
                                    oninput: move |e: FormEvent| {
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
                                div { class: "w-1.5 h-1.5 rounded-full bg-[var(--bc-danger)] shadow-[0_0_8px_rgba(255,59,48,0.8)] translate-y-[0.5px]" }
                                span { class: "text-caption font-medium text-[var(--bc-danger)] opacity-90",
                                    "{login_error().unwrap_or_default()}"
                                }
                            }
                        }

                        // Login Button
                        BCButton {
                            variant: ButtonVariant::Gradient,
                            class: "mt-6 w-full h-12",
                            loading: loading(),
                            disabled: loading(),
                            onclick: handle_login,

                            if loading() {
                                "登录中..."
                            } else {
                                "登录"
                                svg { class: "w-5 h-5 transition-transform duration-300 group-hover:translate-x-1", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M13 7l5 5m0 0l-5 5m5-5H6" }
                                }
                            }
                        }
                    }

                    // Footer Link
                    div { class: "text-center mt-8 relative z-10",
                        Link {
                            to: Route::RegisterPage {},
                            class: "text-[15px] font-medium text-[var(--bc-text-secondary)] hover:text-[var(--bc-text-primary)] transition-colors",
                            "还没有账号? 立即注册"
                        }
                    }
                }
            }
        }
    }
}
