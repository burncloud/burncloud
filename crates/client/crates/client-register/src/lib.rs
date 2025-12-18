use burncloud_client_shared::auth_context::{use_auth, CurrentUser};
use burncloud_client_shared::auth_service::AuthService;
use burncloud_client_shared::components::logo::Logo;
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
    // let mut validation_timer = use_signal(|| None::<i32>); // Unused

    let toast = use_toast();
    let navigator = use_navigator();
    let auth = use_auth();

    let logo_margin = "mb-10";

    // Calculate password strength
    let password_strength = calculate_password_strength(&password());
    let password_glow_class = if password().is_empty() {
        "focus-within:shadow-[0_0_0_2px_rgba(88,86,214,0.3)]"
    } else {
        match password_strength {
            PasswordStrength::Weak => "focus-within:shadow-[0_0_0_2px_rgba(255,59,48,0.4)]",
            PasswordStrength::Medium => "focus-within:shadow-[0_0_0_2px_rgba(255,149,0,0.5)]",
            PasswordStrength::Strong => "focus-within:shadow-[0_0_0_2px_rgba(52,199,89,0.5)]",
        }
    };

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

    let handle_register = move |_: Event<MouseData>| {
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

                    // Redirect to dashboard instead of login
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

    // Handle Enter key submission
    let handle_keydown = move |e: Event<KeyboardData>| {
        if e.key() == Key::Enter && form_valid && !loading() {
            // Manually call register logic since we can't easily construct MouseData
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

                        // Redirect to dashboard instead of login
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
        }
    };

    rsx! {
        // Container: Aurora Canvas
        div { class: "h-full w-full min-h-screen overflow-hidden bg-[#F5F5F7] text-[#1D1D1F] relative selection:bg-[#0071E3] selection:text-white font-sans flex items-center justify-center py-12",

            // ========== BACKGROUND: Liquid Light Field ==========
            div { class: "absolute inset-0 pointer-events-none overflow-hidden",
                // Draggable Region
                div { class: "absolute top-0 left-0 w-full h-16 z-50", style: "-webkit-app-region: drag;" }

                // Layer 1: Primary Aurora Blob - slower
                div { class: "absolute top-[-15%] right-[-15%] w-[900px] h-[900px] bg-gradient-to-l from-[#5856D6]/15 via-[#AF52DE]/12 to-[#007AFF]/10 rounded-full blur-[100px] animate-aurora animate-morph [animation-duration:30s]" }

                // Layer 2: Secondary Flow - slower
                div { class: "absolute bottom-[-20%] left-[-10%] w-[800px] h-[800px] bg-gradient-to-r from-[#34C759]/12 via-[#30B0C7]/10 to-transparent rounded-full blur-[80px] animate-aurora [animation-delay:5s] [animation-duration:40s]" }

                // Layer 3: Accent Orb - slower
                div { class: "absolute top-[30%] left-[15%] w-[350px] h-[350px] bg-gradient-to-br from-[#FF9500]/10 to-[#FF2D55]/8 rounded-full blur-[60px] animate-float [animation-delay:3s] [animation-duration:20s]" }

                // Noise Texture
                div {
                    class: "absolute inset-0 opacity-[0.03] mix-blend-overlay",
                    style: "background-image: url(\"data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.8' numOctaves='3' stitchTiles='stitch'/%3E%3C/filter%3E%3Crect width='100%25' height='100%25' filter='url(%23noiseFilter)'/%3E%3C/svg%3E\");"
                }
            }

            // ========== REGISTER CARD: Glass Morphism ==========
            div { class: "relative z-10 w-full max-w-[400px] mx-4 animate-in",

                // Glass Card - refined aesthetics with sophisticated shadow
                div { class: "backdrop-blur-xl bg-white/60 rounded-[24px] shadow-[0_8px_32px_-4px_rgba(0,0,0,0.08),0_24px_56px_-8px_rgba(88,86,214,0.12)] p-8 relative overflow-hidden",

                    // Glossy reflection
                    div { class: "absolute top-0 left-0 w-56 h-56 bg-gradient-to-br from-white/80 to-transparent opacity-60 pointer-events-none rounded-full blur-2xl -translate-y-1/2 -translate-x-1/2" }

                    // Logo & Header
                    div { class: "text-center mb-8 relative z-10",
                        // Logo (Simplified - Zen)
                        div { class: "relative inline-flex items-center justify-center w-16 h-16 {logo_margin}",
                            div { class: "w-full h-full rounded-[20px] bg-white shadow-[0_8px_24px_-4px_rgba(88,86,214,0.15)] flex items-center justify-center",
                                Logo { class: "w-8 h-8 text-[#5856D6] fill-current" }
                            }
                        }

                        // Header Slogan
                        div { class: "flex flex-col items-center justify-center space-y-1 mb-4",
                            h1 { class: "text-2xl font-semibold tracking-tight text-[#1D1D1F]",
                                "Unleash Intelligence."
                            }
                            h1 { class: "text-2xl font-semibold tracking-tight bg-clip-text text-transparent bg-gradient-to-r from-[#5856D6] to-[#AF52DE]",
                                "Your Second Brain."
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
                        div { class: "group relative",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider ml-1",
                                "用户名"
                            }
                            div {
                                class: if username_error().is_some() {
                                    "relative flex items-center w-full h-12 bg-[#F5F5F7]/80 rounded-xl transition-all duration-200 shadow-[0_0_0_2px_rgba(255,59,48,0.3)] bg-white"
                                } else {
                                    "relative flex items-center w-full h-12 bg-[#F5F5F7]/80 rounded-xl transition-all duration-200 focus-within:shadow-[0_0_0_2px_rgba(88,86,214,0.3)] focus-within:bg-white hover:bg-[#F5F5F7]"
                                },
                                div { class: "pl-4 pr-1 text-[#86868B] group-focus-within:text-[#007AFF] group-focus-within:scale-110 transition-all duration-300 flex-shrink-0 origin-center",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M15.75 6a3.75 3.75 0 11-7.5 0 3.75 3.75 0 017.5 0zM4.501 20.118a7.5 7.5 0 0114.998 0A17.933 17.933 0 0112 21.75c-2.676 0-5.216-.584-7.499-1.632z" }
                                    }
                                }
                                input {
                                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-[#007AFF] px-2 text-[15px] text-[#1D1D1F] placeholder-[#86868B]",
                                    r#type: "text",
                                    value: "{username}",
                                    placeholder: "设置您的唯一标识",
                                    autofocus: true,
                                    tabindex: "1",
                                    oninput: handle_username_change
                                }
                                // Availability indicator
                                if username_checking() {
                                    div { class: "pr-4 flex-shrink-0",
                                        svg { class: "w-5 h-5 animate-spin text-[#007AFF]", fill: "none", view_box: "0 0 24 24",
                                            circle { class: "opacity-25", cx: "12", cy: "12", r: "10", stroke: "currentColor", stroke_width: "4" }
                                            path { class: "opacity-75", fill: "currentColor", d: "M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" }
                                        }
                                    }
                                } else if username_available() == Some(true) && username_error().is_none() {
                                    div { class: "pr-4 text-green-500 flex-shrink-0",
                                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" }
                                        }
                                    }
                                }
                            }
                            div {
                                class: if username_error().is_some() {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-100 z-20 pointer-events-none"
                                } else {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-0 z-20 pointer-events-none"
                                },
                                div { class: "w-1.5 h-1.5 rounded-full bg-[#FF3B30] shadow-[0_0_8px_rgba(255,59,48,0.8)] translate-y-[0.5px]" }
                                span { class: "text-[12px] text-[#FF3B30] font-medium opacity-90",
                                    "{username_error().unwrap_or_else(|| \" \".to_string())}"
                                }
                            }
                        }

                        // Email Input with validation
                        div { class: "group relative",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider ml-1",
                                "邮箱"
                            }
                            div {
                                class: if email_error().is_some() {
                                    "relative flex items-center w-full h-12 bg-[#F5F5F7]/80 rounded-xl transition-all duration-200 shadow-[0_0_0_2px_rgba(255,59,48,0.3)] bg-white"
                                } else {
                                    "relative flex items-center w-full h-12 bg-[#F5F5F7]/80 rounded-xl transition-all duration-200 focus-within:shadow-[0_0_0_2px_rgba(88,86,214,0.3)] focus-within:bg-white hover:bg-[#F5F5F7]"
                                },
                                div { class: "pl-4 pr-1 text-[#86868B] group-focus-within:text-[#007AFF] group-focus-within:scale-110 transition-all duration-300 flex-shrink-0 origin-center",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M21.75 6.75v10.5a2.25 2.25 0 01-2.25 2.25h-15a2.25 2.25 0 01-2.25-2.25V6.75m19.5 0A2.25 2.25 0 0019.5 4.5h-15a2.25 2.25 0 00-2.25 2.25m19.5 0v.243a2.25 2.25 0 01-1.07 1.916l-7.5 4.615a2.25 2.25 0 01-2.36 0L3.32 8.91a2.25 2.25 0 01-1.07-1.916V6.75" }
                                    }
                                }
                                input {
                                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-[#007AFF] px-2 text-[15px] text-[#1D1D1F] placeholder-[#86868B]",
                                    r#type: "email",
                                    value: "{email}",
                                    placeholder: "用于接收通知 (可选)",
                                    tabindex: "2",
                                    oninput: handle_email_change
                                }
                            }
                            div {
                                class: if email_error().is_some() {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-100 z-20 pointer-events-none"
                                } else {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-0 z-20 pointer-events-none"
                                },
                                div { class: "w-1.5 h-1.5 rounded-full bg-[#FF3B30] shadow-[0_0_8px_rgba(255,59,48,0.8)] translate-y-[0.5px]" }
                                span { class: "text-[12px] text-[#FF3B30] font-medium opacity-90",
                                    "{email_error().unwrap_or_else(|| \" \".to_string())}"
                                }
                            }
                        }

                        // Password Input with strength meter and visibility toggle
                        div { class: "group relative",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider ml-1",
                                "密码"
                            }
                            div {
                                class: if password_error().is_some() {
                                    "relative flex items-center w-full h-12 bg-[#F5F5F7]/80 rounded-xl transition-all duration-200 shadow-[0_0_0_2px_rgba(255,59,48,0.3)] bg-white"
                                } else {
                                    "{password_glow_class} relative flex items-center w-full h-12 bg-[#F5F5F7]/80 rounded-xl transition-all duration-200 focus-within:bg-white hover:bg-[#F5F5F7]"
                                },
                                div { class: "pl-4 pr-1 text-[#86868B] group-focus-within:text-[#007AFF] group-focus-within:scale-110 transition-all duration-300 flex-shrink-0 origin-center",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" }
                                    }
                                }
                                input {
                                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-[#007AFF] px-2 text-[15px] text-[#1D1D1F] placeholder-[#86868B]",
                                    r#type: if show_password() { "text" } else { "password" },
                                    value: "{password}",
                                    placeholder: "设置强密码",
                                    tabindex: "3",
                                    oninput: handle_password_change
                                }
                                // Visibility toggle
                                button {
                                    class: "pr-4 text-[#86868B] hover:text-[#007AFF] transition-colors flex-shrink-0",
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
                            }
                            div {
                                class: if password_error().is_some() {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-100 z-20 pointer-events-none"
                                } else {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-0 z-20 pointer-events-none"
                                },
                                div { class: "w-1.5 h-1.5 rounded-full bg-[#FF3B30] shadow-[0_0_8px_rgba(255,59,48,0.8)] translate-y-[0.5px]" }
                                span { class: "text-[12px] text-[#FF3B30] font-medium opacity-90",
                                    "{password_error().unwrap_or_else(|| \" \".to_string())}"
                                }
                            }
                        }

                        // Confirm Password Input with match indicator
                        div { class: "group relative",
                            label { class: "block text-[13px] font-medium text-[#86868B] mb-2 uppercase tracking-wider ml-1",
                                "确认密码"
                            }
                            div {
                                class: if confirm_error().is_some() {
                                    "relative flex items-center w-full h-12 bg-[#F5F5F7]/80 rounded-xl transition-all duration-200 shadow-[0_0_0_2px_rgba(255,59,48,0.3)] bg-white"
                                } else {
                                    "relative flex items-center w-full h-12 bg-[#F5F5F7]/80 rounded-xl transition-all duration-200 focus-within:shadow-[0_0_0_2px_rgba(88,86,214,0.3)] focus-within:bg-white hover:bg-[#F5F5F7]"
                                },
                                div { class: "pl-4 pr-1 text-[#86868B] group-focus-within:text-[#007AFF] group-focus-within:scale-110 transition-all duration-300 flex-shrink-0 origin-center",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "1.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12.75L11.25 15 15 9.75M21 12a9 9 0 11-18 0 9 9 0 0118 0z" }
                                    }
                                }
                                input {
                                    class: "w-full h-full bg-transparent border-none focus:ring-0 focus:outline-none caret-[#007AFF] px-2 text-[15px] text-[#1D1D1F] placeholder-[#86868B]",
                                    r#type: if show_confirm_password() { "text" } else { "password" },
                                    value: "{confirm_password}",
                                    placeholder: "再次输入密码",
                                    tabindex: "4",
                                    oninput: handle_confirm_password_change
                                }
                                // Match indicator
                                if passwords_match {
                                    div { class: "pr-4 text-green-500 flex-shrink-0",
                                        svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M5 13l4 4L19 7" }
                                        }
                                    }
                                } else {
                                    // Visibility toggle when no match/empty
                                    button {
                                        class: "pr-4 text-[#86868B] hover:text-[#007AFF] transition-colors flex-shrink-0",
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
                                }
                            }
                            div {
                                class: if confirm_error().is_some() {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-100 z-20 pointer-events-none"
                                } else {
                                    "absolute top-0 right-0 flex items-center gap-1.5 transition-opacity duration-200 opacity-0 z-20 pointer-events-none"
                                },
                                div { class: "w-1.5 h-1.5 rounded-full bg-[#FF3B30] shadow-[0_0_8px_rgba(255,59,48,0.8)] translate-y-[0.5px]" }
                                span { class: "text-[12px] text-[#FF3B30] font-medium opacity-90",
                                    "{confirm_error().unwrap_or_else(|| \" \".to_string())}"
                                }
                            }
                        }


                        // Register Button - Primary Action with high visual weight
                        button {
                            class: "group relative w-full h-12 mt-6 text-[16px] font-semibold text-white transition-all duration-300 bg-gradient-to-r from-[#007AFF] to-[#5856D6] rounded-xl shadow-[0_10px_30px_-10px_rgba(0,122,255,0.5)] hover:shadow-[0_20px_40px_-10px_rgba(0,122,255,0.6)] hover:scale-[1.015] hover:-translate-y-0.5 active:scale-[0.985] active:translate-y-0 disabled:opacity-60 disabled:cursor-not-allowed disabled:hover:scale-100 disabled:hover:translate-y-0 overflow-hidden",
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
                                    "开始体验"
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
                            to: "/login",
                            class: "text-[15px] font-medium text-[#86868B] hover:text-[#1D1D1F] transition-colors",
                            "已有账号？返回登录"
                        }
                    }
                }
            }
        }
    }
}
