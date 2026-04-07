use crate::app::Route;
use burncloud_client_shared::components::logo::Logo;
use burncloud_client_shared::DesktopMode;
use dioxus::prelude::*;

#[component]
pub fn Root() -> Element {
    let navigator = use_navigator();

    use_effect(move || {
        spawn(async move {
            // Wait for Router to stabilize (handle hydration/initialization delay)
            // This prevents false positives where Router momentarily thinks path is "/"
            // before updating to the real URL (e.g. "/console/dashboard").
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            navigator.replace(Route::HomePage {});
        });
    });

    // Render nothing (white screen) to avoid flashing
    rsx! {
        div { class: "h-screen w-screen", style: "background-color: var(--bc-bg-canvas);" }
    }
}

#[component]
pub fn HomePage() -> Element {
    let active_nodes = use_signal(|| 842);
    let response_time = use_signal(|| "12");
    let uptime = use_signal(|| "99.9");

    // Check if running in desktop mode to adjust padding for window controls
    let is_desktop = try_use_context::<DesktopMode>().is_some();
    let nav_padding = if is_desktop {
        "relative z-40 flex items-center justify-between pl-8 pr-8 pt-14 pb-6 max-w-[1200px] mx-auto w-full animate-slide-up"
    } else {
        "relative z-40 flex items-center justify-between px-8 py-6 max-w-[1200px] mx-auto w-full animate-slide-up"
    };

    rsx! {
        // Container: White Ceramic Base
        div { class: "min-h-full w-full relative font-sans",
            style: "background-color: var(--bc-bg-canvas); color: var(--bc-text-primary); --tw-selection-color: var(--bc-primary);",

            // ========== BACKGROUND: Liquid Light Field ==========
            div { class: "absolute inset-0 pointer-events-none overflow-hidden",
                // Layer 1: Primary Aurora Blob - morphing shape
                div { class: "absolute top-[-20%] left-[-5%] w-[900px] h-[900px] bg-gradient-to-r from-[#FF2D55]/15 via-[#AF52DE]/12 to-[#007AFF]/15 animate-aurora animate-morph" }

                // Layer 2: Secondary Flow
                div { class: "absolute bottom-[-15%] right-[-5%] w-[800px] h-[800px] bg-gradient-to-l from-[#30B0C7]/20 via-[#5856D6]/15 to-transparent rounded-full blur-[80px] animate-aurora [animation-delay:7s] [animation-duration:25s]" }

                // Layer 3: Accent Orb - top right glow
                div { class: "absolute top-[10%] right-[15%] w-[400px] h-[400px] bg-gradient-to-br from-[#5AC8FA]/20 to-[#007AFF]/10 rounded-full blur-[60px] animate-float [animation-delay:2s]" }

                // Layer 4: Bottom accent
                div { class: "absolute bottom-[20%] left-[20%] w-[300px] h-[300px] bg-gradient-to-tr from-[#FF9500]/10 to-[#FF2D55]/10 rounded-full blur-[50px] animate-float [animation-delay:4s]" }

                // Orbiting particles - subtle and minimal
                div { class: "absolute top-1/2 left-1/3 -translate-x-1/2 -translate-y-1/2 w-[500px] h-[500px] opacity-60",
                    div { class: "absolute top-1/2 left-1/2 w-2 h-2 bg-[#007AFF]/25 rounded-full blur-[3px] animate-orbit [animation-duration:20s]" }
                    div { class: "absolute top-1/2 left-1/2 w-1.5 h-1.5 bg-[#AF52DE]/20 rounded-full blur-[2px] animate-orbit [animation-duration:30s] [animation-delay:10s]" }
                }

                // Grid pattern overlay
                div {
                    class: "absolute inset-0 opacity-[0.03]",
                    style: "background-image: radial-gradient(circle at 1px 1px, var(--bc-text-primary) 1px, transparent 0); background-size: 40px 40px;"
                }
            }

            // ========== NAVBAR: Minimalist Totem ==========
            nav { class: "{nav_padding}",
                // Logo: Pure Symbol
                div { class: "flex items-center gap-3 select-none group",
                    div { class: "flex items-center justify-center transition-all duration-500 group-hover:scale-110 group-hover:rotate-6",
                        Logo { class: "w-10 h-10 fill-current" }
                    }
                    span { class: "font-semibold text-xl tracking-tight text-primary transition-all duration-300 group-hover:tracking-wide", "BurnCloud" }
                }

                // Action: The Capsule with glow
                Link {
                    to: Route::LoginPage {},
                    class: "px-6 py-2.5 rounded-full text-[15px] font-medium transition-all duration-300 hover:scale-105 hover:shadow-[0_8px_20px_rgba(0,0,0,0.2)]",
                    style: "background-color: rgba(0, 0, 0, 0.06); color: var(--bc-text-primary);",
                    "Sign In"
                }
            }

            // ========== MAIN STAGE: Asymmetrical Balance ==========
            div { class: "relative z-10 max-w-[1200px] mx-auto px-8 pt-12 lg:pt-16 pb-16 flex flex-col lg:flex-row gap-10 lg:gap-16 items-center lg:items-start",

                // ========== LEFT: Typography ==========
                div { class: "flex-1 text-center lg:text-left pt-6 relative",
                    // Floating decorative dots around headline - hidden on mobile
                    div { class: "hidden lg:block absolute top-0 left-0 w-full h-full pointer-events-none overflow-visible",
                        // Primary purple dot - after "Interface" period
                        div { class: "absolute top-[12%] left-[88%] w-3.5 h-3.5 rounded-full bg-[#AF52DE] shadow-[0_0_12px_rgba(175,82,222,0.6)] animate-float [animation-duration:8s]" }
                        // Smaller purple accent - near purple
                        div { class: "absolute top-[9%] left-[92%] w-2 h-2 rounded-full bg-[#5856D6] shadow-[0_0_8px_rgba(88,86,214,0.5)] animate-float [animation-duration:9s] [animation-delay:1s]" }
                        // Blue dot - after "Intelligence" period
                        div { class: "absolute top-[34%] left-[95%] w-3 h-3 rounded-full bg-[#007AFF] shadow-[0_0_12px_rgba(0,122,255,0.6)] animate-float [animation-duration:10s] [animation-delay:2s]" }
                        // Green dot - near bottom right
                        div { class: "absolute top-[58%] left-[90%] w-2.5 h-2.5 rounded-full bg-[#34C759] shadow-[0_0_10px_rgba(52,199,89,0.5)] animate-float [animation-duration:12s] [animation-delay:1s]" }
                        // Light blue accent
                        div { class: "absolute top-[42%] left-[85%] w-2 h-2 rounded-full bg-[#5AC8FA] shadow-[0_0_8px_rgba(90,200,250,0.5)] animate-float [animation-duration:11s] [animation-delay:3s]" }
                    }

                    // Main headline with staggered animation
                    h1 { class: "text-3xl sm:text-4xl lg:text-6xl xl:text-7xl font-semibold tracking-tight mb-7 text-primary animate-slide-up animate-delay-100 relative z-10",
                        span { class: "block leading-tight mb-4", "One Interface." }
                        div { class: "block pb-4",
                            span { class: "block text-transparent bg-clip-text bg-gradient-to-r from-[#007AFF] via-[#5856D6] to-[#AF52DE] animate-gradient-flow leading-tight",
                                "Every Intelligence."
                            }
                        }
                    }

                    // Subtitle
                    p { class: "text-[20px] lg:text-[22px] text-secondary mb-9 max-w-lg mx-auto lg:mx-0 font-normal leading-[1.7] animate-slide-up animate-delay-200 relative z-10",
                        "Connect to the world's leading AI models—GPT, Claude, Gemini—with the simplicity of a single tap."
                    }

                    // CTA Buttons with enhanced effects
                    div { class: "flex flex-col sm:flex-row gap-4 justify-center lg:justify-start items-center animate-slide-up animate-delay-300 relative z-10",
                        // Primary CTA with glow pulse
                        Link {
                            to: Route::RegisterPage {},
                            class: "bc-btn-gradient group relative inline-flex items-center justify-center px-10 py-4 text-[17px] font-semibold rounded-full animate-glow-pulse overflow-hidden",
                            style: "box-shadow: var(--bc-shadow-primary);",
                            // Shimmer overlay
                            div { class: "absolute inset-0 animate-shimmer opacity-0 group-hover:opacity-100 transition-opacity duration-300" }
                            span { class: "relative z-10 flex items-center gap-2",
                                "Get Started"
                                svg { class: "w-4 h-4 transition-transform duration-300 group-hover:translate-x-1", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2.5", d: "M9 5l7 7-7 7" }
                                }
                            }
                        }
                    }

                    // Trust indicators
                    div { class: "mt-8 flex items-center gap-6 justify-center lg:justify-start animate-slide-up animate-delay-400 relative z-10",
                        div { class: "flex items-center gap-2 text-[13px] text-secondary font-medium",
                            div { class: "w-2 h-2 rounded-full bg-[#34C759] shadow-[0_0_6px_rgba(52,199,89,0.4)]" }
                            "Enterprise Ready"
                        }
                        div { class: "w-[1px] h-4",
                            style: "background-color: var(--bc-text-disabled);",
                        }
                        div { class: "text-[13px] text-secondary font-medium", "Open Source" }
                    }
                }

                // ========== RIGHT: Bento Cards ==========
                div { class: "flex-1 w-full max-w-[520px] animate-slide-up animate-delay-300",
                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-5 auto-rows-[minmax(140px,auto)] sm:auto-rows-[minmax(160px,auto)]",

                        // ===== Card 1: Global Network (Hero Card) =====
                        div { class: "col-span-1 sm:col-span-2 row-span-1 magnetic-hover",
                            div { class: "group h-full bc-card-glass rounded-[28px] p-7 flex flex-col justify-between overflow-hidden relative transition-all duration-500",
                                style: "box-shadow: var(--bc-shadow-md);",
                                // Glossy reflection
                                div { class: "absolute top-0 right-0 w-72 h-72 bg-gradient-to-br from-white/90 to-transparent opacity-60 pointer-events-none rounded-full blur-3xl -translate-y-1/2 translate-x-1/2 transition-opacity group-hover:opacity-80" }

                                // Animated background gradient
                                div { class: "absolute inset-0 bg-gradient-to-br from-[#007AFF]/5 to-[#5856D6]/5 opacity-0 group-hover:opacity-100 transition-opacity duration-700" }

                                div { class: "flex justify-between items-start relative z-10",
                                    span { class: "text-[12px] font-semibold text-secondary uppercase tracking-[0.15em]", "Global Network" }
                                    // Pulsing live indicator
                                    div { class: "relative",
                                        div { class: "w-3 h-3 rounded-full bg-[#34C759] shadow-[0_0_8px_rgba(52,199,89,0.5)]" }
                                        div { class: "absolute inset-0 w-3 h-3 rounded-full bg-[#34C759] animate-ripple" }
                                    }
                                }

                                div { class: "relative z-10 mt-3",
                                    div { class: "flex items-baseline gap-2",
                                        div { class: "text-5xl lg:text-6xl font-medium tracking-tight text-primary animate-count", "{active_nodes}" }
                                        div { class: "text-[16px] text-secondary font-medium", "Nodes" }
                                    }
                                    div { class: "text-[14px] text-secondary mt-2 font-medium", "Powering AI worldwide, 24/7" }
                                }

                                // Interactive Dot Network
                                div { class: "absolute bottom-[-15%] right-[-8%] opacity-10 group-hover:opacity-25 transition-all duration-700 group-hover:scale-110",
                                    svg { width: "220", height: "220", view_box: "0 0 100 100", fill: "currentColor",
                                        // Nodes
                                        circle { cx: "15", cy: "15", r: "2.5", class: "animate-pulse" }
                                        circle { cx: "35", cy: "25", r: "2", class: "animate-pulse [animation-delay:0.3s]" }
                                        circle { cx: "55", cy: "12", r: "2.5", class: "animate-pulse [animation-delay:0.6s]" }
                                        circle { cx: "25", cy: "45", r: "2", class: "animate-pulse [animation-delay:0.9s]" }
                                        circle { cx: "45", cy: "55", r: "3", class: "animate-pulse [animation-delay:1.2s]" }
                                        circle { cx: "65", cy: "42", r: "2", class: "animate-pulse [animation-delay:1.5s]" }
                                        circle { cx: "80", cy: "25", r: "2.5", class: "animate-pulse [animation-delay:1.8s]" }
                                        circle { cx: "75", cy: "60", r: "2", class: "animate-pulse [animation-delay:2.1s]" }
                                        circle { cx: "90", cy: "50", r: "2", class: "animate-pulse [animation-delay:2.4s]" }
                                        // Connecting lines
                                        line { x1: "15", y1: "15", x2: "35", y2: "25", stroke: "currentColor", stroke_width: "0.5", opacity: "0.3" }
                                        line { x1: "35", y1: "25", x2: "55", y2: "12", stroke: "currentColor", stroke_width: "0.5", opacity: "0.3" }
                                        line { x1: "35", y1: "25", x2: "45", y2: "55", stroke: "currentColor", stroke_width: "0.5", opacity: "0.3" }
                                        line { x1: "55", y1: "12", x2: "80", y2: "25", stroke: "currentColor", stroke_width: "0.5", opacity: "0.3" }
                                        line { x1: "45", y1: "55", x2: "65", y2: "42", stroke: "currentColor", stroke_width: "0.5", opacity: "0.3" }
                                        line { x1: "65", y1: "42", x2: "80", y2: "25", stroke: "currentColor", stroke_width: "0.5", opacity: "0.3" }
                                        line { x1: "65", y1: "42", x2: "75", y2: "60", stroke: "currentColor", stroke_width: "0.5", opacity: "0.3" }
                                        line { x1: "75", y1: "60", x2: "90", y2: "50", stroke: "currentColor", stroke_width: "0.5", opacity: "0.3" }
                                    }
                                }
                            }
                        }

                        // ===== Card 2: Lightning Fast =====
                        div { class: "col-span-1 magnetic-hover",
                            div { class: "group h-full bc-card-glass rounded-[28px] p-6 flex flex-col justify-between relative overflow-hidden transition-all duration-500",
                                style: "box-shadow: var(--bc-shadow-md);",
                                // Purple gradient overlay on hover
                                div { class: "absolute inset-0 bg-gradient-to-br from-[#AF52DE]/10 to-[#5856D6]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-500" }

                                div { class: "relative z-10",
                                    div { class: "text-[12px] font-semibold text-secondary uppercase tracking-[0.15em] mb-3", "Response Time" }
                                    div { class: "flex items-baseline gap-1",
                                        div { class: "text-4xl font-medium text-primary transition-transform duration-300 group-hover:scale-110", "{response_time}" }
                                        div { class: "text-[15px] text-secondary font-medium", "ms" }
                                    }
                                }
                                // Lightning Icon with glow
                                div { class: "relative z-10 mt-4 w-11 h-11 rounded-full bg-[#DDD6FE] flex items-center justify-center text-[#7C3AED] transition-all duration-300 group-hover:scale-110 group-hover:shadow-[0_0_20px_rgba(124,58,237,0.4)]",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" }
                                    }
                                }
                            }
                        }

                        // ===== Card 3: Always Available =====
                        div { class: "col-span-1 magnetic-hover",
                            div { class: "group h-full bc-card-glass rounded-[28px] p-6 flex flex-col justify-between relative overflow-hidden transition-all duration-500",
                                style: "box-shadow: var(--bc-shadow-md);",
                                // Green gradient overlay on hover
                                div { class: "absolute inset-0 bg-gradient-to-br from-[#34C759]/10 to-[#30D158]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-500" }

                                div { class: "relative z-10",
                                    div { class: "text-[12px] font-semibold text-secondary uppercase tracking-[0.15em] mb-3", "Uptime" }
                                    div { class: "flex items-baseline gap-0",
                                        div { class: "text-4xl font-medium text-primary transition-transform duration-300 group-hover:scale-110", "{uptime}" }
                                        div { class: "text-[16px] text-secondary font-medium", "%" }
                                    }
                                }
                                // Checkmark Icon with glow
                                div { class: "relative z-10 mt-4 w-11 h-11 rounded-full bg-[#BBF7D0] flex items-center justify-center text-[#16A34A] transition-all duration-300 group-hover:scale-110 group-hover:shadow-[0_0_20px_rgba(22,163,74,0.4)]",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" }
                                    }
                                }
                            }
                        }

                        // ===== Card 4: Multi-Provider =====
                        div { class: "col-span-1 sm:col-span-2 magnetic-hover",
                            div { class: "group h-full bc-card-glass rounded-[28px] p-6 flex items-center justify-between relative overflow-hidden transition-all duration-500",
                                style: "box-shadow: var(--bc-shadow-md);",
                                div { class: "relative z-10",
                                    div { class: "text-[12px] font-semibold text-secondary uppercase tracking-[0.15em] mb-2", "Unified API" }
                                    div { class: "text-[15px] text-primary font-medium", "One key. All providers." }
                                }

                                // Provider logos/icons animated
                                div { class: "flex items-center gap-3 relative z-10",
                                    // OpenAI
                                    div { class: "w-10 h-10 rounded-xl flex items-center justify-center text-white text-[11px] font-bold transition-all duration-300 hover:scale-110 hover:rotate-6 shadow-lg",
                                        style: "background-color: var(--bc-text-primary);",
                                        "GPT"
                                    }
                                    // Anthropic
                                    div { class: "w-10 h-10 rounded-xl bg-gradient-to-br from-[#D97757] to-[#C96442] flex items-center justify-center text-white text-[10px] font-bold transition-all duration-300 hover:scale-110 hover:-rotate-6 shadow-lg [animation-delay:0.1s]",
                                        "Claude"
                                    }
                                    // Google
                                    div { class: "w-10 h-10 rounded-xl bg-gradient-to-br from-[#4285F4] to-[#34A853] flex items-center justify-center text-white text-[10px] font-bold transition-all duration-300 hover:scale-110 hover:rotate-6 shadow-lg [animation-delay:0.2s]",
                                        "Gem"
                                    }
                                    // More indicator
                                    div { class: "w-10 h-10 rounded-xl border flex items-center justify-center text-[13px] font-semibold transition-all duration-300 hover:scale-110 shadow-sm",
                                        style: "background-color: var(--bc-bg-hover); border-color: var(--bc-border); color: var(--bc-text-secondary);",
                                        "+9"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ========== FOOTER: Subtle ==========
            div { class: "relative z-10 w-full text-center pb-8 pointer-events-none animate-slide-up animate-delay-500",
                span { class: "text-[13px] font-medium text-secondary tracking-[0.12em] uppercase",
                    "Engineered with "
                    span { class: "text-[14px]",
                        style: "color: var(--bc-warning);",
                        "🦀"
                    }
                    " Rust"
                }
            }
        }
    }
}
