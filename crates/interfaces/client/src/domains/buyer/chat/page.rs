use super::state::ChatState;
use crate::shared::ui::{GlobalStyle, Icon, IconName, Logo};
use dioxus::prelude::*;

/// 渲染买方聊天工作区，并提供模型名称搜索与本地消息交互。
///
/// # Examples
///
/// `BuyerChat` 由 `/buyer/chat` 路由挂载，用户可以先搜索并选择模型，
/// 再在当前页面查看本地发送的消息。
///
/// # Notes
///
/// 本页面只维护客户端本地状态，不会向真实模型网关发送请求。
#[component]
pub fn BuyerChat() -> Element {
    let mut state = use_signal(ChatState::default);
    let mut sidebar_collapsed = use_signal(|| false);
    let snapshot = state.read().clone();
    let suggestions = snapshot.suggestions();
    let selected_model = snapshot.selected_model();
    let sidebar_class = if sidebar_collapsed() {
        "model-chat-sidebar collapsed"
    } else {
        "model-chat-sidebar"
    };

    rsx! {
        GlobalStyle {}
        div { class: "model-chat-app",
            aside { class: sidebar_class,
                div { class: "model-chat-brand",
                    div { class: "model-chat-brand-mark", Logo { size: 24 } }
                    strong { "BurnCloud" }
                    button {
                        class: "model-chat-icon-button model-chat-brand-action",
                        r#type: "button",
                        aria_label: "新建对话",
                        onclick: move |_| state.set(ChatState::default()),
                        Icon { name: IconName::Plus, size: 17 }
                    }
                }
                button {
                    class: "model-chat-new-button",
                    r#type: "button",
                    onclick: move |_| state.set(ChatState::default()),
                    Icon { name: IconName::Plus, size: 15 }
                    span { "新建对话" }
                }
                div { class: "model-chat-history-title", "近期对话记录" }
                nav { class: "model-chat-history", aria_label: "近期对话记录",
                    button { class: "model-chat-history-item active", r#type: "button",
                        span { class: "model-chat-history-dot" }
                        span { "选择模型并开始对话" }
                    }
                    button { class: "model-chat-history-item", r#type: "button", "BurnCloud 的硬件安全说明" }
                    button { class: "model-chat-history-item", r#type: "button", "模型路由与 SLA 保障" }
                }
                div { class: "model-chat-sidebar-footer",
                    span { "BurnCloud v2.4" }
                    span { class: "model-chat-language", Icon { name: IconName::Globe, size: 13 } "简体中文" }
                }
            }
            main { class: "model-chat-main",
                header { class: "model-chat-topbar",
                    button {
                        class: "model-chat-icon-button",
                        r#type: "button",
                        aria_label: "折叠侧边栏",
                        aria_pressed: sidebar_collapsed().to_string(),
                        onclick: move |_| sidebar_collapsed.set(!sidebar_collapsed()),
                        Icon { name: IconName::Menu, size: 17 }
                    }
                    div { class: "model-chat-topbar-model",
                        span { class: "model-chat-status-dot" }
                        if let Some(model) = selected_model {
                            span { {model.model.name} }
                            span { class: "model-chat-context", "可用模型" }
                        } else {
                            span { "尚未选择模型" }
                        }
                    }
                    a { class: "model-chat-workspace-link", href: "/buyer/overview", "进入工作台" " →" }
                }
                section { class: "model-chat-content",
                    if snapshot.messages().is_empty() {
                        div { class: "model-chat-empty",
                            div { class: "model-chat-empty-mark", Logo { size: 34 } }
                            h1 { "开始新的对话" }
                            p { "先选择一个模型，再发送你的问题。" }
                        }
                    } else {
                        div { class: "model-chat-thread",
                            for message in snapshot.messages().iter() {
                                div { class: if message.is_user() { "model-chat-message user" } else { "model-chat-message" },
                                    if message.is_user() {
                                        div { class: "model-chat-message-meta", "你 · " {message.model_name()} }
                                    }
                                    p { {message.content()} }
                                }
                            }
                        }
                    }
                    div { class: "model-chat-composer-wrap",
                        if snapshot.model_picker_open() {
                            div { class: "model-chat-results", role: "listbox", aria_label: "模型搜索结果",
                                if suggestions.is_empty() {
                                    div { class: "model-chat-empty-result", "未找到匹配的模型" }
                                } else {
                                    for candidate in suggestions.iter().copied() {
                                        button {
                                            class: "model-chat-result-item",
                                            r#type: "button",
                                            role: "option",
                                            aria_selected: if selected_model.map(|model| model.model.id) == Some(candidate.model.id) { "true" } else { "false" },
                                            onclick: move |_| state.with_mut(|value| value.select_model(candidate.model.id)),
                                            div { class: "model-chat-result-heading",
                                                strong { {candidate.model.name} }
                                                span { {candidate.model.family} }
                                            }
                                            p { {candidate.model.description} }
                                            small { "更新于 " {candidate.updated_at} }
                                        }
                                    }
                                }
                            }
                        }
                        div { class: "model-chat-composer",
                            label { class: "model-chat-model-input",
                                Icon { name: IconName::Search, size: 16 }
                                input {
                                    r#type: "search",
                                    value: snapshot.model_query().to_string(),
                                    placeholder: "请输入模型名称",
                                    aria_label: "请输入模型名称",
                                    oninput: move |event| state.with_mut(|value| value.set_model_query(event.value())),
                                }
                                if let Some(model) = selected_model {
                                    span { class: "model-chat-selected-badge", {model.model.id} }
                                }
                            }
                            textarea {
                                rows: 2,
                                value: snapshot.draft().to_string(),
                                placeholder: "输入消息或指令，按 Enter 发送",
                                aria_label: "输入消息或指令",
                                oninput: move |event| state.with_mut(|value| value.set_draft(event.value())),
                                onkeydown: move |event| {
                                    if event.key() == Key::Enter && !event.modifiers().contains(Modifiers::SHIFT) {
                                        event.prevent_default();
                                        state.with_mut(ChatState::send_message);
                                    }
                                },
                            }
                            div { class: "model-chat-composer-footer",
                                span { class: "model-chat-hint", "模型描述会随关键词实时过滤 · Shift + Enter 换行" }
                                button {
                                    class: "model-chat-send-button",
                                    r#type: "button",
                                    disabled: !snapshot.can_send(),
                                    aria_label: "发送消息",
                                    onclick: move |_| state.with_mut(ChatState::send_message),
                                    Icon { name: IconName::ArrowRight, size: 17 }
                                }
                            }
                        }
                    }
                }
                footer { class: "model-chat-footer", "BurnCloud Gateway v2.4 · Verifiable P2P Decentralized Inference" }
            }
        }
    }
}
