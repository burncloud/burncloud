use dioxus::prelude::*;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Debug, Clone, PartialEq)]
struct LogEntry {
    request_id: String,
    user_id: Option<String>,
    path: String,
    status_code: u16,
    latency_ms: i64,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
struct UsageStats {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
}

#[component]
pub fn Dashboard() -> Element {
    let logs = use_resource(move || async move {
        let client = reqwest::Client::new();
        let url = "http://127.0.0.1:3000/console/api/logs?limit=10"; 
        match client.get(url).send().await {
             Ok(resp) => {
                 if let Ok(json) = resp.json::<Value>().await {
                     if let Some(_arr) = json["data"].as_array() {
                         return serde_json::from_value::<Vec<LogEntry>>(json["data"].clone()).ok();
                     }
                 }
                 None
             },
             Err(_) => None
        }
    });

    let usage = use_resource(move || async move {
        let client = reqwest::Client::new();
        // Hardcoded demo-user for now
        let url = "http://127.0.0.1:3000/console/api/usage/demo-user";
        client.get(url).send().await.ok()?.json::<UsageStats>().await.ok()
    });

    rsx! {
        div { class: "page-header",
            h1 { class: "text-large-title font-bold text-primary m-0",
                "仪表盘"
            }
            p { class: "text-secondary m-0 mt-sm",
                "BurnCloud 大模型本地部署平台概览"
            }
        }

        div { class: "page-content",
            div { class: "grid",
                style: "grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: var(--spacing-xl);",

                // 系统状态卡片 (Static)
                div { class: "card metric-card",
                    div { class: "metric-header",
                        h3 { class: "text-subtitle font-semibold m-0", "系统状态" }
                        span { class: "status-indicator status-running",
                            span { class: "status-dot" }
                            "运行正常"
                        }
                    }
                    div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "CPU使用率" }
                            span { class: "metric-value text-subtitle", "45.2%" }
                        }
                        div { class: "progress",
                            div { class: "progress-fill", style: "width: 45.2%" }
                        }
                    }
                }

                // 模型状态卡片 (Static)
                div { class: "card metric-card",
                    div { class: "metric-header",
                        h3 { class: "text-subtitle font-semibold m-0", "模型状态" }
                        span { class: "text-primary font-medium", "2个运行中" }
                    }
                    div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center",
                            div { class: "flex items-center gap-sm",
                                span { "🧠" }
                                span { class: "font-medium", "Qwen2.5-7B" }
                            }
                            span { class: "status-indicator status-running",
                                span { class: "status-dot" }
                                "运行中"
                            }
                        }
                    }
                }
                
                 // API统计卡片 (Dynamic Usage)
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
                            _ => rsx! { div { "加载中..." } }
                        }
                    }
                }
                
                // 存储使用卡片 (Static)
                 div { class: "card metric-card",
                    div { class: "metric-header",
                        h3 { class: "text-subtitle font-semibold m-0", "存储使用" }
                    }
                     div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "模型文件" }
                            span { class: "metric-value text-subtitle", "23.4GB" }
                        }
                    }
                }
            }

            // 快速操作区域 (Static)
            div { class: "mt-xxxl",
                h2 { class: "text-title font-semibold mb-lg", "快速操作" }
                div { class: "flex gap-lg",
                    button { class: "btn btn-primary",
                         span { "🚀" }
                         "部署新模型"
                    }
                     button { class: "btn btn-secondary",
                        span { "🔧" }
                        "系统设置"
                    }
                }
            }

            // API 调用日志 (Dynamic)
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
                                Some(None) => rsx! { div { class: "text-secondary", "暂无日志或加载失败 (Server 4000 not running?)" } },
                                None => rsx! { div { class: "text-secondary", "加载中..." } }
                            }
                        }
                    }
                }
            }
        }
    }
}
