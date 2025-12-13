use crate::app::Route;
use burncloud_client_shared::components::{BCButton, BCCard};
use dioxus::prelude::*;

#[component]
pub fn HomePage() -> Element {
    // 模拟数据
    let active_nodes = use_signal(|| 842);
    let total_tps = use_signal(|| "12.4k");
    let earnings = use_signal(|| "$0.02/req");

    rsx! {
        // 主容器：明亮、通透、现代
        div { class: "h-full w-full overflow-hidden bg-base-100 text-base-content relative selection:bg-primary selection:text-primary-content animate-fade-in",
            
            // 背景装饰：极光渐变 (Aurora Gradient) - 微妙且高级
            div { class: "absolute top-[-20%] right-[-10%] w-[600px] h-[600px] bg-blue-100 rounded-full blur-[128px] opacity-60 pointer-events-none" }
            div { class: "absolute bottom-[-20%] left-[-10%] w-[500px] h-[500px] bg-purple-100 rounded-full blur-[128px] opacity-60 pointer-events-none" }

            // 导航栏 (极简)
            nav { class: "relative z-50 flex items-center justify-between px-8 py-6 max-w-7xl mx-auto w-full",
                // Logo
                div { class: "flex items-center gap-2",
                    div { class: "w-8 h-8 bg-black text-white rounded-lg flex items-center justify-center",
                         svg { class: "w-5 h-5", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", stroke_width: "2",
                            path { stroke_linecap: "round", stroke_linejoin: "round", d: "M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" }
                        }
                    }
                    span { class: "font-semibold text-lg tracking-tight", "BurnCloud" }
                }
                
                // 登录按钮
                Link {
                    to: Route::LoginPage {},
                    class: "btn btn-sm btn-ghost text-base-content/70 hover:text-base-content hover:bg-base-200 transition-colors",
                    "Sign in"
                }
            }

            // 主要内容区：左侧大字报，右侧 Bento Grid
            div { class: "relative z-10 max-w-7xl mx-auto px-8 pt-12 flex flex-col lg:flex-row gap-16 items-center",
                
                // Left Column: Value Proposition (Macro Typography)
                div { class: "flex-1 text-center lg:text-left",
                    h1 { class: "text-6xl lg:text-7xl font-bold tracking-tight mb-8 leading-[1.1]",
                        span { class: "block text-base-content", "Deploy Local." }
                        span { class: "block text-transparent bg-clip-text bg-gradient-to-r from-blue-600 to-purple-600", "Scale Global." }
                    }
                    p { class: "text-xl text-base-content/60 mb-10 max-w-md mx-auto lg:mx-0 leading-relaxed",
                        "The unified operating system for your AI infrastructure. Manage models, route traffic, and monetize idle compute."
                    }
                    
                    div { class: "flex flex-col sm:flex-row gap-4 justify-center lg:justify-start",
                        Link {
                            to: Route::RegisterPage {},
                            class: "btn btn-primary btn-lg rounded-full px-8 shadow-lg hover:shadow-xl hover:-translate-y-1 transition-all",
                            "Get Started Free"
                        }
                        a {
                            href: "https://github.com/burncloud/burncloud",
                            target: "_blank",
                            class: "btn btn-ghost btn-lg rounded-full px-8",
                            "View on GitHub"
                        }
                    }
                }

                // Right Column: The Bento Grid (Functional Aesthetics)
                div { class: "flex-1 w-full max-w-lg",
                    div { class: "grid grid-cols-2 gap-4 auto-rows-[minmax(140px,auto)]",
                        
                        // Card 1: Main Stats (Large)
                        div { class: "col-span-2 row-span-1",
                            BCCard {
                                class: "h-full bg-white/50 backdrop-blur-xl border border-white/20 hover:border-blue-500/30 transition-colors group",
                                div { class: "p-6 flex flex-col h-full justify-between",
                                    div { class: "flex items-center justify-between",
                                        span { class: "text-sm font-medium text-base-content/50 uppercase tracking-wider", "Global Grid" }
                                        span { class: "flex h-2 w-2 relative",
                                            span { class: "animate-ping absolute inline-flex h-full w-full rounded-full bg-emerald-400 opacity-75" }
                                            span { class: "relative inline-flex rounded-full h-2 w-2 bg-emerald-500" }
                                        }
                                    }
                                    div {
                                        div { class: "text-4xl font-mono font-semibold text-base-content mt-2 group-hover:text-blue-600 transition-colors", "{active_nodes}" }
                                        div { class: "text-sm text-base-content/60", "Active Nodes Online" }
                                    }
                                }
                            }
                        }

                        // Card 2: Performance
                        div { class: "col-span-1",
                            BCCard {
                                class: "h-full bg-white/50 backdrop-blur-xl border border-white/20 hover:border-purple-500/30 transition-colors",
                                div { class: "p-5 flex flex-col h-full justify-between",
                                    div { class: "w-8 h-8 rounded-full bg-purple-100 text-purple-600 flex items-center justify-center mb-2",
                                        svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M13 10V3L4 14h7v7l9-11h-7z" } }
                                    }
                                    div {
                                        div { class: "text-2xl font-bold", "{total_tps}" }
                                        div { class: "text-xs text-base-content/50", "Tokens / Sec" }
                                    }
                                }
                            }
                        }

                        // Card 3: Earnings
                        div { class: "col-span-1",
                            BCCard {
                                class: "h-full bg-white/50 backdrop-blur-xl border border-white/20 hover:border-green-500/30 transition-colors",
                                div { class: "p-5 flex flex-col h-full justify-between",
                                     div { class: "w-8 h-8 rounded-full bg-green-100 text-green-600 flex items-center justify-center mb-2",
                                        svg { class: "w-4 h-4", fill: "none", view_box: "0 0 24 24", stroke: "currentColor", path { stroke_linecap: "round", stroke_linejoin: "round", stroke_width: "2", d: "M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z" } }
                                    }
                                    div {
                                        div { class: "text-2xl font-bold", "{earnings}" }
                                        div { class: "text-xs text-base-content/50", "Avg. Yield" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Trust / Social Proof (Bottom)
            div { class: "absolute bottom-0 w-full border-t border-base-200/50 bg-base-100/50 backdrop-blur-sm py-6",
                div { class: "max-w-7xl mx-auto px-8 flex justify-between items-center text-xs text-base-content/40 font-medium uppercase tracking-widest",
                    span { "Trusted by open source teams" }
                    div { class: "flex gap-8",
                        span { "Llama" }
                        span { "Mistral" }
                        span { "Qwen" }
                        span { "DeepSeek" }
                    }
                }
            }
        }
    }
}