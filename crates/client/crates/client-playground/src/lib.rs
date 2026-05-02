use burncloud_client_shared::api_client::{ChatUsage, RouteTrace};
use burncloud_client_shared::components::PageHeader;
use burncloud_client_shared::services::channel_service::ChannelService;
use burncloud_client_shared::services::playground_service::{
    ExportFormat, PlaygroundConfig, PlaygroundMessage, PlaygroundService,
};
use burncloud_client_shared::services::token_service::TokenService;
use dioxus::prelude::*;
use uuid::Uuid;

// --- ChatMessage with stable Dioxus key and metadata ---

#[derive(Clone, PartialEq)]
struct MessageMetadata {
    trace: RouteTrace,
    usage: ChatUsage,
}

#[derive(Clone, PartialEq)]
struct ChatMessage {
    id: String,
    role: String,
    content: String,
    metadata: Option<MessageMetadata>,
}

// --- Helper functions ---

fn role_label(role: &str) -> &str {
    match role {
        "user" => "U",
        "system" => "S",
        _ => "AI",
    }
}

fn role_bg(role: &str) -> String {
    match role {
        "user" => "var(--bc-primary)",
        "system" => "rgba(0,0,0,0.06)",
        _ => "#0A0A0A",
    }
    .to_string()
}

fn role_color(role: &str) -> String {
    match role {
        "system" => "var(--bc-text-secondary)",
        _ => "#fff",
    }
    .to_string()
}

fn format_cost(usd: f64) -> String {
    if usd < 0.01 {
        format!("${:.4}", usd)
    } else {
        format!("${:.2}", usd)
    }
}

fn format_cost_cny(usd: f64) -> String {
    let cny = usd * 7.2;
    if cny < 0.01 {
        format!("≈ ¥{:.2}", cny)
    } else {
        format!("≈ ¥{:.1}", cny)
    }
}

// --- Playground component ---

#[component]
pub fn Playground() -> Element {
    let mut messages: Signal<Vec<ChatMessage>> = use_signal(Vec::new);
    let mut input_text = use_signal(String::new);
    let mut selected_channel = use_signal(|| 0i64);
    let mut selected_token = use_signal(String::new);
    let mut temperature = use_signal(|| 0.7f64);
    let mut max_tokens = use_signal(|| 4096i64);
    let mut sending = use_signal(|| false);
    let mut stream_mode = use_signal(|| true);
    let mut show_reasoning = use_signal(|| false);
    let mut json_mode = use_signal(|| false);
    let mut total_prompt_tokens = use_signal(|| 0i64);
    let mut total_completion_tokens = use_signal(|| 0i64);
    let mut total_cost_usd = use_signal(|| 0.0f64);
    let mut route_traces: Signal<Vec<RouteTrace>> = use_signal(Vec::new);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);
    let current_model = use_signal(|| "gpt-4o-mini".to_string());

    let channels = use_resource(move || async move {
        ChannelService::list(0, 50).await.unwrap_or_default()
    });

    let tokens = use_resource(move || async move {
        TokenService::list().await.unwrap_or_default()
    });

    let channel_list = channels.read().clone().unwrap_or_default();
    let active_channels: Vec<_> = channel_list.iter().filter(|c| c.status == 1).collect();
    let token_list = tokens.read().clone().unwrap_or_default();
    let active_tokens: Vec<_> = token_list.iter().filter(|t| t.status == "active").collect();

    // Auto-select first channel and first token if none selected
    if selected_channel() == 0 && !active_channels.is_empty() {
        selected_channel.set(active_channels[0].id);
    }
    if selected_token().is_empty() && !active_tokens.is_empty() {
        selected_token.set(active_tokens[0].token.clone());
    }

    let mut send_trigger = use_signal(|| 0u32);

    // Send message effect
    use_effect(move || {
        let _ = send_trigger();
        let text = input_text.read().clone();
        if text.is_empty() || sending() { return; }

        // Clear previous error
        error_msg.set(None);

        sending.set(true);
        let user_msg = ChatMessage {
            id: Uuid::new_v4().to_string(),
            role: "user".to_string(),
            content: text.clone(),
            metadata: None,
        };
        messages.write().push(user_msg);
        input_text.set(String::new());

        let is_stream = stream_mode();
        let bearer = selected_token();
        let model = current_model();
        let temp = temperature();
        let max_tok = max_tokens();

        // Build PlaygroundMessage list from current messages
        let playground_msgs: Vec<PlaygroundMessage> = messages
            .read()
            .iter()
            .map(|m| PlaygroundMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();

        let config = PlaygroundConfig {
            model: model.clone(),
            channel_id: Some(selected_channel()),
            temperature: Some(temp),
            max_tokens: Some(max_tok),
            stream: is_stream,
        };

        if is_stream {
            // Streaming: add placeholder assistant message, then append tokens
            let assistant_id = Uuid::new_v4().to_string();
            let assistant_id_for_callback = assistant_id.clone();
            let assistant_id_for_result = assistant_id.clone();
            messages.write().push(ChatMessage {
                id: assistant_id,
                role: "assistant".to_string(),
                content: String::new(),
                metadata: None,
            });

            spawn(async move {
                let result = PlaygroundService::send_message_stream(
                    &playground_msgs,
                    &config,
                    &bearer,
                    move |chunk: &str| {
                        let id = assistant_id_for_callback.clone();
                        let mut msgs = messages.write();
                        if let Some(msg) = msgs.iter_mut().find(|m| m.id == id) {
                            msg.content.push_str(chunk);
                        }
                    },
                )
                .await;

                match result {
                    Ok((usage, trace)) => {
                        total_prompt_tokens += usage.prompt_tokens;
                        total_completion_tokens += usage.completion_tokens;
                        let cost = PlaygroundService::calculate_cost(&usage, &model);
                        total_cost_usd.set(total_cost_usd() + cost);
                        route_traces.write().push(trace.clone());
                        // Attach metadata to the assistant message
                        let mut msgs = messages.write();
                        if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_id_for_result) {
                            msg.metadata = Some(MessageMetadata { trace, usage });
                        }
                    }
                    Err(e) => {
                        let mut msgs = messages.write();
                        if let Some(msg) = msgs.iter_mut().find(|m| m.id == assistant_id_for_result) {
                            if !msg.content.is_empty() {
                                msg.content.push_str("\n\n[连接中断]");
                            } else {
                                msg.content = format!("错误: {}", e);
                            }
                        }
                        error_msg.set(Some(e.to_string()));
                    }
                }
                sending.set(false);
            });
        } else {
            // Non-streaming: wait for full response
            spawn(async move {
                let result = PlaygroundService::send_message(
                    &playground_msgs,
                    &config,
                    &bearer,
                )
                .await;

                match result {
                    Ok(send_result) => {
                        let assistant_msg = ChatMessage {
                            id: Uuid::new_v4().to_string(),
                            role: "assistant".to_string(),
                            content: send_result.content,
                            metadata: Some(MessageMetadata {
                                trace: send_result.trace.clone(),
                                usage: send_result.usage,
                            }),
                        };
                        total_prompt_tokens += send_result.usage.prompt_tokens;
                        total_completion_tokens += send_result.usage.completion_tokens;
                        let cost = PlaygroundService::calculate_cost(&send_result.usage, &model);
                        total_cost_usd.set(total_cost_usd() + cost);
                        route_traces.write().push(send_result.trace);
                        messages.write().push(assistant_msg);
                    }
                    Err(e) => {
                        error_msg.set(Some(e.to_string()));
                    }
                }
                sending.set(false);
            });
        }
    });

    let msg_list = messages.read();
    let total_tokens = *total_prompt_tokens.read() + *total_completion_tokens.read();
    let cost_display = format_cost(total_cost_usd());
    let cost_cny_display = format_cost_cny(total_cost_usd());

    // Clear button handler
    let on_clear = move |_| {
        messages.write().clear();
        total_prompt_tokens.set(0);
        total_completion_tokens.set(0);
        total_cost_usd.set(0.0);
        route_traces.write().clear();
        error_msg.set(None);
    };

    // Export button handler
    let on_export = move |_| {
        let playground_msgs: Vec<PlaygroundMessage> = messages
            .read()
            .iter()
            .map(|m| PlaygroundMessage {
                role: m.role.clone(),
                content: m.content.clone(),
            })
            .collect();
        let content = PlaygroundService::export_conversation(&playground_msgs, ExportFormat::Markdown);
        let _ = content;
    };

    rsx! {
        PageHeader {
            title: "演练场",
            subtitle: Some("直连网关 · 测试模型路由与系统提示".to_string()),
            actions: rsx! {
                button { class: "btn btn-secondary", onclick: on_clear, "清空" }
                button { class: "btn btn-secondary", onclick: on_export, "导出" }
            },
        }

        // Error banner
        if let Some(err) = error_msg() {
            div { class: "bc-error-banner-warning mb-sm",
                "{err}"
            }
        }

        div { class: "bc-playground-layout",
            // Config rail
            div { class: "bc-config-rail",
                // Channel selector
                div { class: "config-row",
                    label { class: "config-label", "渠道" }
                    div { class: "select-input w-full bc-h-40px",
                        select {
                            onchange: move |e| {
                                if let Ok(id) = e.value().parse::<i64>() {
                                    selected_channel.set(id);
                                }
                            },
                            for ch in &active_channels {
                                option {
                                    value: "{ch.id}",
                                    selected: ch.id == selected_channel(),
                                    "{ch.name}"
                                }
                            }
                        }
                    }
                }

                // Token selector
                div { class: "config-row",
                    label { class: "config-label", "Token" }
                    div { class: "select-input w-full bc-h-40px",
                        select {
                            aria_label: "选择 API Token",
                            onchange: move |e| {
                                selected_token.set(e.value());
                            },
                            for t in &active_tokens {
                                option {
                                    value: "{t.token}",
                                    selected: t.token == selected_token(),
                                    "{t.token} ({t.used_quota}/{t.quota_limit})"
                                }
                            }
                        }
                    }
                }

                // Model display
                div { class: "config-row",
                    label { class: "config-label", "模型" }
                    div { class: "mono bc-body-13px text-secondary",
                        "{current_model}"
                    }
                }

                div { class: "config-row",
                    label { class: "config-label", "Temperature" }
                    div { class: "flex items-center gap-md",
                        input {
                            r#type: "range",
                            min: "0",
                            max: "2",
                            step: "0.1",
                            value: "{temperature():.1}",
                            class: "bc-range-input",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<f64>() {
                                    temperature.set(v);
                                }
                            },
                        }
                        span { class: "mono bc-body-13px text-secondary bc-temp-value", "{temperature():.1}" }
                    }
                }

                div { class: "config-row",
                    label { class: "config-label", "Max tokens" }
                    div { class: "input sm w-full",
                        input {
                            r#type: "number",
                            value: "{max_tokens}",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<i64>() {
                                    max_tokens.set(v);
                                }
                            },
                        }
                    }
                }

                div { class: "config-row",
                    label { class: "config-label", "选项" }
                    label { class: "bc-option-row",
                        span { "流式响应" }
                        label { class: "switch",
                            input { r#type: "checkbox", checked: stream_mode(), onchange: move |e| stream_mode.set(e.checked()) }
                            span { class: "switch-track" }
                        }
                    }
                    label { class: "bc-option-row",
                        span { "显示推理过程" }
                        label { class: "switch",
                            input { r#type: "checkbox", checked: show_reasoning(), onchange: move |e| show_reasoning.set(e.checked()) }
                            span { class: "switch-track" }
                        }
                    }
                    label { class: "bc-option-row",
                        span { "JSON 模式" }
                        label { class: "switch",
                            input { r#type: "checkbox", checked: json_mode(), onchange: move |e| json_mode.set(e.checked()) }
                            span { class: "switch-track" }
                        }
                    }
                }
            }

            // Conversation
            div { class: "bc-conversation-area",
                div { role: "log", aria_live: "polite", class: "bc-conversation-log",
                    if msg_list.is_empty() {
                        div { class: "flex items-center justify-center h-full text-secondary",
                            "输入消息开始对话"
                        }
                    } else {
                        for msg in msg_list.iter() {
                            div { key: "{msg.id}", class: "bc-msg-row",
                                div { class: "bc-msg-avatar", style: "--bc-dynamic-avatar-bg:{role_bg(&msg.role)}; --bc-dynamic-avatar-color:{role_color(&msg.role)}",
                                    "{role_label(&msg.role)}"
                                }
                                div { class: "flex-1",
                                    div { class: "config-label mb-xs", "{msg.role}" }
                                    div { class: "bc-msg-content",
                                        "{msg.content}"
                                    }
                                }
                            }
                        }
                    }
                }

                // Input bar
                div { class: "bc-input-bar",
                    div { class: "input flex-1",
                        input {
                            r#type: "text",
                            aria_label: "输入对话消息",
                            value: "{input_text}",
                            placeholder: "输入消息… ⌘+Enter 发送",
                            oninput: move |e| input_text.set(e.value()),
                            onkeypress: move |e: KeyboardEvent| {
                                if e.key() == Key::Enter && e.modifiers().ctrl() {
                                    send_trigger += 1;
                                }
                            },
                        }
                    }
                    button {
                        class: "btn btn-primary",
                        disabled: sending(),
                        onclick: move |_| send_trigger += 1,
                        if sending() { "生成中..." } else { "发送 ↗" }
                    }
                }
            }

            // Token meter
            div { class: "bc-token-meter",
                label { class: "config-label bc-mb-0", "Usage · this session" }

                div { class: "stat-card bc-stat-compact",
                    span { class: "stat-eyebrow", "TOKENS" }
                    div { class: "stat-value bc-heading-22px", "{total_tokens}" }
                    span { class: "stat-foot", "{total_prompt_tokens} in · {total_completion_tokens} out" }
                }

                div { class: "stat-card bc-stat-compact",
                    span { class: "stat-eyebrow", "COST" }
                    div { class: "stat-value bc-heading-22px", "{cost_display}" }
                    span { class: "stat-foot", "{cost_cny_display}" }
                }

                div {
                    label { class: "config-label", "路由轨迹" }
                    div { class: "mono bc-route-trace-text",
                        if route_traces.read().is_empty() {
                            div { "暂无路由记录" }
                        } else {
                            for trace in route_traces.read().iter() {
                                div {
                                    "→ ch:{trace.channel_id.as_deref().unwrap_or(\"?\")} · {trace.model_id.as_deref().unwrap_or(\"?\")}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
