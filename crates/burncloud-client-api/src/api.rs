use dioxus::prelude::*;

#[component]
pub fn ApiManagement() -> Element {
    rsx! {
        div { class: "page-header",
            h1 { class: "text-large-title font-bold text-primary m-0",
                "API管理"
            }
            p { class: "text-secondary m-0 mt-sm",
                "管理和配置API接口"
            }
        }

        div { class: "page-content",
            div { class: "card",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "API端点" }
                    div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center p-md border-b",
                            div {
                                div { class: "font-medium", "/v1/chat/completions" }
                                div { class: "text-caption text-secondary", "对话完成接口" }
                            }
                            span { class: "status-indicator status-running",
                                span { class: "status-dot" }
                                "正常"
                            }
                        }
                        div { class: "flex justify-between items-center p-md border-b",
                            div {
                                div { class: "font-medium", "/v1/models" }
                                div { class: "text-caption text-secondary", "模型列表接口" }
                            }
                            span { class: "status-indicator status-running",
                                span { class: "status-dot" }
                                "正常"
                            }
                        }
                    }
                }
            }
        }
    }
}