use dioxus::prelude::*;

#[component]
pub fn DeployConfig() -> Element {
    let mut selected_model = use_signal(|| "Qwen2.5-7B-Chat".to_string());
    let mut port = use_signal(|| "8001".to_string());
    let mut bind_address = use_signal(|| "127.0.0.1".to_string());
    let mut api_key = use_signal(|| "••••••••••".to_string());
    let mut max_concurrent = use_signal(|| "4".to_string());
    let mut enable_gpu = use_signal(|| true);
    let mut memory_limit = use_signal(|| 8.0);
    let mut cpu_cores = use_signal(|| 4);
    let mut quantization = use_signal(|| "INT4".to_string());
    let mut show_advanced = use_signal(|| false);
    let mut context_length = use_signal(|| "4096".to_string());
    let mut temperature = use_signal(|| "0.7".to_string());
    let mut log_level = use_signal(|| "INFO".to_string());

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0",
                        "部署配置"
                    }
                    p { class: "text-secondary m-0 mt-sm",
                        "配置和部署大语言模型服务"
                    }
                }
                div { class: "flex gap-md",
                    button { class: "btn btn-secondary",
                        "保存配置"
                    }
                    button { class: "btn btn-primary",
                        span { "🚀" }
                        "快速部署"
                    }
                }
            }
        }

        div { class: "page-content",
            div { class: "grid",
                style: "grid-template-columns: 1fr; gap: var(--spacing-xl); max-width: 800px;",

                // 模型选择
                div { class: "card",
                    div { class: "p-lg",
                        h3 { class: "text-subtitle font-semibold mb-md", "模型选择" }
                        div { class: "flex items-center gap-md",
                            select {
                                class: "input",
                                style: "flex: 1; max-width: 300px;",
                                value: "{selected_model}",
                                onchange: move |evt| selected_model.set(evt.value()),
                                option { value: "Qwen2.5-7B-Chat", "Qwen2.5-7B-Chat" }
                                option { value: "DeepSeek-V2-Chat", "DeepSeek-V2-Chat" }
                                option { value: "Qwen2.5-14B-Chat", "Qwen2.5-14B-Chat" }
                            }
                            span { class: "text-secondary", "已安装模型" }
                        }
                    }
                }

                // 服务配置
                div { class: "card",
                    div { class: "p-lg",
                        h3 { class: "text-subtitle font-semibold mb-md",
                            span { "📡" }
                            " 服务配置"
                        }
                        div { class: "grid",
                            style: "grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--spacing-lg);",

                            div {
                                label { class: "metric-label mb-sm", "端口号" }
                                input {
                                    class: "input",
                                    r#type: "text",
                                    value: "{port}",
                                    placeholder: "8001",
                                    oninput: move |evt| port.set(evt.value())
                                }
                                div { class: "text-caption text-secondary mt-xs", "🔍自动检测可用端口" }
                            }

                            div {
                                label { class: "metric-label mb-sm", "绑定地址" }
                                select {
                                    class: "input",
                                    value: "{bind_address}",
                                    onchange: move |evt| bind_address.set(evt.value()),
                                    option { value: "127.0.0.1", "127.0.0.1 (本地)" }
                                    option { value: "0.0.0.0", "0.0.0.0 (局域网)" }
                                    option { value: "192.168.1.100", "192.168.1.100 (指定IP)" }
                                }
                                div { class: "text-caption text-secondary mt-xs", "网络访问范围" }
                            }

                            div {
                                label { class: "metric-label mb-sm", "API密钥" }
                                div { class: "flex gap-sm",
                                    input {
                                        class: "input",
                                        style: "flex: 1;",
                                        r#type: "password",
                                        value: "{api_key}",
                                        oninput: move |evt| api_key.set(evt.value())
                                    }
                                    button { class: "btn btn-secondary", "生成新密钥" }
                                }
                            }

                            div {
                                label { class: "metric-label mb-sm", "最大并发" }
                                input {
                                    class: "input",
                                    r#type: "number",
                                    value: "{max_concurrent}",
                                    min: "1",
                                    max: "16",
                                    oninput: move |evt| max_concurrent.set(evt.value())
                                }
                                div { class: "text-caption text-secondary mt-xs", "🔧根据硬件自动推荐" }
                            }
                        }
                    }
                }

                // 资源配置
                div { class: "card",
                    div { class: "p-lg",
                        h3 { class: "text-subtitle font-semibold mb-md",
                            span { "🧠" }
                            " 资源配置"
                        }
                        div { class: "flex flex-col gap-lg",
                            // GPU配置
                            div {
                                div { class: "flex items-center justify-between mb-md",
                                    label { class: "metric-label", "GPU加速" }
                                    input {
                                        r#type: "checkbox",
                                        checked: "{enable_gpu}",
                                        onchange: move |evt| enable_gpu.set(evt.checked())
                                    }
                                }
                                if *enable_gpu.read() {
                                    div { class: "text-caption text-secondary",
                                        "✅ 检测到: NVIDIA RTX 4090 (12GB VRAM)"
                                    }
                                }
                            }

                            // 内存限制
                            div {
                                div { class: "flex items-center justify-between mb-md",
                                    label { class: "metric-label", "内存限制" }
                                    span { class: "text-secondary", "{memory_limit:.1}GB / 16GB" }
                                }
                                input {
                                    r#type: "range",
                                    class: "w-full",
                                    min: "1",
                                    max: "16",
                                    step: "0.5",
                                    value: "{memory_limit}",
                                    oninput: move |evt| {
                                        if let Ok(val) = evt.value().parse::<f64>() {
                                            memory_limit.set(val);
                                        }
                                    }
                                }
                                div { class: "progress mt-sm",
                                    div {
                                        class: "progress-fill",
                                        style: "width: {memory_limit() / 16.0 * 100.0}%"
                                    }
                                }
                                div { class: "text-caption text-secondary mt-xs", "(推荐: 6-8GB)" }
                            }

                            // CPU核心
                            div {
                                div { class: "flex items-center justify-between mb-md",
                                    label { class: "metric-label", "CPU核心" }
                                    span { class: "text-secondary", "{cpu_cores}核 / 8核" }
                                }
                                input {
                                    r#type: "range",
                                    class: "w-full",
                                    min: "1",
                                    max: "8",
                                    value: "{cpu_cores}",
                                    oninput: move |evt| {
                                        if let Ok(val) = evt.value().parse::<i32>() {
                                            cpu_cores.set(val);
                                        }
                                    }
                                }
                                div { class: "progress mt-sm",
                                    div {
                                        class: "progress-fill",
                                        style: "width: {cpu_cores() as f64 / 8.0 * 100.0}%"
                                    }
                                }
                                div { class: "text-caption text-secondary mt-xs", "(推荐: 4核)" }
                            }

                            // 量化级别
                            div {
                                div { class: "flex items-center justify-between mb-md",
                                    label { class: "metric-label", "量化级别" }
                                    span { class: "text-secondary", "🎯性能vs质量平衡" }
                                }
                                select {
                                    class: "input",
                                    value: "{quantization}",
                                    onchange: move |evt| quantization.set(evt.value()),
                                    option { value: "FP16", "FP16 (最高质量)" }
                                    option { value: "INT8", "INT8 (平衡)" }
                                    option { value: "INT4", "INT4 (推荐)" }
                                }
                            }
                        }
                    }
                }

                // 高级选项
                div { class: "card",
                    div { class: "p-lg",
                        div { class: "flex items-center justify-between mb-md",
                            h3 { class: "text-subtitle font-semibold m-0",
                                span { "🔧" }
                                " 高级选项"
                            }
                            button {
                                class: "btn btn-subtle",
                                onclick: move |_| {
                                    let current = *show_advanced.read();
                                    show_advanced.set(!current);
                                },
                                if *show_advanced.read() { "收起" } else { "展开" }
                            }
                        }

                        if *show_advanced.read() {
                            div { class: "grid",
                                style: "grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--spacing-lg); border: 2px dashed var(--neutral-quaternary); border-radius: var(--radius-medium); padding: var(--spacing-lg); margin-top: var(--spacing-md);",

                                div {
                                    label { class: "metric-label mb-sm", "上下文长度" }
                                    input {
                                        class: "input",
                                        r#type: "text",
                                        value: "{context_length}",
                                        placeholder: "4096",
                                        oninput: move |evt| context_length.set(evt.value())
                                    }
                                    div { class: "text-caption text-secondary mt-xs", "tokens" }
                                }

                                div {
                                    label { class: "metric-label mb-sm", "温度参数" }
                                    input {
                                        class: "input",
                                        r#type: "text",
                                        value: "{temperature}",
                                        placeholder: "0.7",
                                        oninput: move |evt| temperature.set(evt.value())
                                    }
                                    div { class: "text-caption text-secondary mt-xs", "(0.1-2.0)" }
                                }

                                div {
                                    label { class: "metric-label mb-sm", "日志级别" }
                                    select {
                                        class: "input",
                                        value: "{log_level}",
                                        onchange: move |evt| log_level.set(evt.value()),
                                        option { value: "DEBUG", "DEBUG" }
                                        option { value: "INFO", "INFO" }
                                        option { value: "WARN", "WARN" }
                                        option { value: "ERROR", "ERROR" }
                                    }
                                }
                            }
                        }
                    }
                }

                // 部署按钮
                div { class: "flex justify-between items-center",
                    div { class: "text-secondary",
                        "配置完成后将启动模型服务，端口: {port()}"
                    }
                    div { class: "flex gap-md",
                        button { class: "btn btn-secondary", "保存配置" }
                        button {
                            class: "btn btn-primary",
                            style: "padding: var(--spacing-md) var(--spacing-xxl);",
                            onclick: move |_| {
                                // TODO: 实现部署逻辑
                            },
                            span { "🚀" }
                            "部署模型"
                        }
                    }
                }
            }
        }
    }
}