use crate::app::Route;
use dioxus::prelude::*;

#[component]
pub fn HomePage() -> Element {
    let active_nodes = use_signal(|| 842);
    let response_time = use_signal(|| "12");
    let uptime = use_signal(|| "99.9");

    rsx! {
        // Container: White Ceramic Base
        div { class: "h-full w-full overflow-hidden bg-[#F5F5F7] text-[#1D1D1F] relative selection:bg-[#0071E3] selection:text-white font-sans",

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

                // Orbiting particles
                div { class: "absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[600px] h-[600px]",
                    div { class: "absolute top-1/2 left-1/2 w-3 h-3 bg-[#007AFF]/40 rounded-full blur-[2px] animate-orbit" }
                    div { class: "absolute top-1/2 left-1/2 w-2 h-2 bg-[#AF52DE]/40 rounded-full blur-[1px] animate-orbit [animation-duration:15s] [animation-delay:5s]" }
                    div { class: "absolute top-1/2 left-1/2 w-2 h-2 bg-[#34C759]/40 rounded-full blur-[1px] animate-orbit [animation-duration:25s] [animation-delay:10s]" }
                }

                // Grid pattern overlay
                div {
                    class: "absolute inset-0 opacity-[0.03]",
                    style: "background-image: radial-gradient(circle at 1px 1px, #1D1D1F 1px, transparent 0); background-size: 40px 40px;"
                }
            }

            // ========== NAVBAR: Minimalist Totem ==========
            nav { class: "relative z-50 flex items-center justify-between px-8 py-6 max-w-[1200px] mx-auto w-full animate-slide-up",
                // Logo: Pure Symbol
                div { class: "flex items-center gap-3 select-none group",
                    div { class: "w-10 h-10 bg-black/90 text-white rounded-[14px] flex items-center justify-center shadow-[0_4px_12px_rgba(0,0,0,0.15)] backdrop-blur-md transition-all duration-500 group-hover:scale-110 group-hover:shadow-[0_8px_24px_rgba(0,0,0,0.25)] group-hover:rotate-6",
                        svg { class: "w-5 h-5 transition-transform duration-500 group-hover:scale-110", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2.5",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" }
                        }
                    }
                    span { class: "font-semibold text-xl tracking-tight text-[#1D1D1F] transition-all duration-300 group-hover:tracking-wide", "BurnCloud" }
                }

                // Action: The Capsule with glow
                Link {
                    to: Route::LoginPage {},
                    class: "px-6 py-2.5 rounded-full bg-[#E8E8ED] hover:bg-[#1D1D1F] hover:text-white text-[#1D1D1F] text-[15px] font-medium transition-all duration-300 hover:shadow-[0_8px_20px_rgba(0,0,0,0.2)] hover:scale-105",
                    "Sign In"
                }
            }

            // ========== MAIN STAGE: Asymmetrical Balance ==========
            div { class: "relative z-10 max-w-[1200px] mx-auto px-8 pt-16 lg:pt-24 pb-16 flex flex-col lg:flex-row gap-20 items-center lg:items-start",

                // ========== LEFT: Typography ==========
                div { class: "flex-1 text-center lg:text-left pt-6",
                    // Main headline with staggered animation
                    h1 { class: "text-6xl lg:text-7xl xl:text-8xl font-semibold tracking-tighter mb-8 leading-[1.0] text-[#1D1D1F] animate-slide-up animate-delay-100",
                        span { class: "block", "One Interface." }
                        span { class: "block text-transparent bg-clip-text bg-gradient-to-r from-[#007AFF] via-[#5856D6] to-[#AF52DE] animate-gradient-flow",
                            "Every Intelligence."
                        }
                    }

                    // Subtitle
                    p { class: "text-[20px] lg:text-[22px] text-[#86868B] mb-10 max-w-lg mx-auto lg:mx-0 font-normal leading-relaxed animate-slide-up animate-delay-200",
                        "Connect to the world's leading AI models—GPT, Claude, Gemini—with the simplicity of a single tap."
                    }

                    // CTA Buttons with enhanced effects
                    div { class: "flex flex-col sm:flex-row gap-4 justify-center lg:justify-start items-center animate-slide-up animate-delay-300",
                        // Primary CTA with glow pulse
                        Link {
                            to: Route::RegisterPage {},
                            class: "group relative inline-flex items-center justify-center px-10 py-4 text-[17px] font-semibold text-white transition-all duration-500 bg-gradient-to-r from-[#0071E3] to-[#5856D6] rounded-full hover:from-[#0077ED] hover:to-[#6E6AE8] shadow-[0_10px_30px_-5px_rgba(0,113,227,0.4)] hover:shadow-[0_20px_40px_-5px_rgba(0,113,227,0.5)] hover:scale-[1.05] animate-glow-pulse overflow-hidden",
                            // Shimmer overlay
                            div { class: "absolute inset-0 animate-shimmer opacity-0 group-hover:opacity-100 transition-opacity duration-300" }
                            span { class: "relative z-10 flex items-center gap-2",
                                "Get Started"
                                svg { class: "w-4 h-4 transition-transform duration-300 group-hover:translate-x-1", fill: "none", stroke: "currentColor", view_box: "0 0 24 24",
                                    path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2.5", d: "M9 5l7 7-7 7" }
                                }
                            }
                        }
                        // Secondary CTA
                        Link {
                            to: Route::LoginPage {},
                            class: "group inline-flex items-center justify-center px-10 py-4 text-[17px] font-medium text-[#1D1D1F] transition-all duration-300 hover:text-[#0071E3]",
                            span { "Learn More" }
                            span { class: "ml-1 transition-transform duration-300 group-hover:translate-x-2", "→" }
                        }
                    }

                    // Trust indicators
                    div { class: "mt-12 flex items-center gap-6 justify-center lg:justify-start animate-slide-up animate-delay-400",
                        div { class: "flex items-center gap-2 text-[13px] text-[#86868B]",
                            div { class: "w-2 h-2 rounded-full bg-[#34C759] animate-pulse" }
                            "Enterprise Ready"
                        }
                        div { class: "w-px h-4 bg-[#D2D2D7]" }
                        div { class: "text-[13px] text-[#86868B]", "SOC 2 Compliant" }
                        div { class: "w-px h-4 bg-[#D2D2D7]" }
                        div { class: "text-[13px] text-[#86868B]", "Open Source" }
                    }
                }

                // ========== RIGHT: Bento Cards ==========
                div { class: "flex-1 w-full max-w-[520px] animate-slide-up animate-delay-300",
                    div { class: "grid grid-cols-2 gap-5 auto-rows-[minmax(160px,auto)]",

                        // ===== Card 1: Global Network (Hero Card) =====
                        div { class: "col-span-2 row-span-1 magnetic-hover",
                            div { class: "group h-full glass rounded-[28px] shadow-[0_20px_50px_-12px_rgba(0,0,0,0.08)] p-7 flex flex-col justify-between overflow-hidden relative transition-all duration-500 hover:shadow-[0_30px_60px_-12px_rgba(0,0,0,0.15)]",
                                // Glossy reflection
                                div { class: "absolute top-0 right-0 w-72 h-72 bg-gradient-to-br from-white/90 to-transparent opacity-60 pointer-events-none rounded-full blur-3xl -translate-y-1/2 translate-x-1/2 transition-opacity group-hover:opacity-80" }

                                // Animated background gradient
                                div { class: "absolute inset-0 bg-gradient-to-br from-[#007AFF]/5 to-[#5856D6]/5 opacity-0 group-hover:opacity-100 transition-opacity duration-700" }

                                div { class: "flex justify-between items-start relative z-10",
                                    span { class: "text-[12px] font-semibold text-[#86868B] uppercase tracking-[0.15em]", "Global Network" }
                                    // Pulsing live indicator
                                    div { class: "relative",
                                        div { class: "w-2.5 h-2.5 rounded-full bg-[#34C759]" }
                                        div { class: "absolute inset-0 w-2.5 h-2.5 rounded-full bg-[#34C759] animate-ripple" }
                                    }
                                }

                                div { class: "relative z-10 mt-3",
                                    div { class: "flex items-baseline gap-2",
                                        div { class: "text-5xl lg:text-6xl font-medium tracking-tight text-[#1D1D1F] animate-count", "{active_nodes}" }
                                        div { class: "text-[16px] text-[#86868B] font-medium", "Nodes" }
                                    }
                                    div { class: "text-[14px] text-[#86868B] mt-2 font-medium", "Powering AI worldwide, 24/7" }
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
                            div { class: "group h-full glass rounded-[28px] shadow-[0_20px_50px_-12px_rgba(0,0,0,0.08)] p-6 flex flex-col justify-between relative overflow-hidden transition-all duration-500 hover:shadow-[0_30px_60px_-12px_rgba(0,0,0,0.15)]",
                                // Purple gradient overlay on hover
                                div { class: "absolute inset-0 bg-gradient-to-br from-[#AF52DE]/10 to-[#5856D6]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-500" }

                                div { class: "relative z-10",
                                    div { class: "text-[12px] font-semibold text-[#86868B] uppercase tracking-[0.15em] mb-3", "Response Time" }
                                    div { class: "flex items-baseline gap-1",
                                        div { class: "text-4xl font-medium text-[#1D1D1F] transition-transform duration-300 group-hover:scale-110", "{response_time}" }
                                        div { class: "text-[15px] text-[#86868B] font-medium", "ms" }
                                    }
                                }
                                // Lightning Icon with glow
                                div { class: "relative z-10 mt-4 w-11 h-11 rounded-full bg-gradient-to-br from-[#AF52DE]/15 to-[#5856D6]/15 flex items-center justify-center text-[#AF52DE] transition-all duration-300 group-hover:scale-110 group-hover:shadow-[0_0_20px_rgba(175,82,222,0.4)]",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M13 10V3L4 14h7v7l9-11h-7z" }
                                    }
                                }
                            }
                        }

                        // ===== Card 3: Always Available =====
                        div { class: "col-span-1 magnetic-hover",
                            div { class: "group h-full glass rounded-[28px] shadow-[0_20px_50px_-12px_rgba(0,0,0,0.08)] p-6 flex flex-col justify-between relative overflow-hidden transition-all duration-500 hover:shadow-[0_30px_60px_-12px_rgba(0,0,0,0.15)]",
                                // Green gradient overlay on hover
                                div { class: "absolute inset-0 bg-gradient-to-br from-[#34C759]/10 to-[#30D158]/10 opacity-0 group-hover:opacity-100 transition-opacity duration-500" }

                                div { class: "relative z-10",
                                    div { class: "text-[12px] font-semibold text-[#86868B] uppercase tracking-[0.15em] mb-3", "Uptime" }
                                    div { class: "flex items-baseline gap-0",
                                        div { class: "text-4xl font-medium text-[#1D1D1F] transition-transform duration-300 group-hover:scale-110", "{uptime}" }
                                        div { class: "text-[16px] text-[#86868B] font-medium", "%" }
                                    }
                                }
                                // Checkmark Icon with glow
                                div { class: "relative z-10 mt-4 w-11 h-11 rounded-full bg-gradient-to-br from-[#34C759]/15 to-[#30D158]/15 flex items-center justify-center text-[#34C759] transition-all duration-300 group-hover:scale-110 group-hover:shadow-[0_0_20px_rgba(52,199,89,0.4)]",
                                    svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2.5",
                                        path { stroke_linecap: "round", stroke_linejoin: "round", d: "M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" }
                                    }
                                }
                            }
                        }

                        // ===== Card 4: Multi-Provider =====
                        div { class: "col-span-2 magnetic-hover",
                            div { class: "group h-full glass rounded-[28px] shadow-[0_20px_50px_-12px_rgba(0,0,0,0.08)] p-6 flex items-center justify-between relative overflow-hidden transition-all duration-500 hover:shadow-[0_30px_60px_-12px_rgba(0,0,0,0.15)]",
                                div { class: "relative z-10",
                                    div { class: "text-[12px] font-semibold text-[#86868B] uppercase tracking-[0.15em] mb-2", "Unified API" }
                                    div { class: "text-[15px] text-[#1D1D1F] font-medium", "One key. All providers." }
                                }

                                // Provider logos/icons animated
                                div { class: "flex items-center gap-3 relative z-10",
                                    // OpenAI
                                    div { class: "w-10 h-10 rounded-xl bg-[#1D1D1F] flex items-center justify-center text-white text-[11px] font-bold transition-all duration-300 hover:scale-110 hover:rotate-6 shadow-lg",
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
                                    div { class: "w-10 h-10 rounded-xl bg-[#E8E8ED] flex items-center justify-center text-[#86868B] text-[13px] font-medium transition-all duration-300 hover:scale-110 hover:bg-[#D2D2D7]",
                                        "+9"
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // ========== FOOTER: Subtle ==========
            div { class: "absolute bottom-6 w-full text-center pointer-events-none animate-slide-up animate-delay-500",
                span { class: "text-[11px] font-medium text-[#86868B]/40 tracking-[0.2em] uppercase",
                    "Engineered with "
                    span { class: "text-[#FF6B35]", "🦀" }
                    " Rust"
                }
            }
        }
    }
}
