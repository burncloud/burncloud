use dioxus::prelude::*;
use burncloud_client_shared::log_service::LogService;
use burncloud_client_shared::usage_service::UsageService;
use burncloud_client_shared::monitor_service::MonitorService;
use burncloud_client_shared::i18n::{use_i18n, t};

#[component]
pub fn Dashboard() -> Element {
    let i18n = use_i18n();
    let lang = i18n.language.read();

    let mut logs = use_resource(move || async move {
        LogService::list(5).await.ok()
    });

    let usage = use_resource(move || async move {
        UsageService::get_user_usage("demo-user").await.ok()
    });

    let monitor = use_resource(move || async move {
        MonitorService::get_system_metrics().await.ok()
    });

    let logs_data = logs.read().clone();
    let usage_data = usage.read().clone();
    let monitor_data = monitor.read().clone();

    rsx! {
        div { class: "h-full flex flex-col max-w-6xl mx-auto animate-fade-in select-none",
            
            // 1. Hero Header
            div { class: "flex items-end justify-between mb-16",
                div {
                    h1 { class: "text-4xl font-thin tracking-tight text-base-content/90", 
                        "{t(*lang, \"dashboard.title\")}" 
                    }
                    div { class: "flex items-center gap-2 mt-3 pl-1",
                        div { class: "w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" }
                        span { class: "text-xs font-medium tracking-wide opacity-50 uppercase", "Pulse Normal" }
                    }
                }
                div {
                    button { class: "btn btn-primary btn-sm rounded-full px-6 font-normal shadow-sm hover:shadow-md transition-all normal-case", 
                        "+ {t(*lang, \"nav.deploy\")}" 
                    }
                }
            }

            // 2. Main Content Grid
            div { class: "grid grid-cols-1 lg:grid-cols-3 gap-12 h-full",
                
                // Left Column: The Brain & Stats (Spans 2 columns)
                div { class: "lg:col-span-2 flex flex-col gap-10",
                    
                    // The "Brain" Card - Floating, minimal
                    div { class: "flex items-start gap-6 group",
                        div { class: "w-24 h-24 rounded-3xl bg-gradient-to-br from-indigo-500/10 to-purple-500/10 flex items-center justify-center text-4xl shadow-inner",
                            "🧠"
                        }
                        div { class: "pt-2",
                            h2 { class: "text-3xl font-bold text-base-content/90 tracking-tight", "Coding Expert" }
                            div { class: "flex items-center gap-3 mt-2",
                                span { class: "badge badge-neutral badge-sm font-mono opacity-80", "Qwen2.5-7B" }
                                span { class: "text-sm text-base-content/40", "CUDA Accelerated" }
                            }
                        }
                    }

                    // Usage Data - The "Bank Account" view
                    div { class: "grid grid-cols-2 gap-12 pt-8 border-t border-base-200/50",
                        div {
                            div { class: "text-[10px] uppercase tracking-[0.2em] font-bold opacity-30 mb-2", "TOKEN USAGE" }
                            match usage_data {
                                Some(Some(stats)) => rsx! {
                                    div { class: "text-5xl font-thin tabular-nums tracking-tighter text-base-content/80", "{stats.total_tokens}" }
                                },
                                _ => rsx! { div { class: "h-12 bg-base-200/50 rounded w-32 animate-pulse" } }
                            }
                        }
                        div {
                            div { class: "text-[10px] uppercase tracking-[0.2em] font-bold opacity-30 mb-2", "EST. COST" }
                            div { class: "text-5xl font-thin tracking-tighter text-base-content/80", "$0.00" }
                        }
                    }

                    // Micro Metrics (CPU/RAM) - Almost invisible
                    div { class: "grid grid-cols-2 gap-8 pt-4",
                        match &monitor_data {
                            Some(Some(m)) => rsx! {
                                div { class: "space-y-2",
                                    div { class: "flex justify-between items-end",
                                        span { class: "text-[10px] font-bold tracking-widest opacity-30", "NEURAL LOAD" }
                                        span { class: "text-xs font-mono opacity-50", "{m.cpu.usage_percent:.0}%" }
                                    }
                                    div { class: "w-full h-0.5 bg-base-200 rounded-full overflow-hidden",
                                        div { class: "h-full bg-base-content/20 transition-all duration-500", style: "width: {m.cpu.usage_percent}%" }
                                    }
                                }
                                div { class: "space-y-2",
                                    div { class: "flex justify-between items-end",
                                        span { class: "text-[10px] font-bold tracking-widest opacity-30", "CONTEXT MEMORY" }
                                        span { class: "text-xs font-mono opacity-50", "{m.memory.usage_percent:.0}%" }
                                    }
                                    div { class: "w-full h-0.5 bg-base-200 rounded-full overflow-hidden",
                                        div { class: "h-full bg-base-content/20 transition-all duration-500", style: "width: {m.memory.usage_percent}%" }
                                    }
                                }
                            },
                            _ => rsx! { 
                                div { class: "skeleton h-4 w-full" }
                                div { class: "skeleton h-4 w-full" }
                            }
                        }
                    }
                }

                // Right Column: Activity Stream (Timeline)
                div { class: "flex flex-col pl-8 border-l border-base-200/50 dashed",
                    h3 { class: "text-[10px] font-bold opacity-30 uppercase tracking-[0.2em] mb-8", "RECENT ACTIVITY" }
                    
                    div { class: "flex-1 flex flex-col gap-6",
                        match logs_data {
                            Some(Some(list)) => rsx! {
                                for log in list {
                                    div { class: "relative pl-4 border-l-2 border-base-200 hover:border-primary/50 transition-colors py-1 group",
                                        div { class: "text-sm font-medium text-base-content/80 truncate", "{log.path}" }
                                        div { class: "flex justify-between items-center mt-1",
                                            span { class: "text-[10px] font-mono opacity-40", "{log.latency_ms}ms" }
                                            span { 
                                                class: if log.status_code >= 400 { "text-[10px] font-bold text-error opacity-60" } else { "text-[10px] font-bold text-success opacity-60" },
                                                "{log.status_code}" 
                                            }
                                        }
                                    }
                                }
                            },
                            _ => rsx! { div { class: "text-xs opacity-30 italic", "No signals detected." } }
                        }
                    }
                    
                    div { class: "mt-auto pt-4",
                        button { class: "btn btn-ghost btn-xs w-full text-[10px] opacity-30 hover:opacity-100 uppercase tracking-widest", 
                            onclick: move |_| logs.restart(),
                            "Refresh" 
                        }
                    }
                }
            }
        }
    }
}
