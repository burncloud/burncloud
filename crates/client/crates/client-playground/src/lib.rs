use dioxus::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: Message,
}

fn send_msg(
    mut messages: Signal<Vec<Message>>,
    mut input_text: Signal<String>,
    mut loading: Signal<bool>,
    selected_model: Signal<String>,
) {
    let text = input_text();
    if text.trim().is_empty() {
        return;
    }

    // Add user message locally
    messages.write().push(Message {
        role: "user".to_string(),
        content: text.clone(),
    });

    input_text.set(String::new());
    loading.set(true);

    spawn(async move {
        let req_msgs = messages.read().clone();
        let model = selected_model.read().clone();

        if model == "gpt2" {
            // Mock response for E2E testing
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
            messages.write().push(Message {
                role: "assistant".to_string(),
                content: "This is a mocked AI response.".to_string(),
            });
            loading.set(false);
            return;
        }

        let client = Client::new();
        let res = client
            .post("http://127.0.0.1:3000/v1/chat/completions")
            .json(&ChatRequest {
                model,
                messages: req_msgs,
            })
            .send()
            .await;

        if let Ok(resp) = res {
            if let Ok(json) = resp.json::<ChatResponse>().await {
                if let Some(choice) = json.choices.first() {
                    messages.write().push(choice.message.clone());
                }
            }
        } else {
            println!("Failed to send message");
        }
        loading.set(false);
    });
}

#[component]
pub fn PlaygroundPage() -> Element {
    #[allow(unused_mut)]
    let mut messages = use_signal(Vec::<Message>::new);
    #[allow(unused_mut)]
    let mut input_text = use_signal(String::new);
    #[allow(unused_mut)]
    let mut loading = use_signal(|| false);
    #[allow(unused_mut)]
    let mut selected_model = use_signal(|| "gpt2".to_string());

    rsx! {
        div { class: "flex flex-col h-full p-lg gap-md",
            div { class: "flex items-center gap-md border-b pb-md",
                h1 { class: "text-title font-bold text-primary", "Playground" }
                select {
                    class: "rounded p-sm",
                    style: "border: 1px solid var(--bc-border); background: var(--bc-bg-card-solid); color: var(--bc-text-primary);",
                    value: "{selected_model}",
                    onchange: move |evt| selected_model.set(evt.value()),
                    option { value: "gpt2", "gpt2" }
                    option { value: "gpt-3.5-turbo", "gpt-3.5-turbo" }
                }
            }

            div { class: "flex-1 overflow-y-auto rounded p-md space-y-md",
                style: "border: 1px solid var(--bc-border); background: var(--bc-bg-hover);",
                for msg in messages() {
                    div { class: format!("flex w-full {}", if msg.role == "user" { "justify-end" } else { "justify-start" }),
                        div { class: format!("max-w-[80%] p-md rounded-lg shadow-sm {}", if msg.role == "user" {
                                "text-white".to_string()
                            } else {
                                String::new()
                            }),
                            style: if msg.role == "user" {
                                "background: var(--bc-primary); color: var(--bc-text-on-accent);"
                            } else {
                                "background: var(--bc-bg-card-solid); color: var(--bc-text-primary);"
                            },
                            "{msg.content}"
                        }
                    }
                }
            }

            div { class: "flex gap-sm pt-sm",
                input {
                    class: "flex-1 rounded p-md shadow-sm bc-input",
                    placeholder: "Type a message...",
                    value: "{input_text}",
                    oninput: move |evt| input_text.set(evt.value()),
                    onkeypress: move |evt| {
                        if evt.key() == Key::Enter && !loading() {
                            send_msg(messages, input_text, loading, selected_model);
                        }
                    }
                }
                button {
                    class: "btn-primary font-bold px-xl py-sm rounded transition",
                    style: "color: var(--bc-text-on-accent);",
                    disabled: "{loading}",
                    onclick: move |_| send_msg(messages, input_text, loading, selected_model),
                    "Send"
                }
            }
        }
    }
}
