use dioxus::prelude::*;
use burncloud_client_shared::log_service::LogService;
use burncloud_client_shared::usage_service::UsageService;
use burncloud_client_shared::monitor_service::MonitorService;

#[component]
pub fn Dashboard() -> Element {
    let logs = use_resource(move || async move {
        // Return Option<Vec<LogEntry>>
        LogService::list(10).await.ok()
    });

    let usage = use_resource(move || async move {
        // Hardcoded demo-user for now
        UsageService::get_user_usage("demo-user").await.ok()
    });

    let monitor = use_resource(move || async move {
        MonitorService::get_system_metrics().await.ok()
    });

    rsx! {
        div { class: "page-header",
            h1 { class: "text-large-title font-bold text-primary m-0", "仪表盘" }
            p { class: "text-secondary m-0 mt-sm", "BurnCloud 大模型本地部署平台概览" }
        }

        div { class: "page-content",
            div { class: "grid",
                style: "grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: var(--spacing-xl);",

                div { class: "card metric-card",
                    div { class: "metric-header",
                        h3 { class: "text-subtitle font-semibold m-0", "系统状态" }
                        span { class: "status-indicator status-running", span { class: "status-dot" }, "运行正常" }
                    }
                    div { class: "flex flex-col gap-md",
                        match &*monitor.read() {
                            Some(Some(m)) => rsx! {
                                div { class: "flex justify-between items-center", 
                                    span { class: "metric-label", "CPU使用率" } 
                                    span { class: "metric-value text-subtitle", "{m.cpu.usage_percent:.1}%" } 
                                }
                                div { class: "progress", div { class: "progress-fill", style: "width: {m.cpu.usage_percent}%" } }
                                
                                div { class: "flex justify-between items-center mt-sm", 
                                    span { class: "metric-label", "内存" } 
                                    span { class: "metric-value text-secondary", 
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

                div { class: "card metric-card",
                    div { class: "metric-header",
                        h3 { class: "text-subtitle font-semibold m-0", "模型状态" }
                        span { class: "text-primary font-medium", "2个运行中" }
                    }
                    div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center",
                            div { class: "flex items-center gap-sm", span { "🧠" }, span { class: "font-medium", "Qwen2.5-7B" } }
                            span { class: "status-indicator status-running", span { class: "status-dot" }, "运行中" }
                        }
                    }
                }
                
                div { class: "card metric-card",
                    div { class: "metric-header",
                        h3 { class: "text-subtitle font-semibold m-0", "Token 消耗" }
                        span { class: "text-secondary", "demo-user" }
                    }
                    div { class: "flex flex-col gap-md",
                        match &*usage.read() {
                            Some(Some(stats)) => rsx! {
                                div { class: "flex justify-between items-center",
                                    span { class: "metric-label", "Total Tokens" }
                                    span { class: "metric-value", "{stats.total_tokens}" }
                                }
                                div { class: "flex justify-between items-center",
                                    span { class: "metric-label", "Prompt" }
                                    span { class: "metric-value text-secondary", "{stats.prompt_tokens}" }
                                }
                                div { class: "flex justify-between items-center",
                                    span { class: "metric-label", "Completion" }
                                    span { class: "metric-value text-secondary", "{stats.completion_tokens}" }
                                }
                            },
                            Some(None) => rsx! { div { class: "text-secondary", "暂无数据" } },
                            None => rsx! { div { "加载中..." } }
                        }
                    }
                }
                
                div { class: "card metric-card",
                    div { class: "metric-header", h3 { class: "text-subtitle font-semibold m-0", "存储使用" } }
                    div { class: "flex flex-col gap-md",
                        match &*monitor.read() {
                            Some(Some(m)) => {
                                let disk = m.disks.first();
                                match disk {
                                    Some(d) => rsx! {
                                        div { class: "flex justify-between items-center", 
                                            span { class: "metric-label", "磁盘 ({d.mount_point})" }, 
                                            span { class: "metric-value text-subtitle", "{d.used / 1024 / 1024 / 1024}GB / {d.total / 1024 / 1024 / 1024}GB" } 
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

            div { class: "mt-xxxl",
                h2 { class: "text-title font-semibold mb-lg", "快速操作" }
                div { class: "flex gap-lg",
                    button { class: "btn btn-primary", span { "🚀" }, "部署新模型" }
                    button { class: "btn btn-secondary", span { "🔧" }, "系统设置" }
                }
            }

            div { class: "mt-xxxl",
                h2 { class: "text-title font-semibold mb-lg", "API 调用日志 (Real-time)" }
                div { class: "card",
                    div { class: "p-lg",
                        div { class: "flex flex-col gap-md",
                            match &*logs.read() {
                                Some(Some(list)) => rsx! {
                                    for log in list {
                                        div { class: "flex items-center justify-between",
                                            div { class: "flex items-center gap-md",
                                                span { class: "text-secondary", "{log.request_id.chars().take(8).collect::<String>()}" }
                                                span { class: 
                                                    if log.status_code >= 500 { "status-indicator status-stopped" }
                                                    else if log.status_code >= 400 { "status-indicator status-pending" }
                                                    else { "status-indicator status-running" },
                                                    span { class: "status-dot" }
                                                    "{log.status_code}"
                                                }
                                                span { "{log.path}" }
                                                span { class: "text-secondary text-caption", "{log.latency_ms}ms" }
                                            }
                                            span { class: "text-secondary text-caption", "{log.user_id.clone().unwrap_or_default()}" }
                                        }
                                    }
                                },
                                Some(None) => rsx! { div { class: "text-secondary", "API请求失败 (check logs)" } },
                                None => rsx! { div { class: "text-secondary", "加载中..." } }
                            }
                        }
                    }
                }
            }
        }
    }
}