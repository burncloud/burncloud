use burncloud_client_shared::components::logo::Logo;
use burncloud_client_shared::i18n::use_i18n;
use burncloud_client_shared::log_service::LogService;
use burncloud_client_shared::monitor_service::MonitorService;
use burncloud_client_shared::usage_service::UsageService;
use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    let i18n = use_i18n();
    let _lang = i18n.language.read();

    let logs = use_resource(move || async move { LogService::list(5).await.ok() });

    let usage =
        use_resource(move || async move { UsageService::get_user_usage("demo-user").await.ok() });

    let monitor =
        use_resource(move || async move { MonitorService::get_system_metrics().await.ok() });

    let _logs_data = logs.read().clone();
    let _usage_data = usage.read().clone();
    let _monitor_data = monitor.read().clone();

    rsx! {
        div { class: "h-full flex flex-col max-w-6xl mx-auto animate-in select-none",

            // 1. Enterprise Header
            div { class: "flex items-end justify-between mb-xxxl",
                div {
                    div { class: "flex items-center gap-3 mb-md",
                        Logo { class: "w-8 h-8 fill-current" }
                        span { class: "font-semibold text-lg tracking-tight text-primary", "BurnCloud" }
                    }
                    h1 { class: "text-4xl font-bold tracking-tight text-primary",
                        "企业控制台"
                    }
                    div { class: "flex items-center gap-sm mt-md pl-xs",
                        div { class: "w-2 h-2 rounded-full animate-pulse-soft",
                            style: "background: var(--bc-success); box-shadow: 0 0 8px rgba(52, 199, 89, 0.6);"
                        }
                        span { class: "text-caption font-medium tracking-wide text-tertiary uppercase", "实时交易流" }
                    }
                }
            }

            // 2. Main Content Grid
            div { class: "grid grid-cols-1 lg:grid-cols-3 gap-xxxl h-full",

                // Left Column: Financials & Infrastructure (Spans 2 columns)
                div { class: "lg:col-span-2 flex flex-col gap-xxxl",

                    // Financial Hero Section - The "Money"
                    div { class: "grid grid-cols-2 gap-xxxl pb-xxl border-b",
                        div {
                            div { class: "text-xxs uppercase tracking-[0.2em] font-bold text-tertiary mb-sm", "今日流水 (USD)" }
                            div { class: "text-5xl font-bold tabular-nums tracking-tighter text-primary", "$128,432.00" }
                            div { class: "text-caption font-medium mt-sm flex items-center gap-xs",
                                style: "color: var(--bc-success);",
                                span { "▲ 12.4%" }
                                span { class: "text-tertiary", "vs yesterday" }
                            }
                        }
                        div {
                            div { class: "text-xxs uppercase tracking-[0.2em] font-bold text-tertiary mb-sm", "预计营收 (USD)" }
                            div { class: "text-5xl font-bold tracking-tighter", style: "color: var(--bc-success);", "$160,540.00" }
                            div { class: "text-caption font-medium text-tertiary mt-sm", "净利率: 20.0%" }
                        }
                    }

                    // Cloud Provider Health - The "Assets"
                    div { class: "flex flex-col gap-md",
                        div { class: "text-xxs uppercase tracking-[0.2em] font-bold text-tertiary mb-sm", "供应商健康度" }

                        div { class: "grid grid-cols-3 gap-md",
                            // AWS Card
                            div { class: "bc-card-solid p-lg cursor-pointer",
                                style: "transition: border-color var(--bc-transition-fast);",
                                div { class: "flex justify-between items-start mb-lg",
                                    span { class: "font-bold text-lg text-primary", "AWS" }
                                    div { class: "status-dot",
                                        style: "background: var(--bc-success); animation: pulse 2s infinite;"
                                    }
                                }
                                div { class: "text-2xl font-bold tabular-nums text-primary", "1,204" }
                                div { class: "text-xxs text-tertiary uppercase tracking-wider mt-xs", "Active Accounts" }
                            }

                            // Google Cloud Card
                            div { class: "bc-card-solid p-lg cursor-pointer",
                                style: "transition: border-color var(--bc-transition-fast);",
                                div { class: "flex justify-between items-start mb-lg",
                                    span { class: "font-bold text-lg text-primary", "Google" }
                                    div { class: "status-dot",
                                        style: "background: var(--bc-success); animation: pulse 2s infinite;"
                                    }
                                }
                                div { class: "text-2xl font-bold tabular-nums text-primary", "892" }
                                div { class: "text-xxs text-tertiary uppercase tracking-wider mt-xs", "Active Accounts" }
                            }

                            // Azure Card
                            div { class: "bc-card-solid p-lg cursor-pointer",
                                style: "transition: border-color var(--bc-transition-fast);",
                                div { class: "flex justify-between items-start mb-lg",
                                    span { class: "font-bold text-lg text-primary", "Azure" }
                                    div { class: "status-dot animate-pulse",
                                        style: "background: var(--bc-warning);"
                                    }
                                }
                                div { class: "text-2xl font-bold tabular-nums text-primary", "450" }
                                div { class: "text-xxs text-tertiary uppercase tracking-wider mt-xs", "3 Flags Detected" }
                            }
                        }
                    }
                }

                // Right Column: Risk Radar (Activity)
                div { class: "flex flex-col pl-xxl",
                    h3 { class: "text-xxs font-bold uppercase tracking-[0.2em] mb-xxxl",
                        style: "color: var(--bc-danger);",
                        "风控预警 (RISK ALERTS)"
                    }

                    div { class: "flex-1 flex flex-col gap-md",
                        // Mock Alerts for Enterprise Demo
                        div { class: "relative pl-md py-xs",
                            style: "border-left: 2px solid var(--bc-warning);",
                            div { class: "text-caption font-bold text-primary", "Azure Account #8821" }
                            div { class: "text-xxs text-secondary mt-xs", "Quota Usage > 90%" }
                        }
                        div { class: "relative pl-md py-xs",
                            style: "border-left: 2px solid var(--bc-danger);",
                            div { class: "text-caption font-bold text-primary", "GCP-US-EAST-1" }
                            div { class: "text-xxs text-secondary mt-xs", "API Response Timeout (500ms)" }
                        }
                         div { class: "relative pl-md py-xs text-tertiary",
                            style: "border-left: 2px solid var(--bc-border);",
                            div { class: "text-caption font-bold text-primary", "AWS IAM Policy Update" }
                            div { class: "text-xxs text-secondary mt-xs", "Routine Security Check" }
                        }
                    }

                    div { class: "mt-auto pt-lg p-lg bc-card-solid",
                        style: "border-radius: var(--bc-radius-md);",
                        div { class: "text-xxs font-bold text-tertiary uppercase mb-sm", "System Status" }
                        div { class: "flex items-center gap-sm",
                            div { class: "status-dot",
                                style: "background: var(--bc-success);"
                            }
                            span { class: "text-caption font-medium text-primary", "All Gateways Operational" }
                        }
                    }
                }
            }
        }
    }
}
