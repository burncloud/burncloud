use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::{
    backend::{chat_completion, first_active_api_token, ChannelService, ChatMessage, RouteTrace},
    components::Icon,
};

#[component]
pub fn Playground() -> Element {
    let mut channels = use_resource(move || async move { ChannelService::list(100).await });
    let channel_list = channels.read().clone().and_then(Result::ok).unwrap_or_default();
    let mut available_models = BTreeSet::new();
    for channel in &channel_list {
        if channel.status == 1 {
            for model in channel.models.split(',').map(str::trim).filter(|m| !m.is_empty()) {
                available_models.insert(model.to_string());
            }
        }
    }

    let mut model = use_signal(String::new);
    let mut prompt = use_signal(String::new);
    let mut messages = use_signal(Vec::<ChatMessage>::new);
    let mut temperature = use_signal(|| 0.7f64);
    let mut max_tokens = use_signal(|| 1024i64);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut trace = use_signal(RouteTrace::default);
    let mut usage_text = use_signal(String::new);

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Playground" }
                    p { class: "page-subtitle", "Sends a real non-streaming request to /v1/chat/completions using the first active BurnCloud API key." }
                }
                button { class: "button button-secondary", onclick: move |_| channels.restart(), "Refresh Models" }
            }

            div { class: "grid-2", style: "grid-template-columns:minmax(0,1fr) 360px;align-items:start",
                div { class: "card stack", style: "min-height:620px",
                    div { class: "card-pad row between", style: "border-bottom:1px solid var(--border)",
                        div { class: "field", style: "flex:1",
                            label { "Model" }
                            input { class: "input mono", value: "{model}", placeholder: "Enter an exact configured model ID", oninput: move |evt| model.set(evt.value()) }
                        }
                        button {
                            class: "button button-secondary button-sm",
                            onclick: move |_| {
                                messages.set(Vec::new());
                                trace.set(RouteTrace::default());
                                usage_text.set(String::new());
                                error.set(String::new());
                            },
                            "Clear"
                        }
                    }

                    if !available_models.is_empty() {
                        div { class: "card-pad row gap-2", style: "border-bottom:1px solid var(--border);flex-wrap:wrap",
                            span { class: "tiny subtle", "Available:" }
                            for available in available_models {
                                button { class: "button button-ghost button-sm mono", onclick: move |_| model.set(available.clone()), "{available}" }
                            }
                        }
                    }

                    div { class: "card-pad stack", style: "flex:1;overflow:auto;max-height:430px",
                        if messages().is_empty() {
                            div { class: "small muted", style: "margin:auto;text-align:center", "No messages yet. Choose a configured model and send a prompt." }
                        } else {
                            for (index, message) in messages().iter().enumerate() {
                                div { key: "{index}", class: if message.role == "user" { "card card-pad" } else { "terminal" },
                                    div { class: "tiny subtle mono", "{message.role}" }
                                    div { style: "white-space:pre-wrap", "{message.content}" }
                                }
                            }
                        }
                    }

                    if !error().is_empty() { div { class: "terminal auth-status auth-status-error", style: "margin:0 20px", "{error}" } }
                    if !usage_text().is_empty() { div { class: "tiny muted mono", style: "padding:0 20px", "{usage_text}" } }

                    div { class: "card-pad stack", style: "border-top:1px solid var(--border)",
                        textarea { class: "textarea", rows: "4", value: "{prompt}", placeholder: "Ask BurnCloud something…", disabled: loading(), oninput: move |evt| prompt.set(evt.value()) }
                        div { class: "row between",
                            span { class: "tiny subtle", "Uses a real active bc_live_* key from API Keys." }
                            button {
                                class: "button button-primary",
                                disabled: loading(),
                                onclick: move |_| {
                                    let model_id = model().trim().to_string();
                                    let text = prompt().trim().to_string();
                                    if model_id.is_empty() || text.is_empty() {
                                        error.set("Model and prompt are required.".to_string());
                                        return;
                                    }
                                    let mut request_messages = messages();
                                    request_messages.push(ChatMessage { role: "user".to_string(), content: text.clone() });
                                    messages.set(request_messages.clone());
                                    prompt.set(String::new());
                                    loading.set(true);
                                    error.set(String::new());
                                    usage_text.set("Sending request through BurnCloud router…".to_string());
                                    let temp = temperature();
                                    let max = max_tokens();
                                    spawn(async move {
                                        let result = async {
                                            let api_key = first_active_api_token().await?;
                                            chat_completion(&request_messages, &model_id, &api_key, temp, max).await
                                        }.await;
                                        match result {
                                            Ok(response) => {
                                                let mut next = request_messages;
                                                next.push(ChatMessage { role: "assistant".to_string(), content: response.content });
                                                messages.set(next);
                                                trace.set(response.trace);
                                                usage_text.set(format!("prompt={} • completion={} • total={}", response.usage.prompt_tokens, response.usage.completion_tokens, response.usage.total_tokens));
                                            }
                                            Err(message) => {
                                                error.set(format!("Chat request failed: {message}"));
                                                usage_text.set(String::new());
                                            }
                                        }
                                        loading.set(false);
                                    });
                                },
                                Icon { name: "play" }
                                if loading() { "Sending…" } else { "Send Request" }
                            }
                        }
                    }
                }

                div { class: "stack-lg",
                    div { class: "card card-pad stack",
                        span { class: "section-label", "Generation Controls" }
                        div { class: "field", label { "Temperature: {temperature}" } input { r#type: "range", min: "0", max: "2", step: "0.1", value: "{temperature}", oninput: move |evt| temperature.set(evt.value().parse().unwrap_or(0.7)) } }
                        div { class: "field", label { "Max Tokens" } input { class: "input", r#type: "number", min: "1", value: "{max_tokens}", oninput: move |evt| max_tokens.set(evt.value().parse().unwrap_or(1024)) } }
                    }
                    div { class: "card card-pad stack",
                        span { class: "section-label", "Last Router Trace" }
                        div { class: "receipt-row", label { "Channel" } strong { class: "mono", "{trace().channel_id.unwrap_or_else(|| "-".to_string())}" } }
                        div { class: "receipt-row", label { "Model Header" } strong { class: "mono", "{trace().model_id.unwrap_or_else(|| "-".to_string())}" } }
                        p { class: "tiny subtle", "Trace values come from X-Channel-Id and X-Model-Id response headers when provided by the router." }
                    }
                }
            }
        }
    }
}
