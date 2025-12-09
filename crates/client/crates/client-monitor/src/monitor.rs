use dioxus::prelude::*;

#[component]
pub fn ServiceMonitor() -> Element {
    let mut active_tab = use_signal(|| "realtime".to_string());

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
                    span { "📈" }
                    "实时监控"
                }
                button {
                    class: if *active_tab.read() == "logs" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("logs".to_string()),
                    span { "📜" }
                    "日志查看"
                }
            }

            // 内容区域
            match active_tab.read().as_str() {
                "realtime" => rsx! {
                    div { class: "grid gap-lg",
                        style: "grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));",
                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "实时状态" }
                            }
                            div { class: "flex flex-col gap-md",
                                div { class: "flex justify-between items-center",
                                    span { class: "metric-label", "CPU使用率" }
                                    span { class: "metric-value text-subtitle", "45.2%" }
                                }
                                div { class: "flex justify-between items-center",
                                    span { class: "metric-label", "内存使用" }
                                    span { class: "metric-value text-subtitle", "8.1GB" }
                                }
                            }
                        }
                    }
                },
                "logs" => rsx! {
                    div { class: "card",
                        div { class: "p-lg",
                            div { class: "log-viewer",
                                div { class: "log-entry",
                                    span { class: "log-timestamp", "[2024-12-05 10:45:32]" }
                                    span { class: "log-level-info", "INFO" }
                                    span { ": Qwen2.5-7B 启动成功" }
                                }
                                div { class: "log-entry",
                                    span { class: "log-timestamp", "[2024-12-05 10:44:15]" }
                                    span { class: "log-level-warn", "WARN" }
                                    span { ": 内存使用达到80%" }
                                }
                            }
                        }
                    }
                },
                _ => rsx! { div { "未知页面" } }
            }
        }
    }
}
