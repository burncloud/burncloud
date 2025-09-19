use dioxus::prelude::*;

#[component]
pub fn ModelManagement() -> Element {
    let mut search_term = use_signal(|| String::new());

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0",
                        "模型管理"
                    }
                    p { class: "text-secondary m-0 mt-sm",
                        "管理和部署大语言模型"
                    }
                }
                div { class: "flex gap-md",
                    button { class: "btn btn-secondary",
                        span { "🔄" }
                        "刷新"
                    }
                    button { class: "btn btn-secondary",
                        span { "📁" }
                        "浏览本地"
                    }
                    button { class: "btn btn-primary",
                        span { "+" }
                        "添加模型"
                    }
                }
            }
            div { class: "mt-lg",
                input {
                    class: "input",
                    style: "max-width: 400px;",
                    placeholder: "搜索模型...",
                    value: "{search_term}",
                    oninput: move |evt| search_term.set(evt.value())
                }
            }
        }

        div { class: "page-content",
            // 已安装模型
            div { class: "mb-xxxl",
                h2 { class: "text-title font-semibold mb-lg",
                    "已安装模型 (2)"
                }
                div { class: "grid gap-lg",
                    style: "grid-template-columns: 1fr;",
                    // Qwen2.5-7B 模型卡片
                    div { class: "card model-card",
                        div { class: "model-header",
                            div { class: "model-title",
                                span { style: "font-size: 20px;", "🧠" }
                                div {
                                    div { class: "text-subtitle font-semibold", "Qwen2.5-7B-Chat" }
                                    div { class: "text-caption text-secondary", "阿里巴巴通义千问 7B 对话模型" }
                                }
                            }
                            div { class: "flex items-center gap-md",
                                span { class: "status-indicator status-running",
                                    span { class: "status-dot" }
                                    "运行中"
                                }
                                div { class: "model-actions",
                                    button { class: "btn btn-secondary", "停止" }
                                    button { class: "btn btn-subtle", "配置" }
                                    button { class: "btn btn-subtle", "删除" }
                                }
                            }
                        }
                        div { class: "model-details",
                            div {
                                div { class: "metric-label", "版本" }
                                div { class: "font-medium", "v1.2" }
                            }
                            div {
                                div { class: "metric-label", "大小" }
                                div { class: "font-medium", "4.1GB" }
                            }
                            div {
                                div { class: "metric-label", "端口" }
                                div { class: "font-medium", "8001" }
                            }
                            div {
                                div { class: "metric-label", "内存使用" }
                                div { class: "font-medium", "1.2GB" }
                            }
                            div {
                                div { class: "metric-label", "评分" }
                                div { class: "font-medium", "⭐4.8" }
                            }
                            div {
                                div { class: "metric-label", "类型" }
                                div { class: "font-medium", "💬对话专用" }
                            }
                        }
                    }

                    // DeepSeek-V2 模型卡片
                    div { class: "card model-card",
                        div { class: "model-header",
                            div { class: "model-title",
                                span { style: "font-size: 20px;", "🤖" }
                                div {
                                    div { class: "text-subtitle font-semibold", "DeepSeek-V2-Chat" }
                                    div { class: "text-caption text-secondary", "DeepSeek 深度求索第二代对话模型" }
                                }
                            }
                            div { class: "flex items-center gap-md",
                                span { class: "status-indicator status-stopped",
                                    span { class: "status-dot" }
                                    "已停止"
                                }
                                div { class: "model-actions",
                                    button { class: "btn btn-primary", "启动" }
                                    button { class: "btn btn-subtle", "配置" }
                                    button { class: "btn btn-subtle", "删除" }
                                }
                            }
                        }
                        div { class: "model-details",
                            div {
                                div { class: "metric-label", "版本" }
                                div { class: "font-medium", "v2.0" }
                            }
                            div {
                                div { class: "metric-label", "大小" }
                                div { class: "font-medium", "6.8GB" }
                            }
                            div {
                                div { class: "metric-label", "端口" }
                                div { class: "font-medium", "8002" }
                            }
                            div {
                                div { class: "metric-label", "内存使用" }
                                div { class: "font-medium", "--" }
                            }
                            div {
                                div { class: "metric-label", "评分" }
                                div { class: "font-medium", "⭐4.9" }
                            }
                            div {
                                div { class: "metric-label", "类型" }
                                div { class: "font-medium", "💻代码生成" }
                            }
                        }
                    }
                }
            }

            // 可安装模型
            div {
                h2 { class: "text-title font-semibold mb-lg",
                    "可安装模型 (官方仓库)"
                }
                div { class: "grid gap-lg",
                    style: "grid-template-columns: 1fr;",
                    // Qwen2.5-14B 模型卡片
                    div { class: "card model-card",
                        div { class: "model-header",
                            div { class: "model-title",
                                span { style: "font-size: 20px;", "🧠" }
                                div {
                                    div { class: "text-subtitle font-semibold", "Qwen2.5-14B-Chat" }
                                    div { class: "text-caption text-secondary", "阿里巴巴通义千问 14B 对话模型" }
                                }
                            }
                            div { class: "model-actions",
                                button { class: "btn btn-primary", "下载" }
                                button { class: "btn btn-subtle", "详情" }
                            }
                        }
                        div { class: "model-details",
                            div {
                                div { class: "metric-label", "大小" }
                                div { class: "font-medium", "📊7.2GB" }
                            }
                            div {
                                div { class: "metric-label", "评分" }
                                div { class: "font-medium", "⭐4.8" }
                            }
                            div {
                                div { class: "metric-label", "类型" }
                                div { class: "font-medium", "💬对话专用" }
                            }
                            div {
                                div { class: "metric-label", "更新时间" }
                                div { class: "font-medium", "2024-12-01" }
                            }
                        }
                    }

                    // DeepSeek-Coder-V2 模型卡片
                    div { class: "card model-card",
                        div { class: "model-header",
                            div { class: "model-title",
                                span { style: "font-size: 20px;", "💻" }
                                div {
                                    div { class: "text-subtitle font-semibold", "DeepSeek-Coder-V2" }
                                    div { class: "text-caption text-secondary", "DeepSeek 专业代码生成模型" }
                                }
                            }
                            div { class: "model-actions",
                                button { class: "btn btn-primary", "下载" }
                                button { class: "btn btn-subtle", "详情" }
                            }
                        }
                        div { class: "model-details",
                            div {
                                div { class: "metric-label", "大小" }
                                div { class: "font-medium", "📊5.1GB" }
                            }
                            div {
                                div { class: "metric-label", "评分" }
                                div { class: "font-medium", "⭐4.9" }
                            }
                            div {
                                div { class: "metric-label", "类型" }
                                div { class: "font-medium", "💻代码生成" }
                            }
                            div {
                                div { class: "metric-label", "更新时间" }
                                div { class: "font-medium", "2024-11-28" }
                            }
                        }
                    }

                    // Qwen2.5-32B 模型卡片
                    div { class: "card model-card",
                        div { class: "model-header",
                            div { class: "model-title",
                                span { style: "font-size: 20px;", "🧠" }
                                div {
                                    div { class: "text-subtitle font-semibold", "Qwen2.5-32B-Chat" }
                                    div { class: "text-caption text-secondary", "阿里巴巴通义千问 32B 大型对话模型" }
                                }
                                span { class: "text-caption",
                                    style: "background: linear-gradient(45deg, #ff6b6b, #feca57); color: white; padding: 2px 6px; border-radius: 4px; margin-left: 8px;",
                                    "🔥最新版本"
                                }
                            }
                            div { class: "model-actions",
                                button { class: "btn btn-primary", "下载" }
                                button { class: "btn btn-subtle", "详情" }
                            }
                        }
                        div { class: "model-details",
                            div {
                                div { class: "metric-label", "大小" }
                                div { class: "font-medium", "📊18.5GB" }
                            }
                            div {
                                div { class: "metric-label", "评分" }
                                div { class: "font-medium", "⭐4.9" }
                            }
                            div {
                                div { class: "metric-label", "类型" }
                                div { class: "font-medium", "💬高级对话" }
                            }
                            div {
                                div { class: "metric-label", "更新时间" }
                                div { class: "font-medium", "2024-12-05" }
                            }
                        }
                    }
                }
            }
        }
    }
}