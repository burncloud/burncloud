use dioxus::prelude::*;

#[component]
pub fn SystemSettings() -> Element {
    let mut active_tab = use_signal(|| "appearance".to_string());
    let mut theme = use_signal(|| "dark".to_string());
    let mut language = use_signal(|| "zh-CN".to_string());
    let mut font_size = use_signal(|| 3.0); // 1-5 scale
    let mut auto_start = use_signal(|| true);
    let mut minimize_to_tray = use_signal(|| true);
    let mut auto_update = use_signal(|| true);
    let mut send_analytics = use_signal(|| true);
    let mut data_directory = use_signal(|| "C:\\Users\\huang\\BurnCloud".to_string());
    let mut api_key = use_signal(|| "sk-••••••••••••••••••".to_string());
    let mut enable_rate_limit = use_signal(|| true);
    let mut localhost_only = use_signal(|| true);

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0",
                        "系统设置"
                    }
                    p { class: "text-secondary m-0 mt-sm",
                        "配置全局设置、安全选项和用户偏好"
                    }
                }
                div { class: "flex gap-md",
                    button { class: "btn btn-secondary", "恢复默认" }
                    button { class: "btn btn-primary", "保存设置" }
                }
            }
        }

        div { class: "page-content",
            div { class: "grid",
                style: "grid-template-columns: 1fr; gap: var(--spacing-xl); max-width: 800px;",

                // 标签栏
                div { class: "flex gap-sm",
                    style: "border-bottom: 1px solid var(--neutral-quaternary); padding-bottom: var(--spacing-md); margin-bottom: var(--spacing-xl);",
                    button {
                        class: if *active_tab.read() == "appearance" { "btn btn-primary" } else { "btn btn-secondary" },
                        onclick: move |_| active_tab.set("appearance".to_string()),
                        span { "🎨" }
                        "外观"
                    }
                    button {
                        class: if *active_tab.read() == "system" { "btn btn-primary" } else { "btn btn-secondary" },
                        onclick: move |_| active_tab.set("system".to_string()),
                        span { "🔧" }
                        "系统"
                    }
                    button {
                        class: if *active_tab.read() == "security" { "btn btn-primary" } else { "btn btn-secondary" },
                        onclick: move |_| active_tab.set("security".to_string()),
                        span { "🔒" }
                        "安全"
                    }
                    button {
                        class: if *active_tab.read() == "about" { "btn btn-primary" } else { "btn btn-secondary" },
                        onclick: move |_| active_tab.set("about".to_string()),
                        span { "📚" }
                        "关于"
                    }
                }

                if *active_tab.read() == "appearance" {
                    // 外观设置
                    div { class: "card",
                        div { class: "p-lg",
                            h3 { class: "text-subtitle font-semibold mb-lg",
                                span { "🎨" }
                                " 外观设置"
                            }
                            div { class: "flex flex-col gap-xl",
                                // 主题设置
                                div {
                                    label { class: "metric-label mb-md", "主题" }
                                    div { class: "flex gap-lg",
                                        label { class: "flex items-center gap-sm cursor-pointer",
                                            input {
                                                r#type: "radio",
                                                name: "theme",
                                                value: "light",
                                                checked: *theme.read() == "light",
                                                onchange: move |_| theme.set("light".to_string())
                                            }
                                            span { "☀️ 浅色" }
                                        }
                                        label { class: "flex items-center gap-sm cursor-pointer",
                                            input {
                                                r#type: "radio",
                                                name: "theme",
                                                value: "dark",
                                                checked: *theme.read() == "dark",
                                                onchange: move |_| theme.set("dark".to_string())
                                            }
                                            span { "🌙 深色" }
                                        }
                                        label { class: "flex items-center gap-sm cursor-pointer",
                                            input {
                                                r#type: "radio",
                                                name: "theme",
                                                value: "auto",
                                                checked: *theme.read() == "auto",
                                                onchange: move |_| theme.set("auto".to_string())
                                            }
                                            span { "🔄 跟随系统" }
                                        }
                                    }
                                }

                                // 语言设置
                                div {
                                    label { class: "metric-label mb-md", "语言" }
                                    select {
                                        class: "input",
                                        style: "max-width: 200px;",
                                        value: "{language}",
                                        onchange: move |evt| language.set(evt.value()),
                                        option { value: "zh-CN", "简体中文" }
                                        option { value: "en-US", "English" }
                                        option { value: "ja-JP", "日本語" }
                                    }
                                }

                                // 字体大小
                                div {
                                    label { class: "metric-label mb-md", "字体大小" }
                                    div { class: "flex items-center gap-lg",
                                        span { class: "text-secondary", "小" }
                                        input {
                                            r#type: "range",
                                            class: "flex-1",
                                            style: "max-width: 200px;",
                                            min: "1",
                                            max: "5",
                                            value: "{font_size}",
                                            oninput: move |evt| {
                                                if let Ok(val) = evt.value().parse::<f64>() {
                                                    font_size.set(val);
                                                }
                                            }
                                        }
                                        span { class: "text-secondary", "大" }
                                        span { class: "text-primary font-medium",
                                            match font_size() as i32 {
                                                1 => "很小",
                                                2 => "小",
                                                3 => "中等",
                                                4 => "大",
                                                5 => "很大",
                                                _ => "中等"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if *active_tab.read() == "system" {
                    // 系统设置
                    div { class: "card",
                        div { class: "p-lg",
                            h3 { class: "text-subtitle font-semibold mb-lg",
                                span { "🔧" }
                                " 系统设置"
                            }
                            div { class: "flex flex-col gap-xl",
                                // 启动设置
                                div {
                                    h4 { class: "text-body font-medium mb-md", "启动设置" }
                                    div { class: "flex flex-col gap-md",
                                        label { class: "flex items-center gap-sm cursor-pointer",
                                            input {
                                                r#type: "checkbox",
                                                checked: *auto_start.read(),
                                                onchange: move |evt| auto_start.set(evt.checked())
                                            }
                                            span { "开机自启动 BurnCloud" }
                                        }
                                        label { class: "flex items-center gap-sm cursor-pointer",
                                            input {
                                                r#type: "checkbox",
                                                checked: *minimize_to_tray.read(),
                                                onchange: move |evt| minimize_to_tray.set(evt.checked())
                                            }
                                            span { "最小化到系统托盘" }
                                        }
                                    }
                                }

                                // 更新设置 - 临时禁用，因为正在其它地方编写自动更新
                                // div {
                                //     h4 { class: "text-body font-medium mb-md", "更新设置" }
                                //     div { class: "flex flex-col gap-md",
                                //         label { class: "flex items-center gap-sm cursor-pointer",
                                //             input {
                                //                 r#type: "checkbox",
                                //                 checked: *auto_update.read(),
                                //                 onchange: move |evt| auto_update.set(evt.checked())
                                //             }
                                //             span { "自动检查更新" }
                                //         }
                                //         label { class: "flex items-center gap-sm cursor-pointer",
                                //             input {
                                //                 r#type: "checkbox",
                                //                 checked: *send_analytics.read(),
                                //                 onchange: move |evt| send_analytics.set(evt.checked())
                                //             }
                                //             span { "发送匿名使用统计 (帮助改进产品)" }
                                //         }
                                //     }
                                // }

                                // 匿名统计设置 (保留)
                                div {
                                    h4 { class: "text-body font-medium mb-md", "数据统计" }
                                    div { class: "flex flex-col gap-md",
                                        label { class: "flex items-center gap-sm cursor-pointer",
                                            input {
                                                r#type: "checkbox",
                                                checked: *send_analytics.read(),
                                                onchange: move |evt| send_analytics.set(evt.checked())
                                            }
                                            span { "发送匿名使用统计 (帮助改进产品)" }
                                        }
                                    }
                                }

                                // 存储设置
                                div {
                                    h4 { class: "text-body font-medium mb-md", "存储设置" }
                                    div { class: "flex flex-col gap-md",
                                        div {
                                            label { class: "metric-label mb-sm", "数据目录" }
                                            div { class: "flex gap-sm",
                                                input {
                                                    class: "input",
                                                    style: "flex: 1;",
                                                    value: "{data_directory}",
                                                    oninput: move |evt| data_directory.set(evt.value())
                                                }
                                                button { class: "btn btn-secondary", "更改" }
                                            }
                                        }
                                        div { class: "flex items-center justify-between",
                                            span { "缓存清理: 当前占用 2.1GB" }
                                            button { class: "btn btn-secondary", "立即清理" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if *active_tab.read() == "security" {
                    // 安全设置
                    div { class: "card",
                        div { class: "p-lg",
                            h3 { class: "text-subtitle font-semibold mb-lg",
                                span { "🔒" }
                                " 安全设置"
                            }
                            div { class: "flex flex-col gap-xl",
                                // API安全
                                div {
                                    h4 { class: "text-body font-medium mb-md", "API访问控制" }
                                    div { class: "flex flex-col gap-md",
                                        div {
                                            label { class: "metric-label mb-sm", "API访问密钥" }
                                            div { class: "flex gap-sm",
                                                input {
                                                    class: "input",
                                                    style: "flex: 1;",
                                                    r#type: "password",
                                                    value: "{api_key}",
                                                    oninput: move |evt| api_key.set(evt.value())
                                                }
                                                button { class: "btn btn-secondary", "重新生成" }
                                            }
                                        }
                                        label { class: "flex items-center gap-sm cursor-pointer",
                                            input {
                                                r#type: "checkbox",
                                                checked: *enable_rate_limit.read(),
                                                onchange: move |evt| enable_rate_limit.set(evt.checked())
                                            }
                                            span { "启用API速率限制 (100请求/分钟)" }
                                        }
                                        label { class: "flex items-center gap-sm cursor-pointer",
                                            input {
                                                r#type: "checkbox",
                                                checked: *localhost_only.read(),
                                                onchange: move |evt| localhost_only.set(evt.checked())
                                            }
                                            span { "仅允许本机访问 (127.0.0.1)" }
                                        }
                                    }
                                }

                                // 网络安全
                                div {
                                    h4 { class: "text-body font-medium mb-md", "网络安全" }
                                    div { class: "flex flex-col gap-md",
                                        div { class: "flex items-center justify-between",
                                            span { "防火墙白名单" }
                                            button { class: "btn btn-secondary", "管理端口规则" }
                                        }
                                        div { class: "card",
                                            style: "background: var(--bg-card-hover); padding: var(--spacing-md);",
                                            div { class: "text-caption text-secondary",
                                                "当前允许的端口: 8001, 8002, 8003"
                                            }
                                        }
                                    }
                                }

                                // 日志安全
                                div {
                                    h4 { class: "text-body font-medium mb-md", "日志与隐私" }
                                    div { class: "flex flex-col gap-md",
                                        div { class: "card",
                                            style: "background: var(--bg-card-hover); padding: var(--spacing-md);",
                                            div { class: "text-caption text-secondary",
                                                "⚠️ 日志文件可能包含敏感信息，请妥善保管"
                                            }
                                        }
                                        button { class: "btn btn-secondary", "清空所有日志" }
                                    }
                                }
                            }
                        }
                    }
                }

                if *active_tab.read() == "about" {
                    // 关于页面
                    div { class: "card",
                        div { class: "p-lg text-center",
                            div { class: "mb-xl",
                                div { class: "text-xxxl mb-md", "🔥" }
                                h2 { class: "text-large-title font-bold mb-sm", "BurnCloud" }
                                p { class: "text-secondary", "大模型本地部署平台" }
                            }

                            div { class: "grid",
                                style: "grid-template-columns: auto 1fr; gap: var(--spacing-md); text-align: left; max-width: 400px; margin: 0 auto;",

                                span { class: "text-secondary", "版本:" }
                                span { class: "font-medium", "v1.0.0" }

                                span { class: "text-secondary", "构建日期:" }
                                span { class: "font-medium", "2024-12-19" }

                                span { class: "text-secondary", "运行时:" }
                                span { class: "font-medium", "Dioxus 0.6.3" }

                                span { class: "text-secondary", "平台:" }
                                span { class: "font-medium", "Windows 11" }

                                span { class: "text-secondary", "架构:" }
                                span { class: "font-medium", "x86_64" }
                            }

                            div { class: "mt-xl flex flex-col gap-md",
                                div { class: "flex justify-center gap-md",
                                    button { class: "btn btn-secondary",
                                        span { "🔄" }
                                        "检查更新"
                                    }
                                    button { class: "btn btn-secondary",
                                        span { "📚" }
                                        "查看文档"
                                    }
                                    button { class: "btn btn-secondary",
                                        span { "🐛" }
                                        "报告问题"
                                    }
                                }

                                div { class: "text-caption text-secondary",
                                    "© 2024 BurnCloud. 基于 Rust 和 Dioxus 构建。"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}