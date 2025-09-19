use dioxus::prelude::*;

#[component]
pub fn ServiceMonitor() -> Element {
    let mut active_tab = use_signal(|| "realtime".to_string());
    let mut auto_scroll = use_signal(|| true);

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0",
                        "监控与日志"
                    }
                    p { class: "text-secondary m-0 mt-sm",
                        "实时监控系统状态和查看运行日志"
                    }
                }
                div { class: "flex gap-md",
                    button { class: "btn btn-secondary",
                        span { "📈" }
                        "性能报告"
                    }
                    button { class: "btn btn-secondary",
                        span { "📁" }
                        "导出日志"
                    }
                }
            }
        }

        div { class: "page-content",
            // 标签栏
            div { class: "flex gap-sm mb-xl",
                style: "border-bottom: 1px solid var(--neutral-quaternary); padding-bottom: var(--spacing-md);",
                button {
                    class: if *active_tab.read() == "realtime" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("realtime".to_string()),
                    span { "📊" }
                    "实时监控"
                }
                button {
                    class: if *active_tab.read() == "logs" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("logs".to_string()),
                    span { "📜" }
                    "日志查看"
                }
                button {
                    class: if *active_tab.read() == "performance" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("performance".to_string()),
                    span { "📈" }
                    "性能报告"
                }
            }

            if *active_tab.read() == "realtime" {
                // 实时监控标签内容
                div {
                    // 系统状态
                    div { class: "mb-xxxl",
                        h2 { class: "text-title font-semibold mb-lg",
                            span { "📊" }
                            " 系统状态"
                        }
                        div { class: "grid",
                            style: "grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: var(--spacing-xl);",

                            // CPU 使用率
                            div { class: "card metric-card",
                                div { class: "metric-header",
                                    h3 { class: "text-subtitle font-semibold m-0", "CPU使用率" }
                                    span { class: "text-secondary", "🌡️ 52°C" }
                                }
                                div { class: "metric-value", "45.2%" }
                                div { class: "progress mt-md",
                                    div { class: "progress-fill", style: "width: 45.2%" }
                                }
                                div { class: "text-caption text-secondary mt-sm", "8核心 Intel i7-12700K" }
                            }

                            // 内存使用
                            div { class: "card metric-card",
                                div { class: "metric-header",
                                    h3 { class: "text-subtitle font-semibold m-0", "内存使用" }
                                    span { class: "text-secondary", "DDR4" }
                                }
                                div { class: "metric-value", "8.1GB / 16GB" }
                                div { class: "progress mt-md",
                                    div { class: "progress-fill", style: "width: 50.6%" }
                                }
                                div { class: "text-caption text-secondary mt-sm", "可用: 7.9GB" }
                            }

                            // GPU 显存
                            div { class: "card metric-card",
                                div { class: "metric-header",
                                    h3 { class: "text-subtitle font-semibold m-0", "GPU显存" }
                                    span { class: "text-secondary", "RTX 4090" }
                                }
                                div { class: "metric-value", "7.2GB / 12GB" }
                                div { class: "progress mt-md",
                                    div { class: "progress-fill", style: "width: 60%" }
                                }
                                div { class: "text-caption text-secondary mt-sm", "利用率: 85%" }
                            }

                            // 磁盘空间
                            div { class: "card metric-card",
                                div { class: "metric-header",
                                    h3 { class: "text-subtitle font-semibold m-0", "磁盘空间" }
                                    span { class: "text-secondary", "NVMe SSD" }
                                }
                                div { class: "metric-value", "156GB / 500GB" }
                                div { class: "progress mt-md",
                                    div { class: "progress-fill", style: "width: 31.2%" }
                                }
                                div { class: "text-caption text-secondary mt-sm", "读写速度: 3.2GB/s" }
                            }
                        }
                    }

                    // 模型状态
                    div { class: "mb-xxxl",
                        h2 { class: "text-title font-semibold mb-lg",
                            span { "🚀" }
                            " 模型状态"
                        }
                        div { class: "card",
                            div { class: "p-lg",
                                div { class: "flex flex-col gap-lg",
                                    // Qwen2.5-7B 状态
                                    div { class: "flex items-center justify-between p-md",
                                        style: "background: var(--bg-card-hover); border-radius: var(--radius-medium);",
                                        div { class: "flex items-center gap-md",
                                            span { style: "font-size: 20px;", "🧠" }
                                            div {
                                                div { class: "font-semibold", "Qwen2.5-7B" }
                                                div { class: "text-caption text-secondary", "端口: 8001" }
                                            }
                                        }
                                        div { class: "flex items-center gap-xl",
                                            span { class: "status-indicator status-running",
                                                span { class: "status-dot" }
                                                "运行中"
                                            }
                                            div { class: "text-right",
                                                div { class: "font-medium", "📈请求: 142" }
                                                div { class: "text-caption text-secondary", "⚡响应: 1.2s" }
                                            }
                                        }
                                    }

                                    // DeepSeek-V2 状态
                                    div { class: "flex items-center justify-between p-md",
                                        style: "background: var(--bg-card-hover); border-radius: var(--radius-medium);",
                                        div { class: "flex items-center gap-md",
                                            span { style: "font-size: 20px;", "🤖" }
                                            div {
                                                div { class: "font-semibold", "DeepSeek-V2" }
                                                div { class: "text-caption text-secondary", "端口: 8002" }
                                            }
                                        }
                                        div { class: "flex items-center gap-xl",
                                            span { class: "status-indicator status-stopped",
                                                span { class: "status-dot" }
                                                "待机"
                                            }
                                            div { class: "text-right",
                                                div { class: "font-medium", "📈请求: 0" }
                                                div { class: "text-caption text-secondary", "⚡响应: --" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if *active_tab.read() == "logs" {
                // 日志查看标签内容
                div {
                    div { class: "flex justify-between items-center mb-lg",
                        h2 { class: "text-title font-semibold m-0",
                            span { "📜" }
                            " 实时日志"
                        }
                        div { class: "flex gap-md",
                            button {
                                class: if *auto_scroll.read() { "btn btn-primary" } else { "btn btn-secondary" },
                                onclick: move |_| {
                                    let current = *auto_scroll.read();
                                    auto_scroll.set(!current);
                                },
                                if *auto_scroll.read() { "自动滚动" } else { "暂停滚动" }
                            }
                            button { class: "btn btn-secondary", "清空日志" }
                        }
                    }

                    div { class: "card",
                        div { class: "log-viewer",
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:45:32]" }
                                " "
                                span { class: "log-level-info", "INFO" }
                                "  Qwen2.5-7B启动成功，端口8001"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:45:45]" }
                                " "
                                span { class: "log-level-info", "INFO" }
                                "  收到API请求 /v1/chat/completions"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:45:46]" }
                                " "
                                span { class: "log-level-debug", "DEBUG" }
                                " 推理耗时: 1.2s，生成Token: 156"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:46:01]" }
                                " "
                                span { class: "log-level-warn", "WARN" }
                                "  内存使用达到80%，建议降低并发数"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:46:15]" }
                                " "
                                span { class: "log-level-info", "INFO" }
                                "  API请求完成，状态码: 200"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:46:32]" }
                                " "
                                span { class: "log-level-info", "INFO" }
                                "  收到新的API请求，队列长度: 3"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:46:45]" }
                                " "
                                span { class: "log-level-debug", "DEBUG" }
                                " GPU内存分配: 2.1GB"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:47:01]" }
                                " "
                                span { class: "log-level-info", "INFO" }
                                "  模型推理完成，响应时间: 0.8s"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:47:15]" }
                                " "
                                span { class: "log-level-info", "INFO" }
                                "  WebSocket连接建立: client_127.0.0.1:54321"
                            }
                            div { class: "log-entry",
                                span { class: "log-timestamp", "[09:47:30]" }
                                " "
                                span { class: "log-level-debug", "DEBUG" }
                                " 缓存命中率: 87.3%"
                            }
                            div {
                                style: "width: 8px; height: 16px; background: #4fc1ff; animation: blink 1s infinite;",
                                " ▊"
                            }
                        }
                    }
                }
            }

            if *active_tab.read() == "performance" {
                // 性能报告标签内容
                div {
                    h2 { class: "text-title font-semibold mb-lg",
                        span { "📈" }
                        " 性能报告"
                    }
                    div { class: "grid",
                        style: "grid-template-columns: repeat(auto-fit, minmax(250px, 1fr)); gap: var(--spacing-xl);",

                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "平均响应时间" }
                                span { class: "text-secondary", "24小时" }
                            }
                            div { class: "metric-value", "1.2s" }
                            div { class: "text-caption text-secondary", "比昨天快 15%" }
                        }

                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "吞吐量" }
                                span { class: "text-secondary", "每分钟" }
                            }
                            div { class: "metric-value", "45 请求" }
                            div { class: "text-caption text-secondary", "峰值: 78 请求/分钟" }
                        }

                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "成功率" }
                                span { class: "text-secondary", "今日" }
                            }
                            div { class: "metric-value", "99.2%" }
                            div { class: "text-caption text-secondary", "错误请求: 11/1,247" }
                        }

                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "Token生成速度" }
                                span { class: "text-secondary", "平均" }
                            }
                            div { class: "metric-value", "52 tok/s" }
                            div { class: "text-caption text-secondary", "最高: 85 tok/s" }
                        }
                    }

                    // 详细统计
                    div { class: "mt-xxxl",
                        h3 { class: "text-subtitle font-semibold mb-md", "详细统计" }
                        div { class: "card",
                            div { class: "p-lg",
                                div { class: "grid",
                                    style: "grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--spacing-lg);",

                                    div {
                                        div { class: "metric-label", "总请求数" }
                                        div { class: "metric-value text-subtitle", "1,247" }
                                    }
                                    div {
                                        div { class: "metric-label", "成功请求" }
                                        div { class: "metric-value text-subtitle", "1,236" }
                                    }
                                    div {
                                        div { class: "metric-label", "失败请求" }
                                        div { class: "metric-value text-subtitle", "11" }
                                    }
                                    div {
                                        div { class: "metric-label", "最长响应时间" }
                                        div { class: "metric-value text-subtitle", "5.7s" }
                                    }
                                    div {
                                        div { class: "metric-label", "最短响应时间" }
                                        div { class: "metric-value text-subtitle", "0.3s" }
                                    }
                                    div {
                                        div { class: "metric-label", "P95响应时间" }
                                        div { class: "metric-value text-subtitle", "2.1s" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        style {
            "@keyframes blink {{ 0%, 50% {{ opacity: 1; }} 51%, 100% {{ opacity: 0; }} }}"
        }
    }
}