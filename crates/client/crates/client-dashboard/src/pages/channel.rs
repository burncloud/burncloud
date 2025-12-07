use dioxus::prelude::*;
use burncloud_client_shared::channel_service::{ChannelService, Channel};

#[component]
pub fn ChannelPage() -> Element {
    let channels = use_resource(move || async move {
        ChannelService::list().await.unwrap_or(vec![])
    });

    rsx! {
        div { class: "page-header",
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0", "渠道管理" }
                    p { class: "text-secondary m-0 mt-sm", "管理上游模型供应商与API Key" }
                }
                button { class: "btn btn-primary",
                    span { class: "icon-add mr-sm", "+" }
                    "新建渠道"
                }
            }
        }

        div { class: "page-content mt-lg",
            div { class: "card",
                table { class: "w-full border-collapse",
                    thead {
                        tr {
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "ID" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "名称" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "类型" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "模型" }
                            th { class: "text-left p-md border-b border-subtle text-secondary font-medium", "状态" }
                            th { class: "text-right p-md border-b border-subtle text-secondary font-medium", "操作" }
                        }
                    }
                    tbody {
                        match &*channels.read() {
                            Some(list) if !list.is_empty() => rsx! {
                                for channel in list {
                                    tr { class: "hover:bg-subtle transition-colors",
                                        td { class: "p-md text-secondary border-b border-subtle", "{channel.id}" }
                                        td { class: "p-md font-medium border-b border-subtle", "{channel.name}" }
                                        td { class: "p-md border-b border-subtle", 
                                            span { class: "px-sm py-xs rounded bg-surface-variant text-caption", 
                                                match channel.type_ {
                                                    1 => "OpenAI",
                                                    14 => "Anthropic",
                                                    24 => "Google Gemini",
                                                    _ => "Unknown"
                                                }
                                            }
                                        }
                                        td { class: "p-md text-secondary text-caption truncate border-b border-subtle", style: "max-width: 200px;", "{channel.models}" }
                                        td { class: "p-md border-b border-subtle",
                                            if channel.status == 1 {
                                                span { class: "inline-flex items-center gap-xs text-success", span{class:"w-2 h-2 rounded-full bg-success"}, "启用" }
                                            } else {
                                                span { class: "inline-flex items-center gap-xs text-secondary", span{class:"w-2 h-2 rounded-full bg-secondary"}, "禁用" }
                                            }
                                        }
                                        td { class: "p-md text-right border-b border-subtle",
                                            button { class: "btn-icon hover:text-primary", "✏️" }
                                            button { class: "btn-icon hover:text-error ml-sm", "🗑️" }
                                        }
                                    }
                                }
                            },
                            Some(_) => rsx! { tr { td { colspan: "6", class: "p-xl text-center text-secondary", "暂无渠道，请点击右上角创建" } } },
                            None => rsx! { tr { td { colspan: "6", class: "p-xl text-center text-secondary", "加载中..." } } }
                        }
                    }
                }
            }
        }
    }
}
