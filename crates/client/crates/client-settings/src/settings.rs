use dioxus::prelude::*;
use crate::channels::ChannelManager;
use crate::tokens::TokenManager;
use crate::groups::GroupManager;

#[component]
pub fn SystemSettings() -> Element {
    let mut active_tab = use_signal(|| "general");

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
            // Tab Navigation
            div { class: "flex gap-md mb-lg border-b border-border",
                button { 
                    class: if active_tab() == "general" { "btn btn-text text-primary font-bold border-b-2 border-primary rounded-none px-md py-sm" } else { "btn btn-text text-secondary px-md py-sm" },
                    onclick: move |_| active_tab.set("general"),
                    "通用设置"
                }
                button { 
                    class: if active_tab() == "channels" { "btn btn-text text-primary font-bold border-b-2 border-primary rounded-none px-md py-sm" } else { "btn btn-text text-secondary px-md py-sm" },
                    onclick: move |_| active_tab.set("channels"),
                    "渠道管理"
                }
                button { 
                    class: if active_tab() == "groups" { "btn btn-text text-primary font-bold border-b-2 border-primary rounded-none px-md py-sm" } else { "btn btn-text text-secondary px-md py-sm" },
                    onclick: move |_| active_tab.set("groups"),
                    "分组路由"
                }
                button { 
                    class: if active_tab() == "tokens" { "btn btn-text text-primary font-bold border-b-2 border-primary rounded-none px-md py-sm" } else { "btn btn-text text-secondary px-md py-sm" },
                    onclick: move |_| active_tab.set("tokens"),
                    "令牌管理"
                }
            }

            if active_tab() == "general" {
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
            } else if active_tab() == "channels" {
                ChannelManager {}
            } else if active_tab() == "groups" {
                GroupManager {}
            } else if active_tab() == "tokens" {
                TokenManager {}
            }
        }
    }
}
