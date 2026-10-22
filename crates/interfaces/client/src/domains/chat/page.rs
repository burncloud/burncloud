//! Chat 首页页面组合与交互事件。

use super::{
    model::{catalog, ChatMessage, ChatModel, MessageRole},
    state::{ChatState, GenerationPlan},
};
use crate::{
    i18n::Locale,
    shared::ui::{GlobalStyle, Icon, IconName, Logo},
};
use dioxus::prelude::*;
use keyboard_types::{Key, Modifiers};

/// 渲染 BurnCloud 根路径下的 Chat 工作台。
///
/// # Examples
///
/// `ChatHome` 由 `/` 和 `/home` 路由挂载，页面状态只存在于当前客户端会话中。
///
/// # Notes
///
/// v1 使用确定性的本地模拟回复，不调用远端推理接口；模型关键词搜索在后端契约
/// 未确认前会显示不可用状态，不把静态目录伪装成真实搜索结果。
#[component]
pub fn ChatHome() -> Element {
    let mut locale = use_context::<Signal<Locale>>();
    let mut state = use_signal(ChatState::default);
    state.with_mut(ChatState::ensure_selected_model);
    let mut model_query = use_signal(String::new);
    let mut language_menu_open = use_signal(|| false);
    let copy_notice = use_signal(|| Option::<String>::None);
    let snapshot = state.read().clone();
    let selected_model = snapshot.selected_model();
    let sidebar_class = if snapshot.sidebar_collapsed() {
        "chat-sidebar collapsed"
    } else {
        "chat-sidebar open"
    };
    let main_class = if snapshot.sidebar_collapsed() {
        "chat-main expanded"
    } else {
        "chat-main"
    };
    let current_locale = locale();

    rsx! {
        GlobalStyle {}
        div {
            class: "chat-app",
            tabindex: "-1",
            onkeydown: move |event| {
                let modifiers = event.modifiers();
                match event.key() {
                    Key::Escape => {
                        state.with_mut(ChatState::close_menus);
                        language_menu_open.set(false);
                        model_query.set(String::new());
                    }
                    Key::Character(character)
                        if character.eq_ignore_ascii_case("n")
                            && (modifiers.contains(Modifiers::CONTROL)
                                || modifiers.contains(Modifiers::META)) =>
                    {
                        event.prevent_default();
                        if !state.read().is_generating() {
                            state.with_mut(ChatState::new_conversation);
                            model_query.set(String::new());
                        }
                    }
                    _ => {}
                }
            },
            if !snapshot.sidebar_collapsed() {
                button {
                    class: "chat-mobile-scrim",
                    r#type: "button",
                    aria_label: "关闭侧边栏",
                    onclick: move |_| state.with_mut(ChatState::toggle_sidebar),
                }
            }
            aside { class: sidebar_class,
                div { class: "chat-sidebar-header",
                    a { class: "chat-brand", href: "/home", aria_label: "BurnCloud Chat 首页",
                        span { class: "chat-brand-logo", Logo { size: 28 } }
                        span { class: "chat-brand-copy",
                            strong { "BurnCloud" }
                            small { "AI Gateway" }
                        }
                    }
                    button {
                        class: "chat-icon-button chat-sidebar-close",
                        r#type: "button",
                        aria_label: "折叠侧边栏",
                        onclick: move |_| state.with_mut(ChatState::toggle_sidebar),
                        Icon { name: IconName::X, size: 17 }
                    }
                }
                button {
                    class: "chat-new-button",
                    r#type: "button",
                    disabled: snapshot.is_generating(),
                    onclick: move |_| {
                        state.with_mut(ChatState::new_conversation);
                        model_query.set(String::new());
                    },
                    Icon { name: IconName::Plus, size: 16 }
                    span { "新建会话" }
                    span { class: "chat-shortcut", "Ctrl N" }
                }
                div { class: "chat-conversation-heading",
                    span { "近期对话记录" }
                    span { class: "chat-conversation-count", {snapshot.conversations().len().to_string()} }
                }
                nav { class: "chat-conversation-list", aria_label: "近期对话记录",
                    for conversation in snapshot.conversations().iter() {
                        {
                            let conversation_id = conversation.id.clone();
                            let delete_id = conversation.id.clone();
                            let active = snapshot.is_active_conversation(&conversation.id);
                            rsx! {
                                div { class: if active { "chat-conversation active" } else { "chat-conversation" },
                                    button {
                                        class: "chat-conversation-select",
                                        r#type: "button",
                                        disabled: snapshot.is_generating(),
                                        aria_current: if active { "page" } else { "false" },
                                        onclick: move |_| {
                                            state.with_mut(|value| {
                                                value.select_conversation(&conversation_id);
                                            });
                                            model_query.set(String::new());
                                        },
                                        span { class: "chat-conversation-dot" }
                                        span { class: "chat-conversation-title", {conversation.title.clone()} }
                                    }
                                    button {
                                        class: "chat-conversation-delete",
                                        r#type: "button",
                                        disabled: snapshot.is_generating(),
                                        aria_label: "删除会话",
                                        onclick: move |_| {
                                            state.with_mut(|value| {
                                                value.delete_conversation(&delete_id);
                                            });
                                        },
                                        Icon { name: IconName::X, size: 14 }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "chat-sidebar-bottom",
                    div { class: "chat-sidebar-footer",
                        span { class: "chat-version", "BurnCloud v2.4" }
                        div { class: "chat-language-wrap",
                            button {
                                class: "chat-language-button",
                                r#type: "button",
                                aria_haspopup: "menu",
                                aria_expanded: language_menu_open(),
                                onclick: move |_| language_menu_open.set(!language_menu_open()),
                                Icon { name: IconName::Globe, size: 14 }
                                span { {current_locale.native_name()} }
                                Icon { name: IconName::ChevronDown, size: 13 }
                            }
                            if language_menu_open() {
                                div { class: "chat-language-menu", role: "menu",
                                    for language in Locale::ALL {
                                        button {
                                            class: if language == current_locale { "chat-language-option selected" } else { "chat-language-option" },
                                            r#type: "button",
                                            role: "menuitem",
                                            onclick: move |_| {
                                                locale.set(language);
                                                language_menu_open.set(false);
                                                let script = format!(
                                                    "localStorage.setItem('burncloud_selected_language', '{}'); document.documentElement.lang = '{}';",
                                                    language.storage_code(),
                                                    language.browser_code()
                                                );
                                                spawn(async move {
                                                    if let Err(error) = dioxus::document::eval(&script).await {
                                                        tracing::warn!(%error, "语言偏好写入浏览器失败");
                                                    }
                                                });
                                            },
                                            span { class: "chat-language-flag", {language.flag()} }
                                            span { {language.native_name()} }
                                            if language == current_locale { Icon { name: IconName::Check, size: 14 } }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            main { class: main_class,
                header { class: "chat-toolbar",
                    div { class: "chat-toolbar-left",
                        button {
                            class: "chat-icon-button",
                            r#type: "button",
                            aria_label: if snapshot.sidebar_collapsed() { "展开侧边栏" } else { "折叠侧边栏" },
                            aria_pressed: snapshot.sidebar_collapsed().to_string(),
                            onclick: move |_| state.with_mut(ChatState::toggle_sidebar),
                            Icon { name: IconName::Menu, size: 18 }
                        }
                        ModelSelector {
                            state,
                            query: model_query(),
                            query_signal: model_query,
                            selected_model,
                        }
                    }
                    div { class: "chat-toolbar-actions",
                        button {
                            class: "chat-toolbar-new",
                            r#type: "button",
                            disabled: snapshot.is_generating(),
                            onclick: move |_| {
                                state.with_mut(ChatState::new_conversation);
                                model_query.set(String::new());
                            },
                            Icon { name: IconName::Plus, size: 15 }
                            span { "新建" }
                        }
                        a { class: "chat-workbench-link", href: "/buyer", "进入工作台" Icon { name: IconName::ArrowRight, size: 15 } }
                    }
                }
                section { class: "chat-content",
                    div { class: "chat-message-scroll",
                        if let Some(conversation) = snapshot.active_conversation() {
                            if conversation.messages.is_empty() {
                                EmptyConversation {}
                            } else {
                                div { class: "chat-thread",
                                    for (index, message) in conversation.messages.iter().enumerate() {
                                        ChatMessageView {
                                            key: "{index}",
                                            message: message.clone(),
                                            conversation_id: conversation.id.clone(),
                                            message_index: index,
                                            state,
                                            copy_notice,
                                        }
                                    }
                                }
                            }
                        } else {
                            EmptyConversation {}
                        }
                    }
                    ChatComposer {
                        state,
                        snapshot: snapshot.clone(),
                        on_send: move |_| start_generation(state),
                    }
                    if let Some(notice) = copy_notice() {
                        div { class: if notice.starts_with("复制失败") { "chat-main-notice error" } else { "chat-main-notice" }, role: "status", {notice.clone()} }
                    }
                }
                footer { class: "chat-footer", "BurnCloud Gateway v2.4 · Verifiable P2P Decentralized Inference" }
            }
        }
    }
}

#[component]
fn ModelSelector(
    state: Signal<ChatState>,
    query: String,
    query_signal: Signal<String>,
    selected_model: Option<ChatModel>,
) -> Element {
    let snapshot = state.read().clone();
    let model_label = selected_model
        .map(|model| model.model.name)
        .unwrap_or("暂无可用模型");
    let context_label = selected_model
        .map(|model| model.model.context_window)
        .unwrap_or("请先配置在线模型");
    let model_is_online = selected_model.is_some_and(|model| model.is_online());

    rsx! {
        div { class: "chat-model-selector",
            button {
                class: "chat-model-trigger",
                r#type: "button",
                aria_haspopup: "listbox",
                aria_expanded: snapshot.model_menu_open(),
                disabled: snapshot.is_generating(),
                onclick: move |_| state.with_mut(ChatState::toggle_model_menu),
                span { class: if model_is_online { "chat-online-dot" } else { "chat-online-dot offline" } }
                span { class: "chat-model-trigger-copy",
                    strong { {model_label} }
                    small { {context_label} }
                }
                Icon { name: IconName::ChevronDown, size: 16 }
            }
            if snapshot.model_menu_open() {
                div { class: "chat-model-menu", role: "dialog", aria_label: "模型选择",
                    div { class: "chat-model-menu-heading",
                        div {
                            strong { "选择模型" }
                            small { "本地目录用于快速选择；真实关键词搜索等待后端契约" }
                        }
                        button {
                            class: "chat-icon-button chat-menu-close",
                            r#type: "button",
                            aria_label: "关闭模型菜单",
                            onclick: move |_| {
                                state.with_mut(ChatState::close_menus);
                                query_signal.set(String::new());
                            },
                            Icon { name: IconName::X, size: 15 }
                        }
                    }
                    label { class: "chat-model-search",
                        Icon { name: IconName::Search, size: 16 }
                        input {
                            r#type: "search",
                            value: query.clone(),
                            placeholder: "请输入模型名称",
                            aria_label: "请输入模型名称",
                            autofocus: true,
                            oninput: move |event| query_signal.set(event.value()),
                            onkeydown: move |event| {
                                if event.key() == Key::Escape {
                                    state.with_mut(ChatState::close_menus);
                                    query_signal.set(String::new());
                                }
                            },
                        }
                    }
                    if query.trim().is_empty() {
                        div { class: "chat-model-source-note", role: "status",
                            Icon { name: IconName::CheckCircle, size: 14 }
                            span { "本地 Playground 模型目录" }
                        }
                        div { class: "chat-model-options", role: "listbox",
                            for candidate in catalog() {
                                ModelOption {
                                    candidate,
                                    selected_id: snapshot.selected_model_id().to_string(),
                                    state,
                                    query_signal,
                                }
                            }
                        }
                    } else {
                        div { class: "chat-model-search-unavailable", role: "status",
                            Icon { name: IconName::AlertTriangle, size: 18 }
                            strong { "真实模型搜索暂不可用" }
                            p { "当前服务端没有确认关键词、更新时间和排序字段，页面不会把静态示例目录冒充为搜索结果。" }
                            small { "查询：" {query.trim()} " · 请先配置带关键词的模型搜索接口。" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ModelOption(
    candidate: ChatModel,
    selected_id: String,
    state: Signal<ChatState>,
    query_signal: Signal<String>,
) -> Element {
    let selected = selected_id == candidate.id();
    let online = candidate.is_online();
    let model_id = candidate.id().to_string();
    rsx! {
        button {
            class: if selected { "chat-model-option selected" } else if online { "chat-model-option" } else { "chat-model-option unavailable" },
            r#type: "button",
            role: "option",
            aria_selected: selected,
            disabled: !online || state.read().is_generating(),
            onclick: move |_| {
                if state.with_mut(|value| value.set_model(&model_id)) {
                    state.with_mut(ChatState::close_menus);
                    query_signal.set(String::new());
                }
            },
            div { class: "chat-model-option-main",
                span { class: "chat-model-option-dot" }
                div {
                    strong { {candidate.model.name} }
                    small { {candidate.model.family} " · " {candidate.model.context_window} }
                }
                if selected { Icon { name: IconName::Check, size: 15 } }
            }
            p { {candidate.model.description} }
            div { class: "chat-model-option-meta",
                span { {format!("${:.2}/M 输入 · ${:.2}/M 输出", candidate.model.input_price_per_million, candidate.model.output_price_per_million)} }
                span { {format!("{} tok/s · P95 {} ms", candidate.model.throughput_tokens_per_sec, candidate.model.p95_latency_ms)} }
                if !online { span { "暂不可用" } }
            }
        }
    }
}

#[component]
fn EmptyConversation() -> Element {
    rsx! {
        div { class: "chat-empty-state",
            div { class: "chat-empty-logo", Logo { size: 38 } }
            h1 { "开始新的对话" }
            p { "选择一个在线模型，输入问题，体验 BurnCloud 的本地模拟流式回复。" }
            div { class: "chat-empty-hints",
                span { "Enter 发送" }
                span { "Shift + Enter 换行" }
                span { "Ctrl/Cmd + N 新建会话" }
            }
        }
    }
}

#[component]
fn ChatMessageView(
    message: ChatMessage,
    conversation_id: String,
    message_index: usize,
    state: Signal<ChatState>,
    copy_notice: Signal<Option<String>>,
) -> Element {
    let is_user = message.role == MessageRole::User;
    let copied = message.copied;
    let content = message.content.clone();
    let message_class = if is_user {
        "chat-message user"
    } else {
        "chat-message assistant"
    };

    rsx! {
        article { class: message_class,
            div { class: "chat-message-avatar", if is_user { "你" } else { "B" } }
            div { class: "chat-message-body",
                div { class: "chat-message-meta",
                    strong { if is_user { "你" } else { "BurnCloud Assistant" } }
                    if message.streaming { span { class: "chat-streaming-label", "生成中" } }
                }
                if !is_user {
                    if let Some(thinking) = message.thinking.as_deref() {
                        details { class: "chat-thinking", open: !message.streaming,
                            summary { Icon { name: IconName::Zap, size: 14 } "思考过程" }
                            div { class: "chat-thinking-content", {thinking} }
                        }
                    }
                }
                if is_user {
                    div { class: "chat-user-content", {content.clone()} }
                } else {
                    {render_markdown(&content)}
                }
                if !is_user && !message.streaming {
                    if let Some(metrics) = message.metrics {
                        InferenceMetricsView { metrics }
                    }
                    div { class: "chat-message-actions",
                        button {
                            class: if copied { "chat-message-action copied" } else { "chat-message-action" },
                            r#type: "button",
                            aria_label: if copied { "已复制回复" } else { "复制回复" },
                            onclick: move |_| copy_message(
                                state,
                                copy_notice,
                                conversation_id.clone(),
                                message_index,
                                content.clone(),
                            ),
                            Icon { name: if copied { IconName::Check } else { IconName::Copy }, size: 14 }
                            span { if copied { "已复制" } else { "复制回复" } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn InferenceMetricsView(metrics: super::model::InferenceMetrics) -> Element {
    rsx! {
        div { class: "chat-inference-metrics", aria_label: "推理指标",
            div { span { "吞吐量" } strong { {format!("{:.0} tok/s", metrics.tokens_per_second)} } }
            div { span { "TTFT" } strong { {format!("{} ms", metrics.ttft_ms)} } }
            div { span { "节点" } strong { {metrics.node} } }
            div { span { "费用" } strong { {format!("${:.6}", metrics.cost_usd)} } }
            div { span { "Token" } strong { {format!("{} / {}", metrics.prompt_tokens, metrics.completion_tokens)} } }
        }
    }
}

#[component]
fn ChatComposer(
    state: Signal<ChatState>,
    snapshot: ChatState,
    on_send: EventHandler<()>,
) -> Element {
    let temperature_options = [0.0_f32, 0.3, 0.6, 0.9, 1.2, 1.5, 2.0];
    let selected_model = snapshot.selected_model();
    let can_send = snapshot.can_send();
    let model_id = selected_model
        .map(|model| model.id())
        .unwrap_or("无在线模型");
    rsx! {
        div { class: "chat-composer-area",
            if snapshot.temperature_menu_open() {
                div { class: "chat-temperature-menu", role: "menu", aria_label: "temperature 设置",
                    div { class: "chat-temperature-heading", "Temperature" span { "0.0 - 2.0" } }
                    for option in temperature_options {
                        button {
                            class: if (snapshot.temperature() - option).abs() < f32::EPSILON { "chat-temperature-option selected" } else { "chat-temperature-option" },
                            r#type: "button",
                            role: "menuitemradio",
                            aria_checked: (snapshot.temperature() - option).abs() < f32::EPSILON,
                            onclick: move |_| {
                                if state.with_mut(|value| value.set_temperature(option)).is_ok() {
                                    state.with_mut(ChatState::close_menus);
                                }
                            },
                            span { {format!("{:.1}", option)} }
                            if (snapshot.temperature() - option).abs() < f32::EPSILON { Icon { name: IconName::Check, size: 14 } }
                        }
                    }
                }
            }
            div { class: "chat-composer",
                textarea {
                    rows: 3,
                    value: snapshot.draft().to_string(),
                    placeholder: "输入消息或指令，按 Enter 发送",
                    aria_label: "输入消息或指令",
                    disabled: snapshot.is_generating() || selected_model.is_none(),
                    oninput: move |event| state.with_mut(|value| value.set_draft(event.value())),
                    onkeydown: move |event| {
                        if event.key() == Key::Escape {
                            state.with_mut(ChatState::close_menus);
                        } else if event.key() == Key::Enter
                            && !event.modifiers().contains(Modifiers::SHIFT)
                        {
                            event.prevent_default();
                            if state.read().can_send() {
                                on_send.call(());
                            }
                        }
                    },
                }
                div { class: "chat-composer-footer",
                    div { class: "chat-composer-tools",
                        span { class: "chat-model-badge", Icon { name: IconName::Cpu, size: 13 } {model_id} }
                        div { class: "chat-temperature-wrap",
                            button {
                                class: "chat-temperature-trigger",
                                r#type: "button",
                                disabled: selected_model.is_none(),
                                aria_haspopup: "menu",
                                aria_expanded: snapshot.temperature_menu_open(),
                                onclick: move |_| state.with_mut(ChatState::toggle_temperature_menu),
                                span { "温度" }
                                strong { {format!("{:.1}", snapshot.temperature())} }
                                Icon { name: IconName::ChevronDown, size: 13 }
                            }
                        }
                    }
                    div { class: "chat-composer-actions",
                        if snapshot.is_generating() {
                            button {
                                class: "chat-stop-button",
                                r#type: "button",
                                aria_label: "停止生成",
                                onclick: move |_| state.with_mut(ChatState::interrupt_generation),
                                Icon { name: IconName::X, size: 15 }
                                span { "停止生成" }
                            }
                        } else {
                            button {
                                class: "chat-send-button",
                                r#type: "button",
                                disabled: !can_send,
                                aria_label: "发送消息",
                                onclick: move |_| on_send.call(()),
                                Icon { name: IconName::ArrowRight, size: 17 }
                                span { "发送" }
                            }
                        }
                    }
                }
            }
            div { class: "chat-composer-hint", "Enter 发送 · Shift + Enter 换行 · Ctrl/Cmd + N 新建会话 · v1 使用本地确定性模拟回复" }
        }
    }
}

fn start_generation(mut state: Signal<ChatState>) {
    let plan = state.with_mut(ChatState::begin_generation);
    if let Some(plan) = plan {
        spawn_stream(state, plan);
    }
}

fn spawn_stream(mut state: Signal<ChatState>, plan: GenerationPlan) {
    spawn(async move {
        for segment in plan.segments {
            tokio::time::sleep(tokio::time::Duration::from_millis(120)).await;
            if !state.read().is_generation_active(&plan) {
                return;
            }
            if !state.with_mut(|value| value.append_stream_chunk(&plan, segment)) {
                return;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(160)).await;
        if state.read().is_generation_active(&plan) {
            state.with_mut(|value| {
                value.complete_generation(&plan, 760);
            });
        }
    });
}

fn copy_message(
    mut state: Signal<ChatState>,
    mut copy_notice: Signal<Option<String>>,
    conversation_id: String,
    message_index: usize,
    content: String,
) {
    let script = match serde_json::to_string(&content) {
        Ok(value) => format!("navigator.clipboard.writeText({value});"),
        Err(error) => {
            tracing::warn!(%error, "回复文本序列化失败");
            copy_notice.set(Some(format!("复制失败：{error}")));
            return;
        }
    };
    spawn(async move {
        match dioxus::document::eval(&script).await {
            Ok(_) => {
                state.with_mut(|value| {
                    value.mark_message_copied(&conversation_id, message_index, true);
                });
                copy_notice.set(Some("已复制回复".to_string()));
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                state.with_mut(|value| {
                    value.mark_message_copied(&conversation_id, message_index, false);
                });
                copy_notice.set(None);
            }
            Err(error) => {
                tracing::warn!(%error, "回复复制失败");
                copy_notice.set(Some(format!("复制失败：{error}")));
            }
        }
    });
}

enum MarkdownBlock {
    Text(String),
    Heading(String),
    ListItem(String),
    Code(String),
}

fn parse_markdown(content: &str) -> Vec<MarkdownBlock> {
    let mut blocks = Vec::new();
    let mut text = String::new();
    let mut code = String::new();
    let mut in_code = false;

    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            if in_code {
                blocks.push(MarkdownBlock::Code(code.trim_end().to_string()));
                code.clear();
            } else if !text.trim().is_empty() {
                blocks.push(MarkdownBlock::Text(text.trim().to_string()));
                text.clear();
            }
            in_code = !in_code;
        } else if in_code {
            code.push_str(line);
            code.push('\n');
        } else if let Some(heading) = line.strip_prefix("## ") {
            if !text.trim().is_empty() {
                blocks.push(MarkdownBlock::Text(text.trim().to_string()));
                text.clear();
            }
            blocks.push(MarkdownBlock::Heading(heading.to_string()));
        } else if let Some(item) = line.strip_prefix("- ") {
            if !text.trim().is_empty() {
                blocks.push(MarkdownBlock::Text(text.trim().to_string()));
                text.clear();
            }
            blocks.push(MarkdownBlock::ListItem(item.to_string()));
        } else {
            text.push_str(line);
            text.push('\n');
        }
    }
    if in_code && !code.trim().is_empty() {
        blocks.push(MarkdownBlock::Code(code.trim_end().to_string()));
    } else if !text.trim().is_empty() {
        blocks.push(MarkdownBlock::Text(text.trim().to_string()));
    }
    blocks
}

fn render_markdown(content: &str) -> Element {
    rsx! {
        div { class: "chat-markdown",
            for (index, block) in parse_markdown(content).into_iter().enumerate() {
                match block {
                    MarkdownBlock::Heading(value) => rsx! { h3 { key: "heading-{index}", {value} } },
                    MarkdownBlock::Text(value) => rsx! { p { key: "text-{index}", {value} } },
                    MarkdownBlock::ListItem(value) => rsx! { p { class: "chat-markdown-list-item", key: "list-{index}", "• " {value} } },
                    MarkdownBlock::Code(value) => rsx! { pre { key: "code-{index}", code { {value} } } },
                }
            }
        }
    }
}
