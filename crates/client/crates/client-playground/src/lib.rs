use burncloud_client_shared::components::{
    BCButton, PageHeader, StatKpi, StatusPill,
};
use burncloud_client_shared::services::channel_service::ChannelService;
use dioxus::prelude::*;

fn channel_status(status: i32) -> String {
    if status == 1 { "active".to_string() } else { "down".to_string() }
}

fn msg_bg(role: &str) -> String {
    if role == "user" { "var(--bc-bg-hover)" } else { "var(--bc-bg-card-solid)" }.to_string()
}

#[derive(Clone, PartialEq)]
struct ChatMessage {
    role: String,
    content: String,
    tokens: i64,
}

#[component]
pub fn Playground() -> Element {
    let mut messages = use_signal(Vec::<ChatMessage>::new);
    let mut input_text = use_signal(String::new);
    let mut selected_channel = use_signal(|| 0i64);
    let mut temperature = use_signal(|| 0.7f64);
    let mut max_tokens = use_signal(|| 2048i64);
    let mut sending = use_signal(|| false);
    let mut total_prompt_tokens = use_signal(|| 0i64);
    let mut total_completion_tokens = use_signal(|| 0i64);

    let channels = use_resource(move || async move {
        ChannelService::list(0, 50).await.unwrap_or_default()
    });

    let channel_list = channels.read().clone().unwrap_or_default();
    let active_channels: Vec<_> = channel_list.iter().filter(|c| c.status == 1).collect();

    let mut send_trigger = use_signal(|| 0u32);

    // Watch send_trigger and send when it changes
    use_effect(move || {
        let _ = send_trigger(); // subscribe
        let text = input_text.read().clone();
        if text.is_empty() || sending() { return; }

        sending.set(true);
        let user_msg = ChatMessage {
            role: "user".to_string(),
            content: text.clone(),
            tokens: 0,
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
                tokens: 42,
            };
            messages.write().push(assistant_msg);
            total_prompt_tokens += 15;
            total_completion_tokens += 42;
            sending.set(false);
        });
    });

    let msg_list = messages.read();
    let total_tokens = *total_prompt_tokens.read() + *total_completion_tokens.read();

    rsx! {
        PageHeader {
            title: "演练场",
            subtitle: Some("AI 对话测试与调试".to_string()),
        }

        div { class: "page-content",
            div { style: "display:grid; grid-template-columns:260px 1fr 240px; gap:20px; height:calc(100vh - 200px)",
                // Left: Config sidebar
                div { style: "border-right:1px solid var(--bc-border); padding-right:20px; overflow-y:auto",
                    div { class: "config-label", "渠道" }
                    select {
                        class: "select-input",
                        style: "width:100%; margin-bottom:16px",
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

                    div { class: "config-label", "温度 ({temperature():.1})" }
                    input {
                        r#type: "range",
                        min: "0",
                        max: "2",
                        step: "0.1",
                        value: "{temperature():.1}",
                        style: "width:100%; margin-bottom:16px",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<f64>() {
                                temperature.set(v);
                            }
                        },
                    }

                    div { class: "config-label", "最大 Token" }
                    input {
                        r#type: "number",
                        value: "{max_tokens}",
                        style: "width:100%; margin-bottom:16px",
                        oninput: move |e| {
                            if let Ok(v) = e.value().parse::<i64>() {
                                max_tokens.set(v);
                            }
                        },
                    }

                    if !active_channels.is_empty() {
                        div { style: "margin-top:24px",
                            div { class: "config-label", "渠道状态" }
                            for ch in &active_channels {
                                div { class: "config-row",
                                    span { style: "font-size:13px", "{ch.name}" }
                                    StatusPill {
                                        value: channel_status(ch.status)
                                    }
                                }
                            }
                        }
                    }
                }

                // Center: Chat area
                div { style: "display:flex; flex-direction:column; overflow:hidden",
                    // Messages
                    div { style: "flex:1; overflow-y:auto; padding:16px 0",
                        if msg_list.is_empty() {
                            div { style: "display:flex; align-items:center; justify-content:center; height:100%; color:var(--bc-text-secondary)",
                                "输入消息开始对话"
                            }
                        } else {
                            for msg in msg_list.iter() {
                                div {
                                    key: "{msg.content}",
                                    style: "margin-bottom:16px; padding:12px 16px; border-radius:12px; background: {msg_bg(&msg.role)}; border: 1px solid var(--bc-border)",
                                    div { style: "font-size:11px; text-transform:uppercase; letter-spacing:0.08em; color:var(--bc-text-secondary); margin-bottom:6px",
                                        "{msg.role}"
                                    }
                                    div { style: "font-size:14px; line-height:1.6",
                                        "{msg.content}"
                                    }
                                }
                            }
                        }
                    }

                    // Input bar
                    div { style: "display:flex; gap:12px; padding-top:16px; border-top:1px solid var(--bc-border)",
                        input {
                            r#type: "text",
                            value: "{input_text}",
                            placeholder: "输入消息...",
                            style: "flex:1; padding:10px 16px; border-radius:8px; border:1px solid var(--bc-border); background:var(--bc-bg-card-solid); color:var(--bc-text-primary); font-size:14px",
                            oninput: move |e| input_text.set(e.value()),
                            onkeypress: move |e: KeyboardEvent| {
                                if e.key() == Key::Enter {
                                    send_trigger += 1;
                                }
                            },
                        }
                        BCButton {
                            class: "btn-black",
                            disabled: sending(),
                            onclick: move |_| send_trigger += 1,
                            if sending() { "生成中..." } else { "发送" }
                        }
                    }
                }

                // Right: Token metering
                div { style: "border-left:1px solid var(--bc-border); padding-left:20px; overflow-y:auto",
                    div { class: "config-label", style: "margin-bottom:16px", "Token 计量" }
                    StatKpi {
                        label: "总 Token",
                        value: "{total_tokens}",
                    }
                    StatKpi {
                        label: "Prompt",
                        value: "{total_prompt_tokens}",
                    }
                    StatKpi {
                        label: "Completion",
                        value: "{total_completion_tokens}",
                    }

                    div { style: "margin-top:24px",
                        div { class: "config-label", "路由追踪" }
                        div { style: "font-size:12px; color:var(--bc-text-secondary); margin-top:8px",
                            "消息将通过选定渠道发送"
                        }
                    }
                }
            }
        }
    }
}
