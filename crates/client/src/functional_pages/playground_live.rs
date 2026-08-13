use std::collections::{BTreeMap, BTreeSet};

use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{
        chat_completion, ChannelService, ChatMessage, RouteTrace, TokenDto, TokenService, User,
        UserService,
    },
    components::Icon,
};

fn masked(token: &str) -> String {
    if token.len() <= 16 {
        return "••••••••".to_string();
    }
    format!("{}••••••••{}", &token[..8], &token[token.len() - 6..])
}

#[component]
pub fn Playground() -> Element {
    let mut channels_resource = use_resource(move || async move { ChannelService::list(100).await });
    let mut keys_resource = use_resource(move || async move { TokenService::list().await });
    let mut users_resource = use_resource(move || async move { UserService::list().await });

    let channel_snapshot = channels_resource.read().clone();
    let key_snapshot = keys_resource.read().clone();
    let user_snapshot = users_resource.read().clone();
    let channel_error = channel_snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let key_error = key_snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let user_error = user_snapshot.as_ref().and_then(|result| result.as_ref().err().cloned());
    let channels = channel_snapshot.and_then(Result::ok).unwrap_or_default();
    let keys: Vec<TokenDto> = key_snapshot.and_then(Result::ok).unwrap_or_default();
    let users: Vec<User> = user_snapshot.and_then(Result::ok).unwrap_or_default();

    let owner_names: BTreeMap<String, String> = users
        .iter()
        .map(|user| (user.id.clone(), user.username.clone()))
        .collect();
    let active_tokens: Vec<TokenDto> = keys
        .iter()
        .filter(|key| key.status == "active")
        .cloned()
        .collect();
    let active_channels = channels.iter().filter(|channel| channel.status == 1).count();
    let active_keys = active_tokens.len();
    let mut model_set = BTreeSet::new();
    for channel in &channels {
        if channel.status == 1 {
            for model in channel.models.split(',').map(str::trim).filter(|model| !model.is_empty()) {
                model_set.insert(model.to_string());
            }
        }
    }
    let available_models: Vec<String> = model_set.into_iter().collect();
    let model_count = available_models.len();
    let ready = active_channels > 0 && model_count > 0 && active_keys > 0;

    let mut model = use_signal(String::new);
    let mut selected_key = use_signal(String::new);
    let mut prompt = use_signal(String::new);
    let mut messages = use_signal(|| Vec::<ChatMessage>::new());
    let mut temperature = use_signal(|| 0.7f64);
    let mut max_tokens = use_signal(|| 1024i64);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut trace = use_signal(RouteTrace::default);
    let mut usage = use_signal(String::new);

    let trace_value = trace();
    let trace_channel = trace_value.channel_id.unwrap_or_else(|| "-".to_string());
    let trace_model = trace_value.model_id.unwrap_or_else(|| "-".to_string());
    let has_response = messages().iter().any(|message| message.role == "assistant");
    let selected_index = selected_key().parse::<usize>().ok();
    let selected_identity = selected_index
        .and_then(|index| active_tokens.get(index))
        .map(|key| {
            let owner = owner_names.get(&key.user_id).cloned().unwrap_or_else(|| key.user_id.clone());
            format!("{} • {}", owner, masked(&key.token))
        });
    let keys_for_send = active_tokens.clone();

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Playground" }
                    p { class: "page-subtitle", "Verify a real routed request with an explicit model and customer/API-key identity before sending production traffic." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        onclick: move |_| {
                            selected_key.set(String::new());
                            channels_resource.restart();
                            keys_resource.restart();
                            users_resource.restart();
                        },
                        "Refresh readiness"
                    }
                    if has_response {
                        Link { class: "button button-secondary", to: Route::Logs {}, "Open request logs" }
                    }
                }
            }

            if let Some(message) = channel_error {
                div { class: "terminal auth-status auth-status-error", "Providers could not be loaded: {message}" }
            }
            if let Some(message) = key_error {
                div { class: "terminal auth-status auth-status-error", "API keys could not be loaded: {message}" }
            }
            if let Some(message) = user_error {
                div { class: "product-note", "Owner names could not be loaded ({message}). API-key owner IDs are shown instead; test routing still works." }
            }

            div { class: if ready { "readiness-strip ready" } else { "readiness-strip blocked" },
                span { class: "readiness-dot" }
                if ready {
                    strong { "Ready for an end-to-end test" }
                    span { class: "muted", "{active_channels} active providers • {model_count} models • {active_keys} active API keys" }
                } else {
                    strong { "Playground is not ready yet" }
                    span { class: "muted", "Complete the missing setup item below before sending a request." }
                }
            }

            if active_channels == 0 {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "providers" } }
                        h3 { "Connect an active provider first" }
                        p { "Playground can only test real routing. Add an upstream provider and make sure it is active before choosing a model." }
                        Link { class: "button button-primary", to: Route::Providers {}, "Go to Providers" }
                    }
                }
            } else if model_count == 0 {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "models" } }
                        h3 { "No models are exposed" }
                        p { "Your provider exists, but it does not expose any model IDs. Add models to the provider configuration before testing traffic." }
                        Link { class: "button button-primary", to: Route::Providers {}, "Edit Provider Models" }
                    }
                }
            } else if active_keys == 0 {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "key" } }
                        h3 { "Create an API key for the test" }
                        p { "Playground uses the same BurnCloud API access path as an external client. Create an active API key before sending the request." }
                        Link { class: "button button-primary", to: Route::APIKeys {}, "Create API Key" }
                    }
                }
            } else {
                div { class: "grid-2", style: "grid-template-columns:minmax(0,1fr) 340px;align-items:start",
                    div { class: "card stack", style: "min-height:620px",
                        div { class: "card-pad stack", style: "border-bottom:1px solid var(--border);gap:12px",
                            div { class: "grid-2",
                                div { class: "field",
                                    label { "Model to test" }
                                    select {
                                        class: "select mono",
                                        value: "{model}",
                                        onchange: move |event| model.set(event.value()),
                                        option { value: "", "Select a configured model…" }
                                        for available_model in available_models.iter() {
                                            option { value: "{available_model}", "{available_model}" }
                                        }
                                    }
                                }
                                div { class: "field",
                                    label { "Charge test to / API key" }
                                    select {
                                        class: "select",
                                        value: "{selected_key}",
                                        onchange: move |event| {
                                            selected_key.set(event.value());
                                            messages.set(Vec::new());
                                            trace.set(RouteTrace::default());
                                            usage.set(String::new());
                                            error.set(String::new());
                                        },
                                        option { value: "", "Choose an account / API key…" }
                                        for (index, key) in active_tokens.iter().enumerate() {
                                            {
                                                let owner = owner_names.get(&key.user_id).cloned().unwrap_or_else(|| key.user_id.clone());
                                                let key_label = masked(&key.token);
                                                rsx! { option { value: "{index}", "{owner} — {key_label}" } }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "row between", style: "gap:16px",
                                span { class: "tiny subtle", "Changing the API key clears the conversation so one test session does not mix customer attribution." }
                                button {
                                    class: "button button-secondary button-sm",
                                    onclick: move |_| {
                                        messages.set(Vec::new());
                                        trace.set(RouteTrace::default());
                                        usage.set(String::new());
                                        error.set(String::new());
                                    },
                                    "Clear conversation"
                                }
                            }
                        }

                        div { class: "card-pad stack", style: "flex:1;overflow:auto;max-height:430px",
                            if messages().is_empty() {
                                div { class: "product-empty", style: "min-height:280px",
                                    div { class: "product-empty-inner",
                                        div { class: "product-empty-icon", Icon { name: "play" } }
                                        h3 { "Run a controlled routing test" }
                                        p { "Choose a model and the account/API key that should own this test, then verify which provider served the request." }
                                    }
                                }
                            } else {
                                for (index, message) in messages().iter().enumerate() {
                                    div {
                                        key: "{index}",
                                        class: if message.role == "user" { "card card-pad" } else { "terminal" },
                                        div { class: "tiny subtle mono", "{message.role}" }
                                        div { style: "white-space:pre-wrap;line-height:1.55", "{message.content}" }
                                    }
                                }
                            }
                        }

                        if !error().is_empty() {
                            div { class: "terminal auth-status auth-status-error", style: "margin:0 20px", "{error}" }
                        }
                        if !usage().is_empty() {
                            div { class: "tiny muted mono", style: "padding:0 20px", "{usage}" }
                        }

                        div { class: "card-pad stack", style: "border-top:1px solid var(--border)",
                            textarea {
                                class: "textarea",
                                rows: "4",
                                value: "{prompt}",
                                placeholder: "Enter a prompt that represents real traffic…",
                                disabled: loading(),
                                oninput: move |event| prompt.set(event.value()),
                            }
                            div { class: "row between",
                                span { class: "tiny subtle", "This request uses the selected real API key, so usage and billing follow that key's owner." }
                                button {
                                    class: "button button-primary",
                                    disabled: loading() || model().trim().is_empty() || selected_key().trim().is_empty() || prompt().trim().is_empty(),
                                    onclick: move |_| {
                                        let model_id = model().trim().to_string();
                                        let text = prompt().trim().to_string();
                                        let key_index = selected_key().trim().parse::<usize>().ok();
                                        let api_key = key_index.and_then(|index| keys_for_send.get(index).map(|key| key.token.clone()));
                                        if model_id.is_empty() || text.is_empty() {
                                            error.set("Choose a model and enter a prompt.".to_string());
                                            return;
                                        }
                                        let Some(api_key) = api_key else {
                                            error.set("Choose which account / API key should own this test.".to_string());
                                            return;
                                        };
                                        let mut request_messages = messages();
                                        request_messages.push(ChatMessage { role: "user".to_string(), content: text });
                                        messages.set(request_messages.clone());
                                        prompt.set(String::new());
                                        loading.set(true);
                                        error.set(String::new());
                                        usage.set("Routing request through BurnCloud…".to_string());
                                        let temp = temperature();
                                        let max = max_tokens();
                                        spawn(async move {
                                            match chat_completion(&request_messages, &model_id, &api_key, temp, max).await {
                                                Ok(response) => {
                                                    let mut next = request_messages;
                                                    next.push(ChatMessage { role: "assistant".to_string(), content: response.content });
                                                    messages.set(next);
                                                    trace.set(response.trace);
                                                    usage.set(format!(
                                                        "prompt={} • completion={} • total={}",
                                                        response.usage.prompt_tokens,
                                                        response.usage.completion_tokens,
                                                        response.usage.total_tokens
                                                    ));
                                                }
                                                Err(message) => {
                                                    error.set(format!("Request failed: {message}"));
                                                    usage.set(String::new());
                                                }
                                            }
                                            loading.set(false);
                                        });
                                    },
                                    Icon { name: "play" }
                                    if loading() { "Sending…" } else { "Send Test Request" }
                                }
                            }
                        }
                    }

                    div { class: "stack-lg",
                        div { class: "card card-pad stack",
                            div { class: "product-section-head",
                                div { h3 { "Test identity" } p { "Who this controlled request will be attributed to." } }
                            }
                            if let Some(identity) = selected_identity {
                                div { class: "readiness-strip ready",
                                    span { class: "readiness-dot" }
                                    strong { "{identity}" }
                                }
                            } else {
                                div { class: "product-note", "Choose an account/API key before sending. Playground will not silently pick the first active credential." }
                            }
                        }

                        div { class: "card card-pad stack",
                            div { class: "product-section-head",
                                div { h3 { "Request settings" } p { "Keep defaults unless your test needs specific generation behavior." } }
                            }
                            div { class: "field",
                                label { "Temperature: {temperature}" }
                                input { r#type: "range", min: "0", max: "2", step: "0.1", value: "{temperature}", oninput: move |event| temperature.set(event.value().parse().unwrap_or(0.7)) }
                            }
                            div { class: "field",
                                label { "Max output tokens" }
                                input { class: "input", r#type: "number", min: "1", value: "{max_tokens}", oninput: move |event| max_tokens.set(event.value().parse().unwrap_or(1024)) }
                            }
                        }

                        div { class: "card card-pad stack",
                            div { class: "product-section-head",
                                div { h3 { "Last route" } p { "Confirm which upstream handled the test." } }
                            }
                            div { class: "receipt-row", label { "Channel" } strong { class: "mono", "{trace_channel}" } }
                            div { class: "receipt-row", label { "Model" } strong { class: "mono", "{trace_model}" } }
                            if has_response {
                                Link { class: "button button-secondary", to: Route::Logs {}, "Inspect this request in Logs" }
                            } else {
                                p { class: "tiny subtle", "Route metadata appears after the first successful request when the router emits trace headers." }
                            }
                        }

                        div { class: "product-note",
                            "Playground is an operational smoke test, not a separate inference path. A successful response here verifies provider configuration, model exposure, the selected API access identity, and routing together."
                        }
                    }
                }
            }
        }
    }
}
