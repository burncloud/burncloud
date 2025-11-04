use dioxus::prelude::*;

#[component]
pub fn SystemSettings() -> Element {
    rsx! {
        div { class: "page-header",
            h1 { class: "text-large-title font-bold text-primary m-0",
                "系统设置"
            }
            p { class: "text-secondary m-0 mt-sm",
                "配置系统参数和首选项"
            }
        }

        div { class: "page-content",
            div { class: "card",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "通用设置" }
                    div { class: "flex flex-col gap-md",
                        div { class: "flex justify-between items-center",
                            div {
                                div { class: "font-medium", "自动启动" }
                                div { class: "text-caption text-secondary", "开机时自动启动BurnCloud" }
                            }
                            input {
                                r#type: "checkbox",
                                checked: true
                            }
                        }
                        div { class: "flex justify-between items-center",
                            div {
                                div { class: "font-medium", "检查更新" }
                                div { class: "text-caption text-secondary", "自动检查软件更新" }
                            }
                            input {
                                r#type: "checkbox",
                                checked: true
                            }
                        }
                    }
                }
            }
        }
    }
}