use crate::app::Route;
use crate::components::logo::Logo;
use burncloud_client_shared::auth_context::use_auth;
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::use_toast;
use burncloud_client_shared::utils::{
    calculate_password_strength, get_email_error, get_password_error, get_username_error,
    sanitize_input, PasswordStrength,
};
use dioxus::prelude::*;
use std::time::Duration;

#[component]
pub fn RegisterPage() -> Element {
    let mut username = use_signal(|| "".to_string());
    let mut password = use_signal(|| "".to_string());
    let mut confirm_password = use_signal(|| "".to_string());
    let mut email = use_signal(|| "".to_string());
    let mut loading = use_signal(|| false);
    let mut show_password = use_signal(|| false);
    let mut show_confirm_password = use_signal(|| false);
    let mut username_available = use_signal(|| None::<bool>);
    let mut username_checking = use_signal(|| false);
    let mut shake_form = use_signal(|| false);
    
    // Validation errors
    let mut username_error = use_signal(|| None::<String>);
    let mut email_error = use_signal(|| None::<String>);
    let mut password_error = use_signal(|| None::<String>);
    let mut confirm_error = use_signal(|| None::<String>);
    
    // Debounced validation
    let mut validation_timer = use_signal(|| None::<i32>);

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

    // Calculate password strength
    let password_strength = calculate_password_strength(&password());
    let passwords_match = !password().is_empty() 
        && !confirm_password().is_empty() 
        && password() == confirm_password();

    // Check if form is valid
    let form_valid = !username().is_empty()
        && !password().is_empty()
        && !confirm_password().is_empty()
        && username_error().is_none()
        && email_error().is_none()
        && password_error().is_none()
        && passwords_match
        && username_available() != Some(false);

    // Auto-focus first input
    use_effect(move || {
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            // Note: Auto-focus would need browser API, simplified for now
        });
    });

    let handle_username_change = move |e: Event<FormData>| {
        let value = e.value();
        let sanitized = sanitize_input(&value);
        username.set(sanitized.clone());
        
        // Validate username format
        username_error.set(get_username_error(&sanitized));
        
        // Reset availability check
        username_available.set(None);
        
        // Debounced availability check
        if sanitized.len() >= 3 && get_username_error(&sanitized).is_none() {
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                username_checking.set(true);
                match AuthService::check_username_availability(&sanitized).await {
                    Ok(available) => {
                        username_available.set(Some(available));
                        if !available {
                            username_error.set(Some("用户名已被占用".to_string()));
                        }
                    }
                    Err(_) => {
                        // Silently fail availability check
                    }
                }
                username_checking.set(false);
            });
        }
    };

    let handle_email_change = move |e: Event<FormData>| {
        let value = e.value();
        let sanitized = sanitize_input(&value);
        email.set(sanitized.clone());
        
        // Debounced validation
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(300)).await;
            email_error.set(get_email_error(&sanitized));
        });
    };

    let handle_password_change = move |e: Event<FormData>| {
        let value = e.value();
        password.set(value.clone());
        
        // Validate password
        password_error.set(get_password_error(&value));
        
        // Check confirm password match
        if !confirm_password().is_empty() && value != confirm_password() {
            confirm_error.set(Some("两次输入的密码不一致".to_string()));
        } else {
            confirm_error.set(None);
        }
    };

    let handle_confirm_password_change = move |e: Event<FormData>| {
        let value = e.value();
        confirm_password.set(value.clone());
        
        // Check password match
        if !password().is_empty() && value != password() {
            confirm_error.set(Some("两次输入的密码不一致".to_string()));
        } else {
            confirm_error.set(None);
        }
    };

    let handle_register = move |_| {
        // Final validation
        if !form_valid {
            shake_form.set(true);
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(500)).await;
                shake_form.set(false);
            });
            toast.error("请检查表单填写是否正确");
            return;
        }

        loading.set(true);
        spawn(async move {
            let email_val = email();
            let email_opt = if email_val.is_empty() {
                None
            } else {
                Some(email_val.as_str())
            };

            match AuthService::register(&username(), &password(), email_opt).await {
                Ok(login_response) => {
                    // Auto-login: Save auth data
                    auth.login(
                        login_response.username.clone(),
                        login_response.id.clone(),
                        login_response.token.clone(),
                    );
                    
                    toast.success("注册成功！欢迎使用 BurnCloud");
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    loading.set(false);
                    
                    // Redirect to dashboard instead of login
                    navigator.push(Route::DashboardPage {});
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

    // Handle Enter key submission
    let handle_keydown = move |e: Event<KeyboardData>| {
        if e.key() == Key::Enter && form_valid && !loading() {
            handle_register(());
        }
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

                    // Form with shake animation
                    div {
                        class: if shake_form() {
                            "space-y-4 relative z-10 animate-shake"
                        } else {
                            "space-y-4 relative z-10"
                        },
                        onkeydown: handle_keydown,

                        // Username Input with availability check
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
                                // Availability indicator
                                if username_checking() {
                                    div { class: "absolute right-4 top-1/2 -translate-y-1/2",
                                        svg { class: "w-5 h-5 animate-spin text-[#5856D6]", fill: "none", view_box: "0 0 24 24",
                                            circle { class: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", stroke_width: "4" }
                                            path { class: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" }
                                        }
                                    }
                                } else if username_available() == Some(true) && username_error().is_none() {
                                    div { class: "absolute right-4 top-1/2 -translate-y-1/2 text-green-500",
                                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" }
                                        }
                                    }
                                }
                                input {
                                    class: if username_error().is_some() {
                                        "w-full h-13 pl-12 pr-12 bg-white/60 border border-red-500/50 rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-red-500 focus:ring-4 focus:ring-red-500/10 focus:bg-white"
                                    } else {
                                        "w-full h-13 pl-12 pr-12 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#5856D6] focus:ring-4 focus:ring-[#5856D6]/10 focus:bg-white"
                                    },
                                    r#type: "text",
                                    value: "{username}",
                                    placeholder: "设置您的唯一标识",
                                    autofocus: true,
                                    tabindex: "1",
                                    oninput: handle_username_change
                                }
                            }
                            if let Some(err) = username_error() {
                                div { class: "mt-1.5 text-[12px] text-red-500 font-medium pl-1",
                                    "{err}"
                                }
                            }
                        }

                        // Email Input with validation
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
                                    class: if email_error().is_some() {
                                        "w-full h-13 pl-12 pr-4 bg-white/60 border border-red-500/50 rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-red-500 focus:ring-4 focus:ring-red-500/10 focus:bg-white"
                                    } else {
                                        "w-full h-13 pl-12 pr-4 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#5856D6] focus:ring-4 focus:ring-[#5856D6]/10 focus:bg-white"
                                    },
                                    r#type: "email",
                                    value: "{email}",
                                    placeholder: "用于接收通知",
                                    tabindex: "2",
                                    oninput: handle_email_change
                                }
                            }
                            if let Some(err) = email_error() {
                                div { class: "mt-1.5 text-[12px] text-red-500 font-medium pl-1",
                                    "{err}"
                                }
                            }
                        }

                        // Password Input with strength meter and visibility toggle
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
                                // Visibility toggle
                                button {
                                    class: "absolute right-4 top-1/2 -translate-y-1/2 text-[#86868B] hover:text-[#5856D6] transition-colors",
                                    r#type: "button",
                                    tabindex: "-1",
                                    onclick: move |_| show_password.set(!show_password()),
                                    if show_password() {
                                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" }
                                        }
                                    } else {
                                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" }
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" }
                                        }
                                    }
                                }
                                input {
                                    class: if password_error().is_some() {
                                        "w-full h-13 pl-12 pr-12 bg-white/60 border border-red-500/50 rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-red-500 focus:ring-4 focus:ring-red-500/10 focus:bg-white"
                                    } else {
                                        "w-full h-13 pl-12 pr-12 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#5856D6] focus:ring-4 focus:ring-[#5856D6]/10 focus:bg-white"
                                    },
                                    r#type: if show_password() { "text" } else { "password" },
                                    value: "{password}",
                                    placeholder: "设置强密码",
                                    tabindex: "3",
                                    oninput: handle_password_change
                                }
                            }
                            // Password strength meter
                            if !password().is_empty() {
                                div { class: "mt-2 flex items-center gap-2",
                                    div { class: "flex-1 h-1.5 bg-gray-200 rounded-full overflow-hidden",
                                        div {
                                            class: "h-full transition-all duration-300 {password_strength.color_class()} {password_strength.width_class()}"
                                        }
                                    }
                                    span { class: "text-[11px] font-medium text-[#86868B]",
                                        "强度: {password_strength.as_str()}"
                                    }
                                }
                            }
                            if let Some(err) = password_error() {
                                div { class: "mt-1.5 text-[12px] text-red-500 font-medium pl-1",
                                    "{err}"
                                }
                            }
                        }

                        // Confirm Password Input with match indicator
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
                                // Match indicator
                                if passwords_match {
                                    div { class: "absolute right-4 top-1/2 -translate-y-1/2 text-green-500",
                                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" }
                                        }
                                    }
                                }
                                // Visibility toggle
                                button {
                                    class: if passwords_match {
                                        "absolute right-12 top-1/2 -translate-y-1/2 text-[#86868B] hover:text-[#5856D6] transition-colors"
                                    } else {
                                        "absolute right-4 top-1/2 -translate-y-1/2 text-[#86868B] hover:text-[#5856D6] transition-colors"
                                    },
                                    r#type: "button",
                                    tabindex: "-1",
                                    onclick: move |_| show_confirm_password.set(!show_confirm_password()),
                                    if show_confirm_password() {
                                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" }
                                        }
                                    } else {
                                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M2.036 12.322a1.012 1.012 0 010-.639C3.423 7.51 7.36 4.5 12 4.5c4.638 0 8.573 3.007 9.963 7.178.07.207.07.431 0 .639C20.577 16.49 16.64 19.5 12 19.5c-4.638 0-8.573-3.007-9.963-7.178z" }
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15 12a3 3 0 11-6 0 3 3 0 016 0z" }
                                        }
                                    }
                                }
                                input {
                                    class: if confirm_error().is_some() {
                                        "w-full h-13 pl-12 pr-12 bg-white/60 border border-red-500/50 rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-red-500 focus:ring-4 focus:ring-red-500/10 focus:bg-white"
                                    } else {
                                        "w-full h-13 pl-12 pr-12 bg-white/60 border border-[#E5E5EA] rounded-2xl text-[16px] text-[#1D1D1F] placeholder-[#C7C7CC] transition-all duration-300 focus:outline-none focus:border-[#5856D6] focus:ring-4 focus:ring-[#5856D6]/10 focus:bg-white"
                                    },
                                    r#type: if show_confirm_password() { "text" } else { "password" },
                                    value: "{confirm_password}",
                                    placeholder: "再次输入密码",
                                    tabindex: "4",
                                    oninput: handle_confirm_password_change
                                }
                            }
                            if let Some(err) = confirm_error() {
                                div { class: "mt-1.5 text-[12px] text-red-500 font-medium pl-1",
                                    "{err}"
                                }
                            }
                        }

                        // Register Button - Primary Action with high visual weight
                        button {
                            class: "group relative w-full h-14 mt-6 text-[17px] font-semibold text-white transition-all duration-300 bg-[#1D1D1F] rounded-2xl hover:bg-[#2C2C2E] shadow-[0_4px_14px_-2px_rgba(29,29,31,0.25),0_12px_32px_-4px_rgba(29,29,31,0.15)] hover:shadow-[0_8px_24px_-4px_rgba(29,29,31,0.35),0_16px_40px_-8px_rgba(29,29,31,0.2)] hover:scale-[1.015] hover:-translate-y-0.5 active:scale-[0.985] active:translate-y-0 disabled:opacity-60 disabled:cursor-not-allowed disabled:hover:scale-100 disabled:hover:translate-y-0 overflow-hidden",
                            disabled: loading() || !form_valid,
                            tabindex: "5",
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
