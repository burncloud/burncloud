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
    selected_model: Signal<String>
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
        // In a real app, use a configured base URL
        let res = client.post("http://127.0.0.1:3000/v1/chat/completions")
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
            // Handle error or just ignore for prototype
            println!("Failed to send message");
        }
        loading.set(false);
    });
}

#[component]
pub fn PlaygroundPage() -> Element {
    let mut messages = use_signal(|| Vec::<Message>::new());
    let mut input_text = use_signal(|| String::new());
    let mut loading = use_signal(|| false);
    let mut selected_model = use_signal(|| "gpt2".to_string());

    rsx! {
        div { class: "flex flex-col h-full p-6 gap-4",
            div { class: "flex items-center gap-4 border-b pb-4",
                h1 { class: "text-2xl font-bold", "Playground" }
                select {
                    class: "border rounded p-2",
                    value: "{selected_model}",
                    onchange: move |evt| selected_model.set(evt.value()),
                    option { value: "gpt2", "gpt2" }
                    option { value: "gpt-3.5-turbo", "gpt-3.5-turbo" }
                }
            }
            
            div { class: "flex-1 overflow-y-auto border rounded p-4 space-y-4 bg-gray-50",
                for msg in messages() {
                    div { class: format!("flex w-full {}", if msg.role == "user" { "justify-end" } else { "justify-start" }),
                        div { class: format!("max-w-[80%] p-3 rounded-lg shadow-sm {}", if msg.role == "user" { "bg-blue-500 text-white" } else { "bg-white text-gray-800" }),
                            "{msg.content}"
                        }
                    }
                }
            }

            div { class: "flex gap-2 pt-2",
                input {
                    class: "flex-1 border rounded p-3 shadow-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
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
                    class: "bg-blue-600 hover:bg-blue-700 text-white font-bold px-6 py-2 rounded transition disabled:opacity-50 disabled:cursor-not-allowed",
                    disabled: "{loading}",
                    onclick: move |_| send_msg(messages, input_text, loading, selected_model),
                    "Send"
                }
            }
        }
    }
}
