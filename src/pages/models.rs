use dioxus::prelude::*;
use crate::types::{ModelInfo, ModelStatus};
use crate::components::{sidebar::Sidebar, statusbar::StatusBar};

#[component]
pub fn Models() -> Element {
    let installed_models = use_signal(|| vec![
        ModelInfo {
            id: "qwen2.5-7b".to_string(),
            name: "Qwen2.5-7B-Chat".to_string(),
            version: "v1.2".to_string(),
            size: 4_398_046_511_104,
            status: ModelStatus::Running,
            port: Some(8001),
            memory_usage: Some(1_288_490_188_800),
            requests_count: 142,
            avg_response_time: 1.2,
            description: "对话专用模型".to_string(),
            tags: vec!["chat".to_string(), "chinese".to_string()],
        },
        ModelInfo {
            id: "deepseek-v2".to_string(),
            name: "DeepSeek-V2-Chat".to_string(),
            version: "v2.0".to_string(),
            size: 7_301_444_403_200,
            status: ModelStatus::Stopped,
            port: Some(8002),
            memory_usage: None,
            requests_count: 0,
            avg_response_time: 0.0,
            description: "代码生成模型".to_string(),
            tags: vec!["code".to_string(), "chat".to_string()],
        },
    ]);

    let mut search_query = use_signal(String::new);

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
                            h1 { "🧠 模型管理" }
                            div { class: "header-actions",
                                button { class: "btn primary", "➕ 添加模型" }
                                button { class: "btn secondary", "🔄 刷新" }
                                button { class: "btn secondary", "📁 浏览本地" }
                                div { class: "search-box",
                                    input { 
                                        r#type: "text", 
                                        placeholder: "搜索模型...",
                                        value: "{search_query}",
                                        oninput: move |evt| search_query.set(evt.value()),
                                    }
                                }
                            }
                        }

                        // 已安装模型
                        div { class: "models-section",
                            h2 { "已安装模型 ({installed_models.read().len()})" }
                            div { class: "models-grid",
                                for model in installed_models.read().iter() {
                                    div { class: "model-card installed",
                                        div { class: "model-header",
                                            div { class: "model-info",
                                                span { class: "model-icon", "🧠" }
                                                div { class: "model-details",
                                                    h3 { class: "model-name", "{model.name}" }
                                                    p { class: "model-meta", "版本: {model.version}  大小: {format_bytes(model.size)}" }
                                                    if let Some(port) = model.port {
                                                        p { class: "model-meta", "端口: {port}" }
                                                    }
                                                    if let Some(memory) = model.memory_usage {
                                                        p { class: "model-meta", "内存: {format_bytes(memory)}" }
                                                    }
                                                }
                                            }
                                            div { class: "model-status",
                                                span { 
                                                    class: match model.status {
                                                        ModelStatus::Running => "status-indicator running",
                                                        ModelStatus::Stopped => "status-indicator stopped",
                                                        _ => "status-indicator other",
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
                                        }
                                        div { class: "model-actions",
                                            {match model.status {
                                                ModelStatus::Running => rsx! {
                                                    button { class: "btn action stop", "停止" }
                                                },
                                                ModelStatus::Stopped => rsx! {
                                                    button { class: "btn action start", "启动" }
                                                },
                                                _ => rsx! { div {} }
                                            }}
                                            button { class: "btn action config", "配置" }
                                            button { class: "btn action delete", "删除" }
                                        }
                                        if matches!(model.status, ModelStatus::Running) {
                                            div { class: "model-stats",
                                                span { "📈 请求: {model.requests_count}" }
                                                span { "⚡ 响应: {model.avg_response_time:.1}s" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            StatusBar { system_status: use_signal(|| crate::types::SystemStatus::default()) }
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