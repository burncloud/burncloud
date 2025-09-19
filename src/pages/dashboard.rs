use dioxus::prelude::*;
use crate::types::{SystemStatus, ModelInfo, ModelStatus};
use crate::components::{sidebar::Sidebar, statusbar::StatusBar};

#[component]
pub fn Dashboard() -> Element {
    let system_status = use_signal(|| SystemStatus::default());
    let models = use_signal(|| vec![
        ModelInfo {
            id: "qwen2.5-7b".to_string(),
            name: "Qwen2.5-7B-Chat".to_string(),
            version: "v1.2".to_string(),
            size: 4_398_046_511_104, // 4.1GB
            status: ModelStatus::Running,
            port: Some(8001),
            memory_usage: Some(1_288_490_188_800), // 1.2GB
            requests_count: 142,
            avg_response_time: 1.2,
            description: "对话专用模型".to_string(),
            tags: vec!["chat".to_string(), "chinese".to_string()],
        },
        ModelInfo {
            id: "deepseek-v2".to_string(),
            name: "DeepSeek-V2-Chat".to_string(),
            version: "v2.0".to_string(),
            size: 7_301_444_403_200, // 6.8GB
            status: ModelStatus::Stopped,
            port: Some(8002),
            memory_usage: None,
            requests_count: 0,
            avg_response_time: 0.0,
            description: "代码生成模型".to_string(),
            tags: vec!["code".to_string(), "chat".to_string()],
        },
    ]);

    let status = system_status.read();
    let _running_models = models.read().iter().filter(|m| matches!(m.status, ModelStatus::Running)).count();

    rsx! {
        div { 
            class: "app-container",
            div { 
                class: "main-content",
                Sidebar {}
                div { 
                    class: "content-area",
                    div {
                        class: "models-page",
                        div { class: "page-header",
                            h1 { "🧠 仪表盘" }
                            div { class: "header-actions",
                                button { class: "btn primary", "➕ 添加模型" }
                                button { class: "btn secondary", "🔄 刷新" }
                                button { class: "btn secondary", "📁 浏览本地" }
                                div { class: "search-box",
                                    input {
                                        r#type: "text",
                                        placeholder: "搜索功能...",
                                    }
                                }
                            }
                        }

                        div {
                            class: "dashboard",
                            div { class: "dashboard-grid",
                            // 系统状态卡片
                            div { class: "card system-status",
                                h3 { "📊 系统状态" }
                                div { class: "status-grid",
                                    div { class: "status-item",
                                        span { class: "label", "CPU使用率" }
                                        div { class: "progress-bar",
                                            div { 
                                                class: "progress-fill",
                                                style: "width: {status.cpu_usage}%",
                                            }
                                        }
                                        span { class: "value", "{status.cpu_usage:.1}%" }
                                    }
                                    div { class: "status-item",
                                        span { class: "label", "内存使用" }
                                        div { class: "progress-bar",
                                            div { 
                                                class: "progress-fill",
                                                style: "width: {(status.memory_used as f64 / status.memory_total as f64 * 100.0)}%",
                                            }
                                        }
                                        span { class: "value", "{format_bytes(status.memory_used)}/{format_bytes(status.memory_total)}" }
                                    }
                                    div { class: "status-item",
                                        span { class: "label", "GPU显存" }
                                        div { class: "progress-bar",
                                            div { 
                                                class: "progress-fill",
                                                style: "width: {(status.gpu_memory_used as f64 / status.gpu_memory_total as f64 * 100.0)}%",
                                            }
                                        }
                                        span { class: "value", "{format_bytes(status.gpu_memory_used)}/{format_bytes(status.gpu_memory_total)}" }
                                    }
                                    div { class: "status-item",
                                        span { class: "label", "磁盘空间" }
                                        div { class: "progress-bar",
                                            div { 
                                                class: "progress-fill",
                                                style: "width: {(status.disk_used as f64 / status.disk_total as f64 * 100.0)}%",
                                            }
                                        }
                                        span { class: "value", "{format_bytes(status.disk_used)}/{format_bytes(status.disk_total)}" }
                                    }
                                }
                            }

                            // 模型状态卡片
                            div { class: "card model-status",
                                h3 { "🚀 模型状态" }
                                div { class: "model-list",
                                    for model in models.read().iter() {
                                        div { class: "model-item",
                                            div { class: "model-info",
                                                span { class: "model-name", "{model.name}" }
                                                span { 
                                                    class: match model.status {
                                                        ModelStatus::Running => "status running",
                                                        ModelStatus::Stopped => "status stopped",
                                                        _ => "status other",
                                                    },
                                                    {match model.status {
                                                        ModelStatus::Running => "●运行中",
                                                        ModelStatus::Stopped => "○已停止",
                                                        ModelStatus::Starting => "◐启动中",
                                                        ModelStatus::Stopping => "◑停止中",
                                                        ModelStatus::Error(_) => "✕错误",
                                                    }}
                                                }
                                            }
                                            div { class: "model-stats",
                                                if matches!(model.status, ModelStatus::Running) {
                                                    span { "📈请求: {model.requests_count}" }
                                                    span { "⚡响应: {model.avg_response_time:.1}s" }
                                                } else {
                                                    span { "📈请求: --" }
                                                    span { "⚡响应: --" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            // 最近活动卡片
                            div { class: "card recent-activity",
                                h3 { "📝 最近活动" }
                                div { class: "activity-list",
                                    div { class: "activity-item",
                                        span { class: "time", "09:45:32" }
                                        span { class: "message", "Qwen2.5-7B 启动成功" }
                                    }
                                    div { class: "activity-item",
                                        span { class: "time", "09:44:15" }
                                        span { class: "message", "系统启动完成" }
                                    }
                                    div { class: "activity-item",
                                        span { class: "time", "09:43:01" }
                                        span { class: "message", "检测到 GPU 加速支持" }
                                    }
                                }
                            }
                        }
                        }
                    }
                }
            }
            StatusBar { system_status }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    
    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else {
        format!("{}B", bytes)
    }
}