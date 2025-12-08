use dioxus::prelude::*;
use burncloud_client_shared::log_service::{LogService, LogEntry};
use burncloud_client_shared::usage_service::UsageService;
use burncloud_client_shared::monitor_service::MonitorService;
use burncloud_client_shared::components::{
    BCCard, BCBadge, BadgeVariant, BCTable, BCButton, BCModal, ButtonVariant
};
use serde_json; // Explicitly import if used in RSX, though typically crate name works if in Cargo.toml

#[component]
pub fn Dashboard() -> Element {
    let mut logs = use_resource(move || async move {
        LogService::list(10).await.ok()
    });

    let usage = use_resource(move || async move {
        // Hardcoded demo-user for now
        UsageService::get_user_usage("demo-user").await.ok()
    });

    let monitor = use_resource(move || async move {
        MonitorService::get_system_metrics().await.ok()
    });

    let mut selected_log = use_signal(|| None::<LogEntry>);
    let mut is_log_modal_open = use_signal(|| false);

    let mut open_log_details = move |log: LogEntry| {
        selected_log.set(Some(log));
        is_log_modal_open.set(true);
    };

    // Clone data for RSX to avoid lifetime issues
    let logs_data = logs.read().clone();
    let usage_data = usage.read().clone();
    let monitor_data = monitor.read().clone();

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0", "仪表盘" }
                    p { class: "text-secondary m-0 mt-sm", "BurnCloud 大模型本地部署平台概览" }
                }
                div { class: "flex gap-md",
                    BCButton { 
                        variant: ButtonVariant::Primary,
                        "部署新模型" 
                    }
                    BCButton { 
                        variant: ButtonVariant::Secondary,
                        "系统设置" 
                    }
                }
            }
        }

        div { class: "page-content",
            // Metrics Grid
            div { class: "grid mb-xxxl",
                style: "grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: var(--spacing-xl);",

                // System Status Card
                BCCard {
                    div { class: "flex justify-between items-center mb-md",
                        h3 { class: "text-subtitle font-semibold m-0", "系统状态" }
                        BCBadge { variant: BadgeVariant::Success, dot: true, "运行正常" }
                    }
                    div { class: "flex flex-col gap-md",
                        match &monitor_data {
                            Some(Some(m)) => rsx! {
                                div { class: "flex justify-between items-center", 
                                    span { class: "text-body font-medium", "CPU" } 
                                    span { class: "text-body font-bold", "{m.cpu.usage_percent:.1}%" } 
                                }
                                div { class: "progress", div { class: "progress-fill", style: "width: {m.cpu.usage_percent}%" } }
                                
                                div { class: "flex justify-between items-center mt-sm", 
                                    span { class: "text-body font-medium", "内存" } 
                                    span { class: "text-caption text-secondary", 
                                        "{m.memory.used / 1024 / 1024 / 1024}GB / {m.memory.total / 1024 / 1024 / 1024}GB" 
                                    } 
                                }
                                div { class: "progress", div { class: "progress-fill", style: "width: {m.memory.usage_percent}%" } }
                            },
                            Some(None) => rsx! { div { class: "text-secondary", "暂无数据" } },
                            None => rsx! { div { "加载中..." } }
                        }
                    }
                }

                // Model Status Card
                BCCard {
                    div { class: "flex justify-between items-center mb-md",
                        h3 { class: "text-subtitle font-semibold m-0", "模型状态" }
                        span { class: "text-primary font-medium", "2个运行中" }
                    }
                    div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center",
                            div { class: "flex items-center gap-sm", span { "🧠" }, span { class: "font-medium", "Qwen2.5-7B" } }
                            BCBadge { variant: BadgeVariant::Success, dot: true, "运行中" }
                        }
                    }
                }
                
                // Token Usage Card
                BCCard {
                    div { class: "flex justify-between items-center mb-md",
                        h3 { class: "text-subtitle font-semibold m-0", "Token 消耗" }
                        span { class: "text-secondary", "demo-user" }
                    }
                    div { class: "flex flex-col gap-md",
                        match usage_data {
                            Some(Some(stats)) => rsx! {
                                div { class: "flex justify-between items-center",
                                    span { class: "text-secondary", "Total Tokens" }
                                    span { class: "font-bold text-title", "{stats.total_tokens}" }
                                }
                                div { class: "flex justify-between items-center",
                                    span { class: "text-secondary", "Prompt" }
                                    span { class: "text-secondary", "{stats.prompt_tokens}" }
                                }
                                div { class: "flex justify-between items-center",
                                    span { class: "text-secondary", "Completion" }
                                    span { class: "text-secondary", "{stats.completion_tokens}" }
                                }
                            },
                            Some(None) => rsx! { div { class: "text-secondary", "暂无数据" } },
                            None => rsx! { div { "加载中..." } }
                        }
                    }
                }
                
                // Storage Card
                BCCard {
                    div { class: "flex justify-between items-center mb-md",
                        h3 { class: "text-subtitle font-semibold m-0", "存储使用" }
                    }
                    div { class: "flex flex-col gap-md",
                        match &monitor_data {
                            Some(Some(m)) => {
                                let disk = m.disks.first();
                                match disk {
                                    Some(d) => rsx! {
                                        div { class: "flex justify-between items-center", 
                                            span { class: "text-body", "磁盘 ({d.mount_point})" }, 
                                            span { class: "text-body font-bold", "{d.used / 1024 / 1024 / 1024}GB / {d.total / 1024 / 1024 / 1024}GB" } 
                                        }
                                        div { class: "progress", div { class: "progress-fill", style: "width: {d.usage_percent}%" } }
                                    },
                                    None => rsx! { div { "未检测到磁盘" } }
                                }
                            },
                            _ => rsx! { div { "..." } }
                        }
                    }
                }
            }

            // Real-time Logs Table
            div { class: "mt-xxxl",
                div { class: "flex justify-between items-center mb-lg",
                    h2 { class: "text-title font-semibold m-0", "API 调用日志 (Real-time)" }
                    BCButton { variant: ButtonVariant::Ghost, onclick: move |_| logs.restart(), "🔄 刷新" }
                }
                
                BCCard {
                    class: "p-0 overflow-hidden",
                    BCTable {
                        thead {
                            tr {
                                th { "Request ID" }
                                th { "Status" }
                                th { "Path" }
                                th { "Latency" }
                                th { "User" }
                                th { class: "text-right", "Action" }
                            }
                        }
                        tbody {
                            match logs_data {
                                Some(Some(list)) => rsx! {
                                    for log in list {
                                        tr { class: "hover:bg-subtle transition-colors",
                                            td { class: "text-secondary text-caption", "{log.request_id.chars().take(8).collect::<String>()}" }
                                            td {
                                                if log.status_code >= 500 {
                                                    BCBadge { variant: BadgeVariant::Danger, dot: true, "{log.status_code}" }
                                                } else if log.status_code >= 400 {
                                                    BCBadge { variant: BadgeVariant::Warning, dot: true, "{log.status_code}" }
                                                } else {
                                                    BCBadge { variant: BadgeVariant::Success, dot: true, "{log.status_code}" }
                                                }
                                            }
                                            td { class: "text-body font-medium", "{log.path}" }
                                            td { class: "text-secondary", "{log.latency_ms}ms" }
                                            td { class: "text-secondary", "{log.user_id.clone().unwrap_or_default()}" }
                                            td { class: "text-right",
                                                BCButton { 
                                                    variant: ButtonVariant::Ghost,
                                                    class: "text-primary",
                                                    onclick: move |_| open_log_details(log.clone()),
                                                    "Details" 
                                                }
                                            }
                                        }
                                    }
                                },
                                Some(None) => rsx! { tr { td { colspan: "6", class: "p-xl text-center text-secondary", "暂无数据" } } },
                                None => rsx! { tr { td { colspan: "6", class: "p-xl text-center text-secondary", "加载中..." } } }
                            }
                        }
                    }
                }
            }

            // Log Details Modal
            BCModal {
                open: is_log_modal_open(),
                title: "请求详情".to_string(),
                onclose: move |_| is_log_modal_open.set(false),
                
                if let Some(log) = &*selected_log.read() {
                    div { class: "flex flex-col gap-md",
                        div { class: "grid grid-cols-2 gap-md",
                            div {
                                span { class: "text-caption text-secondary block", "Request ID" }
                                span { class: "font-mono text-sm", "{log.request_id}" }
                            }
                            div {
                                span { class: "text-caption text-secondary block", "User ID" }
                                span { class: "font-mono text-sm", "{log.user_id.clone().unwrap_or_default()}" }
                            }
                            div {
                                span { class: "text-caption text-secondary block", "Status Code" }
                                span { class: "font-mono text-sm", "{log.status_code}" }
                            }
                            div {
                                span { class: "text-caption text-secondary block", "Latency" }
                                span { class: "font-mono text-sm", "{log.latency_ms} ms" }
                            }
                        }
                        
                        div {
                            span { class: "text-caption text-secondary block mb-xs", "Path" }
                            div { class: "p-sm bg-surface-variant rounded font-mono text-sm break-all", "{log.path}" }
                        }

                        div {
                            span { class: "text-caption text-secondary block mb-xs", "Raw JSON (Snapshot)" }
                            div { class: "p-md bg-surface-variant rounded font-mono text-xs overflow-auto", style: "max-height: 200px;",
                                pre { class: "m-0",
                                    "{serde_json::to_string_pretty(log).unwrap_or_default()}"
                                }
                            }
                        }
                    }
                } else {
                    div { "No log selected" }
                }
                
                div { class: "modal-footer",
                    BCButton { 
                        variant: ButtonVariant::Primary, 
                        onclick: move |_| is_log_modal_open.set(false), 
                        "关闭" 
                    }
                }
            }
        }
    }
}
