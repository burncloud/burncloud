use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
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

                // 系统状态卡片
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
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "内存使用" }
                            span { class: "metric-value text-subtitle", "8.1GB / 16GB" }
                        }
                        div { class: "progress",
                            div { class: "progress-fill", style: "width: 50.6%" }
                        }
                    }
                }

                // 模型状态卡片
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
                        div { class: "flex justify-between items-center",
                            div { class: "flex items-center gap-sm",
                                span { "🤖" }
                                span { class: "font-medium", "DeepSeek-V2" }
                            }
                            span { class: "status-indicator status-stopped",
                                span { class: "status-dot" }
                                "已停止"
                            }
                        }
                    }
                }

                // API统计卡片
                div { class: "card metric-card",
                    div { class: "metric-header",
                        h3 { class: "text-subtitle font-semibold m-0", "API统计" }
                        span { class: "text-secondary", "今日" }
                    }
                    div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "总请求数" }
                            span { class: "metric-value", "1,247" }
                        }
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "成功率" }
                            span { class: "metric-value", "99.2%" }
                        }
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "平均响应时间" }
                            span { class: "metric-value", "1.2s" }
                        }
                    }
                }

                // 存储使用卡片
                div { class: "card metric-card",
                    div { class: "metric-header",
                        h3 { class: "text-subtitle font-semibold m-0", "存储使用" }
                    }
                    div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "模型文件" }
                            span { class: "metric-value text-subtitle", "23.4GB" }
                        }
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "缓存数据" }
                            span { class: "metric-value text-subtitle", "2.1GB" }
                        }
                        div { class: "flex justify-between items-center",
                            span { class: "metric-label", "可用空间" }
                            span { class: "metric-value text-subtitle", "344GB" }
                        }
                        div { class: "progress",
                            div { class: "progress-fill", style: "width: 31.2%" }
                        }
                    }
                }
            }

            // 快速操作区域
            div { class: "mt-xxxl",
                h2 { class: "text-title font-semibold mb-lg", "快速操作" }
                div { class: "flex gap-lg",
                    button { class: "btn btn-primary",
                        span { "🚀" }
                        "部署新模型"
                    }
                    button { class: "btn btn-secondary",
                        span { "📊" }
                        "查看性能报告"
                    }
                    button { class: "btn btn-secondary",
                        span { "🔧" }
                        "系统设置"
                    }
                    button { class: "btn btn-secondary",
                        span { "📚" }
                        "查看文档"
                    }
                }
            }

            // 最近活动
            div { class: "mt-xxxl",
                h2 { class: "text-title font-semibold mb-lg", "最近活动" }
                div { class: "card",
                    div { class: "p-lg",
                        div { class: "flex flex-col gap-md",
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-md",
                                    span { class: "text-secondary", "09:45:32" }
                                    span { class: "status-indicator status-running",
                                        span { class: "status-dot" }
                                        "INFO"
                                    }
                                    span { "Qwen2.5-7B 启动成功" }
                                }
                                span { class: "text-secondary text-caption", "刚刚" }
                            }
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-md",
                                    span { class: "text-secondary", "09:44:15" }
                                    span { class: "status-indicator status-pending",
                                        span { class: "status-dot" }
                                        "WARN"
                                    }
                                    span { "内存使用达到80%" }
                                }
                                span { class: "text-secondary text-caption", "1分钟前" }
                            }
                            div { class: "flex items-center justify-between",
                                div { class: "flex items-center gap-md",
                                    span { class: "text-secondary", "09:42:03" }
                                    span { class: "status-indicator status-running",
                                        span { class: "status-dot" }
                                        "INFO"
                                    }
                                    span { "API请求成功: /v1/chat/completions" }
                                }
                                span { class: "text-secondary text-caption", "3分钟前" }
                            }
                        }
                    }
                }
            }
        }
    }
}