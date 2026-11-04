//! Chat 首页页面组合与交互事件。

use super::{
    backend::{ChatBackend, LocalBackend},
    catalog,
    model::{
        gib, resource_label, ChatMessage, ChatModel, HardwareEvent, MessageRole, Preparation,
        PreparationStage,
    },
    selector::CatalogPicker,
    state::{ChatAction, ChatState, GenerationPlan, ReasoningEffort},
};
use crate::shared::ui::{GlobalStyle, Icon, IconName, Logo};
use dioxus::core::{Runtime, Task};
use dioxus::prelude::*;
use keyboard_types::{Key, Modifiers};
use std::rc::Rc;

/// 只在当前组件树中注入，不建立全局服务注册器。
#[derive(Clone)]
pub(super) struct BackendHandle(pub(super) Rc<dyn ChatBackend>);

#[derive(Clone, Copy)]
struct GenerationTask(Signal<Option<Task>>);

#[derive(Clone, Copy)]
struct PreparationTask(Signal<Option<Task>>);

#[derive(Clone, Copy)]
struct ChatScope(ScopeId);

const LIVEVIEW_ENTER_PREVENT_DEFAULT: &str =
    "if (event.key === 'Enter' && !event.shiftKey) { event.preventDefault(); }";

/// 渲染带模型搜索和初始化卡片的 Chat 工作台。
///
/// # Examples
///
/// ```
/// use burncloud_client::domains::chat::ChatHome;
/// let _page: fn() -> dioxus::prelude::Element = ChatHome;
/// ```
///
/// # Notes
///
/// `/` 和 `/home` 使用同一组件。目录来自主服务的模型选择接口，与网关共用监听端口。
/// 检测仍为本地模拟；当前目录选择只交接仓库快照，缺少工件清单时明确停止。
/// 底层文件下载与校验能力保留，但正常目录入口不执行真实下载、部署或推理。
/// 状态只存在于当前客户端会话，目录可访问不代表已部署。
#[component]
pub fn ChatHome() -> Element {
    use_context_provider(|| {
        try_consume_context::<BackendHandle>()
            .unwrap_or_else(|| BackendHandle(Rc::new(LocalBackend)))
    });
    use_context_provider(|| ChatScope(Runtime::current().current_scope_id()));
    let mut state = use_signal(|| try_consume_context::<ChatState>().unwrap_or_default());
    let task = use_signal(|| None::<Task>);
    use_context_provider(|| GenerationTask(task));
    let mut preparation_task = use_signal(|| None::<Task>);
    use_context_provider(|| PreparationTask(preparation_task));
    use_effect(move || {
        let running = state
            .read()
            .preparation()
            .is_some_and(|p| p.stage.is_running());
        if !running && preparation_task.peek().is_some() {
            let handle = preparation_task.take();
            if let Some(handle) = handle {
                handle.cancel();
            }
        }
    });
    let snapshot = state.read().clone();
    let sidebar_class = if snapshot.sidebar_collapsed() {
        "chat-sidebar collapsed"
    } else {
        "chat-sidebar open"
    };

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
                        focus_element(".chat-model-trigger");
                    }
                    Key::Character(character)
                        if character.eq_ignore_ascii_case("n")
                            && (modifiers.contains(Modifiers::CONTROL)
                                || modifiers.contains(Modifiers::META)) =>
                    {
                        event.prevent_default();
                        if !state.read().is_busy() {
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
                    if !snapshot.sidebar_collapsed() {
                        button {
                            class: "chat-icon-button",
                            r#type: "button",
                            aria_label: "折叠侧边栏",
                            aria_pressed: "false",
                            onclick: move |_| state.with_mut(ChatState::toggle_sidebar),
                            Icon { name: IconName::PanelLeftClose, size: 16 }
                        }
                    }
                }
                nav { class: "chat-conversation-list", aria_label: "会话列表",
                    button {
                        class: "chat-conversation-new",
                        r#type: "button",
                        aria_label: "新建会话",
                        disabled: snapshot.is_busy(),
                        onclick: move |_| new_conversation_from_ui(state),
                        Icon { name: IconName::Plus, size: 14 }
                        span { "新会话" }
                    }
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
                                        disabled: snapshot.is_busy(),
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
                                        disabled: snapshot.is_busy(),
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
            }
            main { class: "chat-main",
                if snapshot.sidebar_collapsed() {
                    button {
                        class: "chat-collapsed-toggle",
                        r#type: "button",
                        aria_label: "展开侧边栏",
                        onclick: move |_| state.with_mut(ChatState::toggle_sidebar),
                        Icon { name: IconName::PanelLeftClose, size: 16 }
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
        .as_ref()
        .map(|model| model.name.as_str())
        .unwrap_or("请输入模型名称");
    let ready = selected_model
        .as_ref()
        .is_some_and(|model| snapshot.is_model_ready(model.id()));
    let reasoning_effort = snapshot.reasoning_effort();
    let menu_open = use_memo(move || state.read().model_menu_open());
    use_effect(move || {
        if menu_open() {
            focus_element(".chat-model-search-input");
        }
    });

    rsx! {
        div { class: "chat-model-selector",
            button {
                class: "chat-model-trigger", r#type: "button",
                aria_haspopup: "dialog", aria_expanded: snapshot.model_menu_open(),
                disabled: snapshot.is_busy(),
                onclick: move |_| state.with_mut(ChatState::toggle_model_menu),
                span { class: if ready { "chat-online-dot" } else { "chat-online-dot pending" } }
                strong { {model_name} }
                Icon { name: IconName::ChevronDown, size: 13 }
            }
            if snapshot.model_menu_open() {
                button {
                    class: "chat-model-backdrop", r#type: "button", aria_label: "关闭模型菜单",
                    onclick: move |_| { state.with_mut(ChatState::close_menus); focus_element(".chat-model-trigger"); },
                }
                div { class: "chat-model-menu chat-model-search", role: "dialog", aria_label: "模型搜索与推理强度",
                    CatalogPicker { state, on_confirm: move |run_id| {
                        if let Some(run_id) = run_id { spawn_preparation(state, run_id); }
                        focus_element(".chat-composer textarea");
                    } }
                    div { class: "chat-reasoning-heading", "推理强度" span { {reasoning_effort.label()} } }
                    div { class: "chat-reasoning-slider", role: "radiogroup", aria_label: "推理强度",
                        div { class: "chat-reasoning-track", aria_hidden: "true" }
                        div { class: "chat-reasoning-fill", style: format!("width: {}%;", reasoning_effort.position_percent()), aria_hidden: "true" }
                        for effort in ReasoningEffort::ALL {
                            button {
                                key: "{effort.label()}",
                                class: if effort == reasoning_effort { "chat-reasoning-stop selected" } else { "chat-reasoning-stop" },
                                r#type: "button", role: "radio", aria_checked: effort == reasoning_effort,
                                aria_label: format!("推理强度：{}", effort.label()), title: effort.label(),
                                style: format!("left: {}%;", effort.position_percent()),
                                onclick: move |_| state.with_mut(|value| value.set_reasoning_effort(effort)),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PreparationCard(preparation: Preparation, state: Signal<ChatState>) -> Element {
    let run_id = preparation.run_id;
    let stage = preparation.stage;
    let active = state
        .read()
        .preparation()
        .is_some_and(|current| current.run_id == run_id);
    let ready_elsewhere = state.read().is_model_ready(preparation.model.id())
        && matches!(
            stage,
            PreparationStage::AwaitingStart
                | PreparationStage::Cancelled
                | PreparationStage::Failed
        );
    let hardware = &preparation.hardware;
    let checks = [
        (
            "CPU",
            hardware.cpu.clone().unwrap_or_else(|| "未知".into()),
            preparation
                .hardware_events
                .iter()
                .any(|e| matches!(e, HardwareEvent::Cpu(_))),
        ),
        (
            "GPU",
            format!(
                "{} · 单卡最大可用显存 {}",
                hardware
                    .gpus
                    .as_ref()
                    .map(|gpus| gpus
                        .iter()
                        .map(|gpu| gpu.name.as_str())
                        .collect::<Vec<_>>()
                        .join("、"))
                    .unwrap_or_else(|| "未知".into()),
                resource_label(hardware.max_vram())
            ),
            preparation
                .hardware_events
                .iter()
                .any(|e| matches!(e, HardwareEvent::Gpus(_))),
        ),
        (
            "内存",
            format!("可用 {}", resource_label(hardware.ram_bytes)),
            preparation
                .hardware_events
                .iter()
                .any(|e| matches!(e, HardwareEvent::Memory(_))),
        ),
        (
            "磁盘",
            format!("可用 {}", resource_label(hardware.disk_bytes)),
            preparation
                .hardware_events
                .iter()
                .any(|e| matches!(e, HardwareEvent::Disk(_))),
        ),
    ];
    let variant_name = preparation
        .variant()
        .map(|variant| variant.quantization.clone());
    let expected_digest = preparation
        .variant()
        .map(|variant| variant.expected_sha256.clone())
        .unwrap_or_default();
    let progress = preparation.progress();
    let variants: Vec<_> = preparation
        .model
        .variants
        .iter()
        .filter(|variant| variant.fits(hardware))
        .cloned()
        .collect();
    let platform = hardware
        .platform
        .map(|platform| platform.label())
        .unwrap_or("待检测");

    rsx! {
        article { class: "chat-preparation", aria_label: "模型初始化流程",
            header {
                strong { {preparation.model.name.clone()} }
                span { class: "chat-platform-badge", {platform} }
            }
            p { class: "chat-preparation-status", role: "status", aria_live: "polite",
                if ready_elsewhere { "模型已在本次客户端会话就绪；以下为先前流程记录。" } else { {stage.label()} }
            }
            if let Some(selection) = preparation.model.catalog_selection.as_ref() {
                p { class: "chat-preparation-note", "{selection.name} · {selection.version.source} · {selection.version.model} · {selection.version.revision}" }
                p { class: "chat-preparation-note", "当前硬件检测使用模拟数据，不代表本机真实配置；当前入口未提供可下载工件，检测后会提示能力未接入，不执行真实下载或部署。" }
            }
            if !preparation.collapsed {
                if stage != PreparationStage::AwaitingStart {
                    dl { class: "chat-hardware",
                        for (name, detail, checked) in checks.iter() {
                            div {
                                dt { {name.to_string()} }
                                dd { {detail.clone()} }
                                span { class: if *checked { "chat-check-complete" } else { "chat-check-pending" },
                                    if *checked { "已完成" } else { "待检测" }
                                }
                            }
                        }
                    }
                }
                if matches!(stage, PreparationStage::Detecting | PreparationStage::Downloading) {
                    div { class: "chat-preparation-progress",
                        if let Some(percent) = progress {
                            progress { max: 100, value: percent, aria_label: format!("进度：{}", stage.label()) }
                            span { "{percent}%" }
                        } else { progress { aria_label: "下载进行中" } span { "进行中" } }
                    }
                }
                if stage == PreparationStage::SelectingVariant {
                    div { class: "chat-variant-list", aria_label: "符合硬件要求的量化版本",
                        for variant in variants {
                            button {
                                class: "chat-variant", r#type: "button", disabled: !active,
                                onclick: move |_| {
                                    if state.with_mut(|value| value.choose_variant(run_id, &variant.id)) { spawn_preparation(state, run_id); }
                                },
                                span {
                                    strong { {variant.quantization} }
                                    small { "最低内存 {gib(variant.ram_bytes)} GiB · 显存 {gib(variant.vram_bytes)} GiB · 磁盘 {gib(variant.disk_bytes)} GiB" }
                                }
                                span { "下载此版本" }
                            }
                        }
                    }
                } else if let Some(variant_name) = variant_name {
                    p { class: "chat-preparation-note", "量化版本：{variant_name}" }
                }
                if stage == PreparationStage::Verifying || preparation.actual_digest.is_some() {
                    details { class: "chat-digest",
                        summary { "校验摘要" }
                        p { "来源预期摘要" code { {expected_digest} } }
                        p { "本地摘要" code { {preparation.actual_digest.as_deref().unwrap_or("等待校验")} } }
                    }
                }
                if let Some(error) = preparation.error.as_ref() {
                    p { class: "chat-preparation-error", role: "alert", "{error}" }
                }
                if let Some(path) = preparation.local_path.as_deref() {
                    p { class: "chat-local-path", "校验成功 · 本地路径" code { {path} } }
                } else if stage == PreparationStage::Failed {
                    p { class: "chat-preparation-note", "本地路径：空；不会继续后续阶段。" }
                }
            }
            if active {
                div { class: "chat-preparation-actions",
                    if matches!(stage, PreparationStage::AwaitingStart | PreparationStage::Failed | PreparationStage::Cancelled) && preparation.model.catalog_selection.is_none() && !state.read().is_model_ready(preparation.model.id()) {
                        button { class: "chat-flow-button primary", r#type: "button",
                            onclick: move |_| {
                                if let Some(id) = state.with_mut(|value| value.start_preparation(run_id)) { spawn_preparation(state, id); }
                            },
                            if stage == PreparationStage::AwaitingStart { "下载" } else { "重试" }
                        }
                    }
                    if !stage.is_running() && !preparation.collapsed {
                        button { class: "chat-flow-button", r#type: "button",
                            onclick: move |_| { state.with_mut(|value| value.close_preparation(run_id)); },
                            if stage == PreparationStage::Ready { "完成并关闭" } else { "关闭" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn EmptyConversation() -> Element {
    rsx! {
        div { class: "chat-empty-state",
            div { class: "chat-empty-logo", Logo { size: 28 } }
            h1 { "请输入模型名称" }
            p { "先选择模型，再发送消息。模型准备就绪后即可开始对话。" }
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
    if let Some(preparation) = message.preparation {
        return rsx! { PreparationCard { preparation, state } };
    }
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
                    if let Some(error) = message.error.as_ref() { p { class: "chat-preparation-error", role: "alert", "{error}" } }
                    if message.streaming && content.is_empty() {
                        div { class: "chat-streaming-bubble",
                            span { class: "chat-streaming-dot" }
                            span { "正在生成回复…" }
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
                                        span { "思考过程" }
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
                        div { class: "chat-metrics-row",
                                if let Some(metrics) = message.metrics { InferenceMetricsView { metrics } }
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
    let model_name = selected_model
        .as_ref()
        .map(|model| model.name.as_str())
        .unwrap_or("尚未选择");
    let can_send = snapshot.can_send();
    let composer_key = format!(
        "{}-{}",
        snapshot
            .active_conversation()
            .map(|conversation| conversation.id.as_str())
            .unwrap_or("no-conversation"),
        snapshot.is_busy(),
    );

    rsx! {
        div { class: "chat-composer-area",
            div { class: "chat-composer-inner",
                if snapshot.status_loading() { p { role: "status", "正在查询模型状态…" } }
                if let Some(error) = snapshot.status_error() {
                    p { role: "alert", "{error}" }
                    button { r#type: "button", aria_label: "重试状态查询", onclick: move |_| { let id = state.read().selected_model_id().map(str::to_string); if let Some(id) = id { select_model(state, id); } }, "重试状态查询" }
                }
                div {
                    // 发送状态变化时重建编辑区域，避免浏览器保留已发送的编辑值。
                    key: "{composer_key}",
                    class: "chat-composer",
                    textarea {
                        rows: 1,
                        value: snapshot.draft().to_string(),
                        placeholder: format!("输入消息或指令（Shift + Enter 换行）· 当前模型: {model_name}"),
                        aria_label: "输入消息或指令",
                        disabled: snapshot.is_busy(),
                        oninput: move |event| state.with_mut(|value| value.set_draft(event.value())),
                        // LiveView 的事件经 WebSocket 传递，先在浏览器侧阻止 Enter 的默认换行。
                        "onkeydown": LIVEVIEW_ENTER_PREVENT_DEFAULT,
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
                        ModelSelector { state, selected_model: selected_model.clone() }
                        if snapshot.is_generating() {
                            button {
                                class: "chat-send-button chat-stop-button",
                                r#type: "button",
                                aria_label: "停止生成",
                                title: "停止生成",
                                onclick: move |_| stop_generation(state),
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
                }
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
    match state.with_mut(ChatState::send_message) {
        Some(ChatAction::Generate(plan)) => spawn_stream(state, plan),
        Some(ChatAction::Prepare(id)) => spawn_preparation(state, id),
        None => {}
    }
}

fn focus_element(selector: &'static str) {
    spawn(async move {
        if let Err(error) =
            dioxus::document::eval(&format!("document.querySelector('{selector}')?.focus();")).await
        {
            tracing::debug!(%error, "Chat 焦点切换失败");
        }
    });
}

fn select_model(mut state: Signal<ChatState>, model_id: String) {
    let backend = consume_context::<BackendHandle>().0;
    let owner = consume_context::<ChatScope>().0;
    let Some(id) = state.with_mut(|value| value.begin_selection(&model_id)) else {
        return;
    };
    Runtime::current().spawn(owner, async move {
        let result = backend.model_status(model_id).await;
        state.with_mut(|value| value.finish_selection(id, result));
    });
}

fn spawn_preparation(mut state: Signal<ChatState>, run_id: u64) {
    let backend = consume_context::<BackendHandle>().0;
    let mut task = consume_context::<PreparationTask>().0;
    let previous = task.take();
    if let Some(previous) = previous {
        previous.cancel();
    }
    let owner = consume_context::<ChatScope>().0;
    let detecting = state
        .read()
        .preparation()
        .is_some_and(|p| p.run_id == run_id && p.stage == PreparationStage::Detecting);
    let handle = Runtime::current().spawn(owner, async move {
        if detecting {
            let result = backend
                .detect_hardware(Box::new(move |event| {
                    state.with_mut(|value| value.hardware_event(run_id, event))
                }))
                .await;
            state.with_mut(|value| value.finish_hardware(run_id, result));
        } else {
            let request = state.read().initialization_request(run_id);
            if let Some(request) = request {
                let callback =
                    move |event| state.with_mut(|value| value.preparation_event(run_id, event));
                // 目录模型由客户端直接下载并校验；部署未接入，校验通过后以 NotConnected 停止。
                let result = if request.model.catalog_selection.is_some() {
                    match catalog::download_root() {
                        Ok(root) => catalog::download(&root, &request, callback)
                            .await
                            .and_then(|path| Err(catalog::deployment_missing(&path))),
                        Err(error) => Err(error),
                    }
                } else {
                    backend.initialize(request, Box::new(callback)).await
                };
                if let Some(plan) = state.with_mut(|value| value.finish_preparation(run_id, result))
                {
                    spawn_stream(state, plan);
                }
            }
        }
        task.set(None);
    });
    task.set(Some(handle));
}

fn spawn_stream(mut state: Signal<ChatState>, plan: GenerationPlan) {
    let backend = consume_context::<BackendHandle>().0;
    let owner = consume_context::<ChatScope>().0;
    let mut task = consume_context::<GenerationTask>().0;
    let callback_plan = plan.clone();
    let handle = Runtime::current().spawn(owner, async move {
        let result = backend
            .generate(
                *plan.request.clone(),
                Box::new(move |event| {
                    state.with_mut(|value| value.generation_event(&callback_plan, event));
                }),
            )
            .await;
        state.with_mut(|value| value.complete_generation(&plan, result));
        task.set(None);
    });
    task.set(Some(handle));
}

fn stop_generation(mut state: Signal<ChatState>) {
    // 先使编号失效，再销毁 future，保留已经收到的正文与思考。
    state.with_mut(ChatState::interrupt_generation);
    let mut task = consume_context::<GenerationTask>().0;
    let handle = task.take();
    if let Some(handle) = handle {
        handle.cancel();
    }
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
