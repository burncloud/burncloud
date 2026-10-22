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
/// 页面只使用确定性的本地模拟回复，不调用远端推理接口；模型、会话、语言和
/// 推理参数均只保存在当前浏览器实例的内存状态中。
#[component]
pub fn ChatHome() -> Element {
    let mut locale = use_context::<Signal<Locale>>();
    let mut state = use_signal(ChatState::default);
    state.with_mut(ChatState::ensure_selected_model);
    let mut language_menu_open = use_signal(|| false);
    let snapshot = state.read().clone();
    let selected_model = snapshot.selected_model();
    let sidebar_class = if snapshot.sidebar_collapsed() {
        "chat-sidebar collapsed"
    } else {
        "chat-sidebar open"
    };
    let current_locale = locale();
    use_effect(move || {
        // 读取信号建立响应式依赖，后续状态变化时重新执行自动滚动。
        let _state_snapshot = state.read();
        spawn(async move {
            let script = "const scroller = document.querySelector('.chat-message-scroll'); if (scroller) scroller.scrollTop = scroller.scrollHeight;";
            if let Err(error) = dioxus::document::eval(script).await {
                tracing::debug!(%error, "Chat 消息区自动滚动失败");
            }
        });
    });

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
                    }
                    Key::Character(character)
                        if character.eq_ignore_ascii_case("n")
                            && (modifiers.contains(Modifiers::CONTROL)
                                || modifiers.contains(Modifiers::META)) =>
                    {
                        event.prevent_default();
                        if !state.read().is_generating() {
                            new_conversation_from_ui(state);
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
                        span { class: "chat-brand-logo", Logo { size: 20 } }
                        strong { "BurnCloud" }
                    }
                    button {
                        class: "chat-sidebar-new",
                        r#type: "button",
                        aria_label: "新建会话",
                        disabled: snapshot.is_generating(),
                        onclick: move |_| new_conversation_from_ui(state),
                        Icon { name: IconName::Plus, size: 16 }
                    }
                }
                nav { class: "chat-conversation-list", aria_label: "近期对话记录",
                    div { class: "chat-conversation-heading", "近期对话记录" }
                    for conversation in snapshot.conversations().iter() {
                        {
                            let conversation_id = conversation.id.clone();
                            let delete_id = conversation.id.clone();
                            let active = snapshot.is_active_conversation(&conversation.id);
                            rsx! {
                                div {
                                    key: "{conversation.id}",
                                    class: if active { "chat-conversation active" } else { "chat-conversation" },
                                    button {
                                        class: "chat-conversation-select",
                                        r#type: "button",
                                        disabled: snapshot.is_generating(),
                                        aria_current: if active { "page" } else { "false" },
                                        onclick: move |_| {
                                            state.with_mut(|value| value.select_conversation(&conversation_id));
                                        },
                                        Icon { name: IconName::Bot, size: 14 }
                                        span { class: "chat-conversation-title", {conversation.title.clone()} }
                                    }
                                    button {
                                        class: "chat-conversation-delete",
                                        r#type: "button",
                                        disabled: snapshot.is_generating(),
                                        aria_label: "删除会话",
                                        onclick: move |event| {
                                            event.stop_propagation();
                                            state.with_mut(|value| value.delete_conversation(&delete_id));
                                        },
                                        Icon { name: IconName::Trash2, size: 12 }
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
                                id: "language-switcher-button",
                                class: "chat-language-button",
                                r#type: "button",
                                title: "选择语言",
                                aria_haspopup: "menu",
                                aria_expanded: language_menu_open(),
                                onclick: move |_| language_menu_open.set(!language_menu_open()),
                                Icon { name: IconName::Globe, size: 14 }
                                span { class: "chat-language-copy",
                                    span { class: "chat-language-flag", {current_locale.flag()} }
                                    span { class: "chat-language-native", {current_locale.native_name()} }
                                }
                                Icon { name: IconName::ChevronDown, size: 14 }
                            }
                            if language_menu_open() {
                                div { class: "chat-language-menu", role: "menu", aria_label: "语言选择",
                                    div { class: "chat-language-menu-title", "选择语言" }
                                    for language in Locale::ALL {
                                        button {
                                            id: match language {
                                                Locale::Zh => "lang-opt-zh",
                                                Locale::En => "lang-opt-en",
                                                Locale::ZhTw => "lang-opt-zh-TW",
                                                Locale::Ja => "lang-opt-ja",
                                            },
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
                                            span { class: "chat-language-option-flag", {language.flag()} }
                                            span { class: "chat-language-option-copy",
                                                strong { {language.native_name()} }
                                                small { {language.english_name()} }
                                            }
                                            if language == current_locale {
                                                Icon { name: IconName::Check, size: 14 }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            main { class: "chat-main",
                header { class: "chat-toolbar",
                    div { class: "chat-toolbar-left",
                        button {
                            class: "chat-icon-button",
                            r#type: "button",
                            aria_label: if snapshot.sidebar_collapsed() { "展开侧边栏" } else { "折叠侧边栏" },
                            aria_pressed: snapshot.sidebar_collapsed().to_string(),
                            onclick: move |_| state.with_mut(ChatState::toggle_sidebar),
                            Icon { name: IconName::PanelLeftClose, size: 16 }
                        }
                        ModelSelector { state, selected_model }
                    }
                    div { class: "chat-toolbar-actions",
                        button {
                            class: "chat-toolbar-new",
                            r#type: "button",
                            aria_label: "新建会话",
                            disabled: snapshot.is_generating(),
                            onclick: move |_| new_conversation_from_ui(state),
                            Icon { name: IconName::Plus, size: 16 }
                        }
                        a { class: "chat-workbench-link", href: "/buyer", "进入工作台" Icon { name: IconName::ArrowRight, size: 14 } }
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
                                            key: "{conversation.id}-{index}",
                                            message: message.clone(),
                                            conversation_id: conversation.id.clone(),
                                            message_index: index,
                                            state,
                                        }
                                    }
                                    div { id: "chat-scroll-end", class: "chat-scroll-end", aria_hidden: "true" }
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
                }
            }
        }
    }
}

#[component]
fn ModelSelector(state: Signal<ChatState>, selected_model: Option<ChatModel>) -> Element {
    let snapshot = state.read().clone();
    let model_name = selected_model
        .map(|model| model.model.name)
        .unwrap_or("暂无可用模型");
    let context_window = selected_model
        .map(|model| model.model.context_window)
        .unwrap_or("请先配置在线模型");

    rsx! {
        div { class: "chat-model-selector",
            button {
                class: "chat-model-trigger",
                r#type: "button",
                aria_haspopup: "listbox",
                aria_expanded: snapshot.model_menu_open(),
                disabled: snapshot.is_generating(),
                onclick: move |_| state.with_mut(ChatState::toggle_model_menu),
                span { class: "chat-online-dot" }
                strong { {model_name} }
                span { class: "chat-model-context", "(" {context_window} ")" }
                Icon { name: IconName::ChevronDown, size: 13 }
            }
            if snapshot.model_menu_open() {
                button {
                    class: "chat-model-backdrop",
                    r#type: "button",
                    aria_label: "关闭模型菜单",
                    onclick: move |_| state.with_mut(ChatState::close_menus),
                }
                div { class: "chat-model-menu", role: "dialog", aria_label: "模型选择",
                    div { class: "chat-model-menu-heading",
                        span { "可用开源与机密模型" }
                        span { "直接直通底价" }
                    }
                    div { class: "chat-model-options", role: "listbox",
                        for candidate in catalog() {
                            ModelOption {
                                key: "{candidate.id()}",
                                candidate,
                                selected_id: snapshot.selected_model_id().to_string(),
                                state,
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ModelOption(candidate: ChatModel, selected_id: String, state: Signal<ChatState>) -> Element {
    let selected = selected_id == candidate.id();
    let model_id = candidate.id().to_string();
    let price = format_price(candidate.model.input_price_per_million);

    rsx! {
        button {
            class: if selected { "chat-model-option selected" } else { "chat-model-option" },
            r#type: "button",
            role: "option",
            aria_selected: selected,
            disabled: state.read().is_generating(),
            onclick: move |_| {
                if state.with_mut(|value| value.set_model(&model_id)) {
                    state.with_mut(ChatState::close_menus);
                }
            },
            div { class: "chat-model-option-head",
                strong { {candidate.model.name} }
                span { "${price}/M" }
            }
            p { {candidate.model.tagline} }
            div { class: "chat-model-option-meta",
                span { {format!("{} tok/s", candidate.model.throughput_tokens_per_sec)} }
                span { "•" }
                span { {format!("p95 {}ms", candidate.model.p95_latency_ms)} }
            }
        }
    }
}

#[component]
fn EmptyConversation() -> Element {
    rsx! {
        div { class: "chat-empty-state",
            div { class: "chat-empty-logo", Logo { size: 28 } }
            h1 { "今天想运行什么模型或任务？" }
            p { "基于 AMD SEV-SNP 硬件 Enclave 的去中心化高速推理，零加价底价计费，硬件密码学证明直通。" }
        }
    }
}

#[component]
fn ChatMessageView(
    message: ChatMessage,
    conversation_id: String,
    message_index: usize,
    state: Signal<ChatState>,
) -> Element {
    let is_user = message.role == MessageRole::User;
    let copied = message.copied;
    let content = message.content.clone();
    let thinking_conversation_id = conversation_id.clone();
    let copy_conversation_id = conversation_id.clone();
    let message_class = if is_user {
        "chat-message user"
    } else {
        "chat-message assistant"
    };

    rsx! {
        article { class: message_class,
            if !is_user {
                div { class: "chat-message-avatar assistant-avatar", Logo { size: 16 } }
            }
            div { class: "chat-message-body",
                if !is_user {
                    if message.streaming && content.is_empty() {
                        div { class: "chat-streaming-bubble",
                            span { class: "chat-streaming-dot" }
                            span { "正在连接硬件 Enclave 推理流..." }
                        }
                    } else {
                        if let Some(thinking) = message.thinking.as_deref() {
                            div { class: "chat-thinking",
                                button {
                                    class: "chat-thinking-toggle",
                                    r#type: "button",
                                    aria_expanded: message.thinking_open,
                                    onclick: move |_| {
                                        state.with_mut(|value| {
                                            value.toggle_thinking(&thinking_conversation_id, message_index)
                                        });
                                    },
                                    span { class: "chat-thinking-label",
                                        Icon { name: IconName::Zap, size: 12 }
                                        span { "思考过程 (Chain-of-Thought)" }
                                    }
                                    Icon {
                                        name: IconName::ChevronDown,
                                        size: 13,
                                    }
                                }
                                if message.thinking_open {
                                    div { class: "chat-thinking-content", {thinking} }
                                }
                            }
                        }
                        div { class: "chat-bubble assistant-bubble",
                            div { class: "chat-message-content", {content.clone()} }
                        }
                    }
                    if !message.streaming {
                        if let Some(metrics) = message.metrics {
                            div { class: "chat-metrics-row",
                                InferenceMetricsView { metrics }
                                button {
                                    class: "chat-message-copy",
                                    r#type: "button",
                                    title: "Copy response",
                                    aria_label: if copied { "已复制回复" } else { "Copy response" },
                                    onclick: move |_| copy_message(
                                        state,
                                        copy_conversation_id.clone(),
                                        message_index,
                                        content.clone(),
                                    ),
                                    Icon { name: if copied { IconName::Check } else { IconName::Copy }, size: 13 }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "chat-bubble user-bubble", {content.clone()} }
                }
            }
            if is_user {
                div { class: "chat-message-avatar user-avatar", Icon { name: IconName::User, size: 16 } }
            }
        }
    }
}

#[component]
fn InferenceMetricsView(metrics: super::model::InferenceMetrics) -> Element {
    rsx! {
        div { class: "chat-inference-metrics", aria_label: "推理指标",
            span { class: "chat-throughput-badge",
                Icon { name: IconName::Zap, size: 10 }
                {format!("{:.1} tok/s", metrics.tokens_per_second)}
            }
            span { class: "chat-metric-separator", "•" }
            span { {format!("{}ms TTFT", metrics.ttft_ms)} }
            span { class: "chat-metric-separator", "•" }
            span { class: "chat-metric-enclave", {metrics.node} }
            span { class: "chat-metric-separator", "•" }
            span { class: "chat-metric-cost", {format!("${:.5}", metrics.cost_usd)} }
        }
    }
}

#[component]
fn ChatComposer(
    state: Signal<ChatState>,
    snapshot: ChatState,
    on_send: EventHandler<()>,
) -> Element {
    let selected_model = snapshot.selected_model();
    let model_id = selected_model
        .map(|model| model.id())
        .unwrap_or("无在线模型");
    let model_name = selected_model
        .map(|model| model.model.name)
        .unwrap_or("在线模型");
    let can_send = snapshot.can_send();

    rsx! {
        div { class: "chat-composer-area",
            div { class: "chat-composer-inner",
                div { class: "chat-composer",
                    textarea {
                        rows: 1,
                        value: snapshot.draft().to_string(),
                        placeholder: format!("输入消息或指令（Shift + Enter 换行）· 当前模型: {model_name}"),
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
                            span { class: "chat-model-badge", {model_id} }
                            div { class: "chat-temperature-wrap",
                                button {
                                    class: "chat-temperature-trigger",
                                    r#type: "button",
                                    title: "推理参数配置",
                                    aria_expanded: snapshot.temperature_menu_open(),
                                    onclick: move |_| state.with_mut(ChatState::toggle_temperature_menu),
                                    Icon { name: IconName::Settings2, size: 14 }
                                    span { "temp: " {format!("{:.1}", snapshot.temperature())} }
                                }
                            }
                        }
                        if snapshot.is_generating() {
                            button {
                                class: "chat-send-button chat-stop-button",
                                r#type: "button",
                                aria_label: "停止生成",
                                title: "停止生成",
                                onclick: move |_| state.with_mut(ChatState::interrupt_generation),
                                Icon { name: IconName::Stop, size: 14 }
                            }
                        } else {
                            button {
                                class: "chat-send-button",
                                r#type: "button",
                                aria_label: "发送",
                                title: "发送",
                                disabled: !can_send,
                                onclick: move |_| on_send.call(()),
                                Icon { name: IconName::Send, size: 14 }
                            }
                        }
                    }
                    if snapshot.temperature_menu_open() {
                        div { class: "chat-temperature-menu", role: "dialog", aria_label: "推理参数",
                            div { class: "chat-temperature-menu-header",
                                strong { "推理参数" }
                                button {
                                    r#type: "button",
                                    aria_label: "关闭推理参数",
                                    onclick: move |_| state.with_mut(ChatState::close_menus),
                                    Icon { name: IconName::X, size: 13 }
                                }
                            }
                            label { class: "chat-temperature-control",
                                span { class: "chat-temperature-label",
                                    span { "Temperature" }
                                    span { {format!("{:.1}", snapshot.temperature())} }
                                }
                                input {
                                    r#type: "range",
                                    min: "0",
                                    max: "1",
                                    step: "0.1",
                                    value: snapshot.temperature().to_string(),
                                    aria_label: "Temperature",
                                    oninput: move |event| {
                                        if let Ok(value) = event.value().parse::<f32>() {
                                            if let Err(error) = state.with_mut(|current| current.set_temperature(value)) {
                                                tracing::debug!(?error, "温度滑块值未通过边界校验");
                                            }
                                        }
                                    },
                                }
                            }
                            label { class: "chat-system-prompt",
                                span { "System Prompt" }
                                textarea {
                                    rows: 2,
                                    value: snapshot.system_prompt().to_string(),
                                    aria_label: "System Prompt",
                                    oninput: move |event| state.with_mut(|value| value.set_system_prompt(event.value())),
                                }
                            }
                        }
                    }
                }
                footer { class: "chat-footer", "BurnCloud Gateway v2.4 • Verifiable P2P Decentralized Inference" }
            }
        }
    }
}

fn new_conversation_from_ui(mut state: Signal<ChatState>) {
    state.with_mut(ChatState::new_conversation);
    spawn(async move {
        match dioxus::document::eval("window.innerWidth < 768")
            .join::<bool>()
            .await
        {
            Ok(true) => state.with_mut(ChatState::collapse_sidebar),
            Ok(false) => {}
            Err(error) => tracing::debug!(%error, "读取移动端视口宽度失败"),
        }
    });
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
    conversation_id: String,
    message_index: usize,
    content: String,
) {
    let script = match serde_json::to_string(&content) {
        Ok(value) => format!("navigator.clipboard.writeText({value});"),
        Err(error) => {
            tracing::warn!(%error, "回复文本序列化失败");
            return;
        }
    };
    spawn(async move {
        match dioxus::document::eval(&script).await {
            Ok(_) => {
                state.with_mut(|value| {
                    value.mark_message_copied(&conversation_id, message_index, true);
                });
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                state.with_mut(|value| {
                    value.mark_message_copied(&conversation_id, message_index, false);
                });
            }
            Err(error) => tracing::warn!(%error, "回复复制失败"),
        }
    });
}

fn format_price(price: f64) -> String {
    let formatted = format!("{price:.2}");
    formatted
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}
