use dioxus::prelude::*;

#[component]
pub fn ApiManagement() -> Element {
    let mut active_tab = use_signal(|| "test".to_string());
    let mut api_endpoint = use_signal(|| "http://localhost:8001/v1/chat/completions".to_string());
    let mut request_body = use_signal(|| r#"{
  "model": "qwen2.5-7b-chat",
  "messages": [
    {
      "role": "user",
      "content": "你好，请介绍一下你自己"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 1000
}"#.to_string());
    let mut response_data = use_signal(|| String::new());

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0",
                        "API管理"
                    }
                    p { class: "text-secondary m-0 mt-sm",
                        "测试API接口、查看文档和统计数据"
                    }
                }
                div { class: "flex gap-md",
                    button { class: "btn btn-secondary",
                        span { "📚" }
                        "API文档"
                    }
                    button { class: "btn btn-secondary",
                        span { "🔑" }
                        "管理密钥"
                    }
                }
            }
        }

        div { class: "page-content",
            // 标签栏
            div { class: "flex gap-sm mb-xl",
                style: "border-bottom: 1px solid var(--neutral-quaternary); padding-bottom: var(--spacing-md);",
                button {
                    class: if *active_tab.read() == "test" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("test".to_string()),
                    span { "🧪" }
                    "接口测试"
                }
                button {
                    class: if *active_tab.read() == "docs" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("docs".to_string()),
                    span { "📚" }
                    "文档查看"
                }
                button {
                    class: if *active_tab.read() == "stats" { "btn btn-primary" } else { "btn btn-secondary" },
                    onclick: move |_| active_tab.set("stats".to_string()),
                    span { "📊" }
                    "调用统计"
                }
            }

            if *active_tab.read() == "test" {
                // 接口测试标签内容
                div { class: "grid",
                    style: "grid-template-columns: 1fr 1fr; gap: var(--spacing-xl); height: 600px;",

                    // 请求区域
                    div {
                        h2 { class: "text-title font-semibold mb-lg", "API请求" }
                        div { class: "card",
                            style: "height: 100%; display: flex; flex-direction: column;",
                            div { class: "p-lg flex-1",
                                div { class: "flex flex-col gap-md",
                                    div {
                                        label { class: "metric-label mb-sm", "请求地址" }
                                        div { class: "flex gap-sm",
                                            select { class: "btn btn-secondary",
                                                style: "min-width: 80px;",
                                                option { value: "POST", "POST" }
                                                option { value: "GET", "GET" }
                                            }
                                            input {
                                                class: "input",
                                                style: "flex: 1;",
                                                value: "{api_endpoint}",
                                                oninput: move |evt| api_endpoint.set(evt.value())
                                            }
                                        }
                                    }

                                    div {
                                        label { class: "metric-label mb-sm", "请求头" }
                                        div { class: "card",
                                            style: "background: var(--bg-card-hover); padding: var(--spacing-sm);",
                                            div { class: "text-caption text-secondary",
                                                "Content-Type: application/json"
                                            }
                                            div { class: "text-caption text-secondary",
                                                "Authorization: Bearer sk-••••••••••"
                                            }
                                        }
                                    }

                                    div { class: "flex-1 flex flex-col",
                                        label { class: "metric-label mb-sm", "请求体 (JSON)" }
                                        textarea {
                                            class: "input",
                                            style: "flex: 1; min-height: 200px; font-family: 'Cascadia Code', monospace; font-size: 13px;",
                                            value: "{request_body}",
                                            oninput: move |evt| request_body.set(evt.value())
                                        }
                                    }
                                }
                            }
                            div { class: "p-lg border-t",
                                style: "border-color: var(--neutral-quaternary);",
                                div { class: "flex justify-between items-center",
                                    div { class: "text-secondary text-caption",
                                        "预计消耗 tokens: ~50"
                                    }
                                    button {
                                        class: "btn btn-primary",
                                        onclick: move |_| {
                                            response_data.set("正在请求...".to_string());
                                            // TODO: 实际的API调用
                                            let mock_response = r#"{
  "id": "chatcmpl-7QyqpQq9G8B2Z3Xm",
  "object": "chat.completion",
  "created": 1677649420,
  "model": "qwen2.5-7b-chat",
  "usage": {
    "prompt_tokens": 56,
    "completion_tokens": 31,
    "total_tokens": 87
  },
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "你好！我是通义千问，一个由阿里云开发的大型语言模型。我能够回答问题、协助写作、翻译语言、编写代码等多种任务。有什么我可以帮助你的吗？"
      },
      "finish_reason": "stop"
    }
  ]
}"#;
                                            response_data.set(mock_response.to_string());
                                        },
                                        span { "🚀" }
                                        "发送请求"
                                    }
                                }
                            }
                        }
                    }

                    // 响应区域
                    div {
                        h2 { class: "text-title font-semibold mb-lg", "API响应" }
                        div { class: "card",
                            style: "height: 100%; display: flex; flex-direction: column;",
                            div { class: "p-lg flex-1",
                                if response_data.read().is_empty() {
                                    div { class: "flex items-center justify-center h-full text-secondary",
                                        "点击发送请求查看响应结果"
                                    }
                                } else {
                                    div { class: "flex flex-col h-full",
                                        div { class: "mb-md",
                                            div { class: "flex items-center gap-md mb-sm",
                                                span { class: "status-indicator status-running",
                                                    span { class: "status-dot" }
                                                    "200 OK"
                                                }
                                                span { class: "text-secondary text-caption", "响应时间: 1.2s" }
                                            }
                                            div { class: "text-caption text-secondary",
                                                "Content-Type: application/json"
                                            }
                                        }
                                        textarea {
                                            class: "input",
                                            style: "flex: 1; font-family: 'Cascadia Code', monospace; font-size: 13px; background: #1e1e1e; color: #d4d4d4;",
                                            readonly: true,
                                            value: "{response_data}"
                                        }
                                    }
                                }
                            }
                            if !response_data.read().is_empty() {
                                div { class: "p-lg border-t",
                                    style: "border-color: var(--neutral-quaternary);",
                                    div { class: "flex justify-between items-center",
                                        div { class: "text-secondary text-caption",
                                            "消耗 tokens: 87 (输入: 56, 输出: 31)"
                                        }
                                        button { class: "btn btn-secondary", "复制响应" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if *active_tab.read() == "docs" {
                // 文档查看标签内容
                div {
                    h2 { class: "text-title font-semibold mb-lg", "API文档" }
                    div { class: "card",
                        div { class: "p-lg",
                            div { class: "flex flex-col gap-xl",
                                // Chat Completions API
                                div {
                                    h3 { class: "text-subtitle font-semibold mb-md", "Chat Completions" }
                                    div { class: "card",
                                        style: "background: var(--bg-card-hover); padding: var(--spacing-lg);",
                                        div { class: "mb-md",
                                            span { class: "btn btn-secondary", style: "font-size: 12px; padding: 4px 8px;", "POST" }
                                            " "
                                            code { style: "background: var(--bg-canvas); padding: 4px 8px; border-radius: 4px;", "/v1/chat/completions" }
                                        }
                                        p { class: "text-secondary mb-md",
                                            "创建聊天对话的完成响应。兼容 OpenAI API 格式。"
                                        }
                                        div { class: "mb-md",
                                            strong { "请求参数:" }
                                            ul { class: "mt-sm",
                                                li { class: "text-secondary", code { "model" }, " (string, 必需) - 使用的模型名称" }
                                                li { class: "text-secondary", code { "messages" }, " (array, 必需) - 对话消息列表" }
                                                li { class: "text-secondary", code { "temperature" }, " (number, 可选) - 0.0-2.0，控制随机性" }
                                                li { class: "text-secondary", code { "max_tokens" }, " (integer, 可选) - 最大生成token数" }
                                                li { class: "text-secondary", code { "stream" }, " (boolean, 可选) - 是否流式返回" }
                                            }
                                        }
                                    }
                                }

                                // Models API
                                div {
                                    h3 { class: "text-subtitle font-semibold mb-md", "Models" }
                                    div { class: "card",
                                        style: "background: var(--bg-card-hover); padding: var(--spacing-lg);",
                                        div { class: "mb-md",
                                            span { class: "btn btn-secondary", style: "font-size: 12px; padding: 4px 8px;", "GET" }
                                            " "
                                            code { style: "background: var(--bg-canvas); padding: 4px 8px; border-radius: 4px;", "/v1/models" }
                                        }
                                        p { class: "text-secondary mb-md",
                                            "获取当前可用的模型列表。"
                                        }
                                    }
                                }

                                // Embeddings API
                                div {
                                    h3 { class: "text-subtitle font-semibold mb-md", "Embeddings" }
                                    div { class: "card",
                                        style: "background: var(--bg-card-hover); padding: var(--spacing-lg);",
                                        div { class: "mb-md",
                                            span { class: "btn btn-secondary", style: "font-size: 12px; padding: 4px 8px;", "POST" }
                                            " "
                                            code { style: "background: var(--bg-canvas); padding: 4px 8px; border-radius: 4px;", "/v1/embeddings" }
                                        }
                                        p { class: "text-secondary mb-md",
                                            "将文本转换为向量表示。"
                                        }
                                        div { class: "mb-md",
                                            strong { "请求参数:" }
                                            ul { class: "mt-sm",
                                                li { class: "text-secondary", code { "model" }, " (string, 必需) - 嵌入模型名称" }
                                                li { class: "text-secondary", code { "input" }, " (string|array, 必需) - 输入文本" }
                                            }
                                        }
                                    }
                                }

                                // 错误代码
                                div {
                                    h3 { class: "text-subtitle font-semibold mb-md", "错误代码" }
                                    div { class: "card",
                                        style: "background: var(--bg-card-hover); padding: var(--spacing-lg);",
                                        div { class: "grid",
                                            style: "grid-template-columns: auto 1fr; gap: var(--spacing-md);",
                                            strong { "400" }
                                            span { class: "text-secondary", "请求格式错误" }
                                            strong { "401" }
                                            span { class: "text-secondary", "API密钥无效" }
                                            strong { "429" }
                                            span { class: "text-secondary", "请求频率限制" }
                                            strong { "500" }
                                            span { class: "text-secondary", "服务器内部错误" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if *active_tab.read() == "stats" {
                // 调用统计标签内容
                div {
                    h2 { class: "text-title font-semibold mb-lg", "调用统计" }

                    // 统计概览
                    div { class: "grid mb-xl",
                        style: "grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--spacing-xl);",

                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "今日请求" }
                                span { class: "text-secondary", "24小时" }
                            }
                            div { class: "metric-value", "1,247" }
                            div { class: "text-caption text-secondary", "比昨天 +23%" }
                        }

                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "成功率" }
                                span { class: "text-secondary", "today" }
                            }
                            div { class: "metric-value", "99.2%" }
                            div { class: "text-caption text-secondary", "失败: 11次" }
                        }

                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "Token消耗" }
                                span { class: "text-secondary", "总计" }
                            }
                            div { class: "metric-value", "85.3K" }
                            div { class: "text-caption text-secondary", "输入: 52K, 输出: 33.3K" }
                        }

                        div { class: "card metric-card",
                            div { class: "metric-header",
                                h3 { class: "text-subtitle font-semibold m-0", "活跃用户" }
                                span { class: "text-secondary", "API密钥" }
                            }
                            div { class: "metric-value", "12" }
                            div { class: "text-caption text-secondary", "总计: 25个密钥" }
                        }
                    }

                    // 详细统计表格
                    div {
                        h3 { class: "text-subtitle font-semibold mb-md", "API端点统计" }
                        div { class: "card",
                            div { class: "p-lg",
                                div { class: "overflow-x-auto",
                                    table { class: "w-full",
                                        thead {
                                            tr { class: "border-b",
                                                style: "border-color: var(--neutral-quaternary);",
                                                th { class: "text-left p-sm font-medium text-secondary", "端点" }
                                                th { class: "text-left p-sm font-medium text-secondary", "请求数" }
                                                th { class: "text-left p-sm font-medium text-secondary", "成功率" }
                                                th { class: "text-left p-sm font-medium text-secondary", "平均耗时" }
                                                th { class: "text-left p-sm font-medium text-secondary", "Token消耗" }
                                            }
                                        }
                                        tbody {
                                            tr { class: "border-b",
                                                style: "border-color: var(--neutral-quaternary);",
                                                td { class: "p-sm", code { "/v1/chat/completions" } }
                                                td { class: "p-sm", "1,198" }
                                                td { class: "p-sm", "99.3%" }
                                                td { class: "p-sm", "1.2s" }
                                                td { class: "p-sm", "81.2K" }
                                            }
                                            tr { class: "border-b",
                                                style: "border-color: var(--neutral-quaternary);",
                                                td { class: "p-sm", code { "/v1/models" } }
                                                td { class: "p-sm", "42" }
                                                td { class: "p-sm", "100%" }
                                                td { class: "p-sm", "0.1s" }
                                                td { class: "p-sm", "0" }
                                            }
                                            tr {
                                                td { class: "p-sm", code { "/v1/embeddings" } }
                                                td { class: "p-sm", "7" }
                                                td { class: "p-sm", "85.7%" }
                                                td { class: "p-sm", "0.8s" }
                                                td { class: "p-sm", "4.1K" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}