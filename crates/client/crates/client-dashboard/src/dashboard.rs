use burncloud_client_shared::i18n::{t, use_i18n};
use burncloud_client_shared::log_service::LogService;
use burncloud_client_shared::monitor_service::MonitorService;
use burncloud_client_shared::usage_service::UsageService;
use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    let i18n = use_i18n();
    let lang = i18n.language.read();

    let mut logs = use_resource(move || async move { LogService::list(5).await.ok() });

    let usage =
        use_resource(move || async move { UsageService::get_user_usage("demo-user").await.ok() });

    let monitor =
        use_resource(move || async move { MonitorService::get_system_metrics().await.ok() });

    let logs_data = logs.read().clone();
    let usage_data = usage.read().clone();
    let monitor_data = monitor.read().clone();

        rsx! {
            div { class: "h-full flex flex-col max-w-6xl mx-auto animate-fade-in select-none",
                
                // 1. Enterprise Header
                div { class: "flex items-end justify-between mb-12",
                    div {
                        h1 { class: "text-4xl font-bold tracking-tight text-base-content/90", 
                            "企业控制台" 
                        }
                        div { class: "flex items-center gap-2 mt-3 pl-1",
                            div { class: "w-2 h-2 rounded-full bg-green-500 shadow-[0_0_8px_rgba(34,197,94,0.6)] animate-pulse" }
                            span { class: "text-xs font-medium tracking-wide opacity-50 uppercase", "实时交易流" }
                        }
                    }
                    div {
                        button { class: "btn btn-neutral text-white btn-sm rounded-full px-6 font-medium shadow-sm hover:shadow-md transition-all normal-case border-none", 
                            "+ 添加云账号" 
                        }
                    }
                }
    
                // 2. Main Content Grid
                div { class: "grid grid-cols-1 lg:grid-cols-3 gap-12 h-full",
                    
                    // Left Column: Financials & Infrastructure (Spans 2 columns)
                    div { class: "lg:col-span-2 flex flex-col gap-10",
                        
                        // Financial Hero Section - The "Money"
                        div { class: "grid grid-cols-2 gap-12 pb-8 border-b border-base-200/50",
                            div {
                                div { class: "text-[10px] uppercase tracking-[0.2em] font-bold opacity-40 mb-2", "今日流水 (USD)" }
                                div { class: "text-5xl font-bold tabular-nums tracking-tighter text-base-content/90", "$128,432.00" }
                                div { class: "text-xs font-medium text-success mt-2 flex items-center gap-1", 
                                    span { "▲ 12.4%" }
                                    span { class: "opacity-50", "vs yesterday" }
                                }
                            }
                            div {
                                div { class: "text-[10px] uppercase tracking-[0.2em] font-bold opacity-40 mb-2", "预计营收 (USD)" }
                                div { class: "text-5xl font-bold tracking-tighter text-success/90", "$160,540.00" }
                                div { class: "text-xs font-medium text-base-content/40 mt-2", "净利率: 20.0%" }
                            }
                        }
    
                        // Cloud Provider Health - The "Assets"
                        div { class: "flex flex-col gap-4",
                            div { class: "text-[10px] uppercase tracking-[0.2em] font-bold opacity-40 mb-2", "供应商健康度" }
                            
                            div { class: "grid grid-cols-3 gap-4",
                                // AWS Card
                                div { class: "p-4 bg-base-200/30 rounded-xl border border-base-200 hover:border-orange-500/30 transition-colors group cursor-pointer",
                                    div { class: "flex justify-between items-start mb-4",
                                        span { class: "font-bold text-lg", "AWS" }
                                        div { class: "w-2 h-2 rounded-full bg-success" }
                                    }
                                    div { class: "text-2xl font-bold tabular-nums", "1,204" }
                                    div { class: "text-[10px] opacity-40 uppercase tracking-wider mt-1", "Active Accounts" }
                                }
                                
                                // Google Cloud Card
                                div { class: "p-4 bg-base-200/30 rounded-xl border border-base-200 hover:border-blue-500/30 transition-colors group cursor-pointer",
                                    div { class: "flex justify-between items-start mb-4",
                                        span { class: "font-bold text-lg", "Google" }
                                        div { class: "w-2 h-2 rounded-full bg-success" }
                                    }
                                    div { class: "text-2xl font-bold tabular-nums", "892" }
                                    div { class: "text-[10px] opacity-40 uppercase tracking-wider mt-1", "Active Accounts" }
                                }
    
                                // Azure Card
                                div { class: "p-4 bg-base-200/30 rounded-xl border border-base-200 hover:border-sky-500/30 transition-colors group cursor-pointer",
                                    div { class: "flex justify-between items-start mb-4",
                                        span { class: "font-bold text-lg", "Azure" }
                                        div { class: "w-2 h-2 rounded-full bg-warning animate-pulse" } // Warning state demo
                                    }
                                    div { class: "text-2xl font-bold tabular-nums", "450" }
                                    div { class: "text-[10px] opacity-40 uppercase tracking-wider mt-1", "3 Flags Detected" }
                                }
                            }
                        }
                    }
    
                    // Right Column: Risk Radar (Activity)
                    div { class: "flex flex-col pl-8", 
                        h3 { class: "text-[10px] font-bold opacity-30 uppercase tracking-[0.2em] mb-8 text-error", "风控预警 (RISK ALERTS)" } 
                        
                        div { class: "flex-1 flex flex-col gap-4",
                            // Mock Alerts for Enterprise Demo
                            div { class: "relative pl-4 border-l-2 border-warning py-1",
                                div { class: "text-xs font-bold text-base-content/80", "Azure Account #8821" }
                                div { class: "text-[10px] opacity-60 mt-0.5", "Quota Usage > 90%" }
                            }
                            div { class: "relative pl-4 border-l-2 border-error py-1",
                                div { class: "text-xs font-bold text-base-content/80", "GCP-US-EAST-1" }
                                div { class: "text-[10px] opacity-60 mt-0.5", "API Response Timeout (500ms)" }
                            }
                             div { class: "relative pl-4 border-l-2 border-base-200 py-1 opacity-50",
                                div { class: "text-xs font-bold text-base-content/80", "AWS IAM Policy Update" }
                                div { class: "text-[10px] opacity-60 mt-0.5", "Routine Security Check" }
                            }
                        }
                        
                        div { class: "mt-auto pt-4 p-4 bg-base-200/30 rounded-xl",
                            div { class: "text-[10px] font-bold opacity-40 uppercase mb-2", "System Status" }
                            div { class: "flex items-center gap-2",
                                div { class: "badge badge-success badge-xs" }
                                span { class: "text-xs font-medium", "All Gateways Operational" }
                            }
                        }
                    }
                }
            }
        }}
