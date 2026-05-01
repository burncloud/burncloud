use burncloud_client_shared::components::PageHeader;
use burncloud_client_shared::services::channel_service::ChannelService;
use dioxus::prelude::*;

#[allow(dead_code)]
fn channel_status(status: i32) -> String {
    if status == 1 { "active".to_string() } else { "down".to_string() }
}

#[derive(Clone, PartialEq)]
struct ChatMessage {
    role: String,
    content: String,
}

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
    }.to_string()
}

fn role_color(role: &str) -> String {
    match role {
        "system" => "var(--bc-text-secondary)",
        _ => "#fff",
    }.to_string()
}

#[component]
pub fn Playground() -> Element {
    let mut messages = use_signal(|| vec![
        ChatMessage { role: "system".to_string(), content: "You are a helpful Rust systems engineer.".to_string() },
        ChatMessage { role: "user".to_string(), content: "解释一下 Tokio 中 select! 宏的取消安全语义。".to_string() },
        ChatMessage { role: "assistant".to_string(), content: "Tokio 的 `select!` 在多个 future 之间竞争，赢家被 await，其余被 drop —— 这就是\"取消\"。\n\n关键是：被 drop 的 future 必须能在任意 await 点被安全丢弃。这意味着它不能在 await 之间持有需要显式释放的资源（比如未提交的事务、半发送的字节）。Tokio 的 IO 原语（`AsyncRead/AsyncWrite`、`Mutex::lock`）都是取消安全的；自定义 future 需要自己保证。".to_string() },
        ChatMessage { role: "user".to_string(), content: "给我一个反面例子。".to_string() },
    ]);
    let mut input_text = use_signal(String::new);
    let mut selected_channel = use_signal(|| 0i64);
    let mut temperature = use_signal(|| 0.7f64);
    let mut max_tokens = use_signal(|| 4096i64);
    let mut sending = use_signal(|| false);
    let mut stream_mode = use_signal(|| true);
    let mut show_reasoning = use_signal(|| false);
    let mut json_mode = use_signal(|| false);
    let mut total_prompt_tokens = use_signal(|| 0i64);
    let mut total_completion_tokens = use_signal(|| 0i64);

    let channels = use_resource(move || async move {
        ChannelService::list(0, 50).await.unwrap_or_default()
    });

    let channel_list = channels.read().clone().unwrap_or_default();
    let active_channels: Vec<_> = channel_list.iter().filter(|c| c.status == 1).collect();

    let mut send_trigger = use_signal(|| 0u32);

    use_effect(move || {
        let _ = send_trigger();
        let text = input_text.read().clone();
        if text.is_empty() || sending() { return; }

        sending.set(true);
        let user_msg = ChatMessage {
            role: "user".to_string(),
            content: text.clone(),
        };
        messages.write().push(user_msg);
        input_text.set(String::new());

        let ch_id = selected_channel();
        let temp = temperature();
        let max_tok = max_tokens();

        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(800)).await;

            let assistant_msg = ChatMessage {
                role: "assistant".to_string(),
                content: format!("收到您的消息。渠道 {} · 温度 {:.1} · 最大 Token {}", ch_id, temp, max_tok),
            };
            messages.write().push(assistant_msg);
            total_prompt_tokens += 15;
            total_completion_tokens += 42;
            sending.set(false);
        });
    });

    let msg_list = messages.read();
    let total_tokens = *total_prompt_tokens.read() + *total_completion_tokens.read();
    let cost_usd = *total_prompt_tokens.read() as f64 * 0.00003 + *total_completion_tokens.read() as f64 * 0.00006;
    let cost_cny = cost_usd * 7.2;

    rsx! {
        PageHeader {
            title: "演练场",
            subtitle: Some("直连网关 · 测试模型路由与系统提示".to_string()),
            actions: rsx! {
                button { class: "btn btn-secondary", "清空" }
                button { class: "btn btn-secondary", "导出" }
            },
        }

        div { style: "display:grid; grid-template-columns:260px 1fr 240px; height:calc(100vh - 180px); min-height:0",
            // Config rail
            div { style: "border-right:1px solid var(--bc-border); background:var(--bc-bg-card-solid); padding:20px; overflow-y:auto",
                div { class: "config-row",
                    label { class: "config-label", "渠道" }
                    div { class: "select-input", style: "width:100%; height:40px",
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

                div { class: "config-row",
                    label { class: "config-label", "Temperature" }
                    div { style: "display:flex; align-items:center; gap:12px",
                        input {
                            r#type: "range",
                            min: "0",
                            max: "2",
                            step: "0.1",
                            value: "{temperature():.1}",
                            style: "flex:1; accent-color:var(--bc-primary)",
                            oninput: move |e| {
                                if let Ok(v) = e.value().parse::<f64>() {
                                    temperature.set(v);
                                }
                            },
                        }
                        span { class: "mono", style: "font-size:13px; color:var(--bc-text-secondary); width:28px; text-align:right", "{temperature():.1}" }
                    }
                }

                div { class: "config-row",
                    label { class: "config-label", "Max tokens" }
                    div { class: "input sm", style: "width:100%",
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
                    label { style: "display:flex; align-items:center; justify-content:space-between; padding:6px 0; font-size:13px",
                        span { "流式响应" }
                        label { class: "switch",
                            input { r#type: "checkbox", checked: stream_mode(), onchange: move |e| stream_mode.set(e.checked()) }
                            span { class: "switch-track" }
                        }
                    }
                    label { style: "display:flex; align-items:center; justify-content:space-between; padding:6px 0; font-size:13px",
                        span { "显示推理过程" }
                        label { class: "switch",
                            input { r#type: "checkbox", checked: show_reasoning(), onchange: move |e| show_reasoning.set(e.checked()) }
                            span { class: "switch-track" }
                        }
                    }
                    label { style: "display:flex; align-items:center; justify-content:space-between; padding:6px 0; font-size:13px",
                        span { "JSON 模式" }
                        label { class: "switch",
                            input { r#type: "checkbox", checked: json_mode(), onchange: move |e| json_mode.set(e.checked()) }
                            span { class: "switch-track" }
                        }
                    }
                }
            }

            // Conversation
            div { style: "display:flex; flex-direction:column; min-height:0",
                div { style: "flex:1; overflow-y:auto; padding:24px; display:flex; flex-direction:column; gap:20px",
                    if msg_list.is_empty() {
                        div { style: "display:flex; align-items:center; justify-content:center; height:100%; color:var(--bc-text-secondary)",
                            "输入消息开始对话"
                        }
                    } else {
                        for msg in msg_list.iter() {
                            div { key: "{msg.content}", style: "display:flex; gap:12px; max-width:720px",
                                div { style: "width:32px; height:32px; border-radius:8px; background:{role_bg(&msg.role)}; color:{role_color(&msg.role)}; display:flex; align-items:center; justify-content:center; font-size:11px; font-weight:700; flex-shrink:0",
                                    "{role_label(&msg.role)}"
                                }
                                div { style: "flex:1",
                                    div { class: "config-label", style: "margin-bottom:4px", "{msg.role}" }
                                    div { style: "font-size:14px; line-height:1.6; white-space:pre-wrap", "{msg.content}" }
                                }
                            }
                        }
                    }
                }

                // Input bar
                div { style: "border-top:1px solid var(--bc-border); background:var(--bc-bg-card-solid); padding:16px; display:flex; gap:8px",
                    div { class: "input", style: "flex:1",
                        input {
                            r#type: "text",
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
            div { style: "border-left:1px solid var(--bc-border); background:var(--bc-bg-card-solid); padding:20px; overflow-y:auto; display:flex; flex-direction:column; gap:16px",
                label { class: "config-label", style: "margin-bottom:0", "Usage · this session" }

                div { class: "stat-card", style: "padding:14px; gap:4px",
                    span { class: "stat-eyebrow", "TOKENS" }
                    div { class: "stat-value", style: "font-size:22px", "{total_tokens}" }
                    span { class: "stat-foot", "{total_prompt_tokens} in · {total_completion_tokens} out" }
                }

                div { class: "stat-card", style: "padding:14px; gap:4px",
                    span { class: "stat-eyebrow", "COST" }
                    div { class: "stat-value", style: "font-size:22px",
                        "${cost_usd:.4}"
                    }
                    span { class: "stat-foot", "≈ ¥{cost_cny:.2}" }
                }

                div {
                    label { class: "config-label", "路由轨迹" }
                    div { class: "mono", style: "font-size:12px; color:var(--bc-text-secondary); line-height:1.9",
                        if total_tokens == 0 {
                            div { "暂无请求" }
                        } else {
                            div { "→ {total_tokens} tokens processed" }
                        }
                    }
                }
            }
        }
    }
}