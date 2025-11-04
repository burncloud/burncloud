use dioxus::prelude::*;

#[component]
pub fn DeployConfig() -> Element {
    let selected_model = use_signal(|| "Qwen2.5-7B-Chat".to_string());
    let port = use_signal(|| "8001".to_string());
    let bind_address = use_signal(|| "127.0.0.1".to_string());

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
                                style: "flex: 1;",
                                value: "{selected_model}",
                                option { value: "Qwen2.5-7B-Chat", "Qwen2.5-7B-Chat" }
                                option { value: "DeepSeek-V2-Chat", "DeepSeek-V2-Chat" }
                            }
                            button { class: "btn btn-secondary", "配置参数" }
                        }
                    }
                }

                // 网络配置
                div { class: "card",
                    div { class: "p-lg",
                        h3 { class: "text-subtitle font-semibold mb-md", "网络配置" }
                        div { class: "grid gap-md",
                            style: "grid-template-columns: 1fr 1fr;",
                            div {
                                label { class: "text-caption font-medium mb-xs", "绑定地址" }
                                input {
                                    class: "input",
                                    value: "{bind_address}",
                                    placeholder: "127.0.0.1"
                                }
                            }
                            div {
                                label { class: "text-caption font-medium mb-xs", "端口" }
                                input {
                                    class: "input",
                                    value: "{port}",
                                    placeholder: "8001"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}