use std::collections::BTreeSet;

use dioxus::prelude::*;
use serde::Deserialize;

use crate::{
    app::Route,
    backend::{
        first_active_api_token, server_root, use_auth, ChannelService, ChatMessage, ChatResult,
        ChatUsage, RouteTrace, TokenDto, TokenService,
    },
    components::Icon,
};

#[derive(Debug, Deserialize)]
struct PlaygroundChoiceMessage {
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PlaygroundChoice {
    message: Option<PlaygroundChoiceMessage>,
}

#[derive(Debug, Deserialize)]
struct PlaygroundChatResponse {
    #[serde(default)]
    choices: Vec<PlaygroundChoice>,
    #[serde(default)]
    usage: ChatUsage,
}

/// Send a smoke-test request through the authenticated console proxy. The client
/// supplies only the opaque token management reference; the bearer secret stays
/// server-side and is injected into the real data-plane router there.
async fn playground_chat_completion(
    messages: &[ChatMessage],
    model: &str,
    token_ref: &str,
    console_token: &str,
    temperature: f64,
    max_tokens: i64,
) -> Result<ChatResult, String> {
    if console_token.is_empty() {
        return Err("No authenticated console session".to_string());
    }

    let response = reqwest::Client::new()
        .post(format!("{}/console/api/playground/chat", server_root()))
        .header("Authorization", format!("Bearer {console_token}"))
        .json(&serde_json::json!({
            "token_ref": token_ref,
            "model": model,
            "messages": messages,
            "temperature": temperature,
            "max_tokens": max_tokens,
        }))
        .send()
        .await
        .map_err(|error| error.to_string())?;

    let status = response.status();
    let trace = RouteTrace {
        channel_id: response
            .headers()
            .get("X-Channel-Id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
        model_id: response
            .headers()
            .get("X-Model-Id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string),
    };
    let text = response.text().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("Chat request failed ({status}): {text}"));
    }

    let parsed: PlaygroundChatResponse = serde_json::from_str(&text)
        .map_err(|error| format!("Invalid chat response: {error}"))?;
    let content = parsed
        .choices
        .first()
        .and_then(|choice| choice.message.as_ref())
        .and_then(|message| message.content.clone())
        .unwrap_or_default();

    Ok(ChatResult {
        content,
        usage: parsed.usage,
        trace,
    })
}

#[component]
pub fn Playground() -> Element {
    let auth = use_auth();
    let console_token = auth.token().unwrap_or_default();
    let mut channels_resource = use_resource(move || async move { ChannelService::list(100).await });
    let mut keys_resource = use_resource(move || async move { TokenService::list().await });

    let channel_snapshot = channels_resource.read().clone();
    let key_snapshot = keys_resource.read().clone();
    let readiness_loading = channel_snapshot.is_none() || key_snapshot.is_none();
    let channel_error = channel_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let key_error = key_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let readiness_error = channel_error.is_some() || key_error.is_some();
    let channels = channel_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let keys: Vec<TokenDto> = key_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    let active_channels = channels.iter().filter(|channel| channel.status == 1).count();
    let active_keys = keys.iter().filter(|key| key.status == "active").count();
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
    let ready = !readiness_loading
        && !readiness_error
        && active_channels > 0
        && model_count > 0
        && active_keys > 0;

    let mut model = use_signal(String::new);
    let mut prompt = use_signal(String::new);
    let mut messages = use_signal(|| Vec::<ChatMessage>::new());
    let mut temperature = use_signal(|| 0.7f64);
    let mut max_tokens = use_signal(|| 1024i64);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut trace = use_signal(RouteTrace::default);
    let mut usage = use_signal(String::new);
    let mut last_requested_model = use_signal(String::new);

    let trace_value = trace();
    let trace_channel = trace_value.channel_id.unwrap_or_else(|| "-".to_string());
    let trace_model = trace_value.model_id.unwrap_or_else(|| "-".to_string());
    let has_response = messages().iter().any(|message| message.role == "assistant");
    let test_failed = ready && !loading() && !error().is_empty();
    let test_passed = ready && !loading() && error().is_empty() && has_response;

    let (test_status_class, test_status_title, test_status_copy) = if !ready {
        (
            "readiness-strip blocked playground-test-status",
            "Playground is not ready yet",
            "Complete the missing setup item below before sending a request.".to_string(),
        )
    } else if loading() {
        (
            "readiness-strip playground-test-running playground-test-status",
            "End-to-end test is running",
            "The request is going through BurnCloud API access, routing, and the selected upstream model.".to_string(),
        )
    } else if test_failed {
        (
            "readiness-strip playground-test-failed playground-test-status",
            "Last end-to-end test failed",
            "The prerequisites are configured, but the latest request did not complete successfully. Review the error and request logs before relying on this path.".to_string(),
        )
    } else if test_passed {
        (
            "readiness-strip ready playground-test-status",
            "End-to-end test passed",
            "A real request completed through BurnCloud using the configured API access and routing path.".to_string(),
        )
    } else {
        (
            "readiness-strip ready playground-test-status",
            "Ready for an end-to-end test",
            format!(
                "{active_channels} active providers • {model_count} models • {active_keys} active API keys"
            ),
        )
    };

    rsx! {
        div { class: "page",
            div { class: "page-header",
                div {
                    h2 { class: "page-title", "Playground" }
                    p { class: "page-subtitle", "Verify the complete path from API access to model routing and upstream response before sending real traffic." }
                }
                div { class: "header-actions",
                    button {
                        class: "button button-secondary",
                        disabled: readiness_loading || loading(),
                        onclick: move |_| {
                            channels_resource.restart();
                            keys_resource.restart();
                        },
                        if readiness_loading { "Refreshing…" } else { "Refresh readiness" }
                    }
                    if has_response {
                        Link { class: "button button-secondary", to: Route::Logs {}, "Open request logs" }
                    }
                }
            }

            if readiness_loading {
                div { class: "card product-empty playground-loading-state",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "play" } }
                        h3 { "Checking test prerequisites" }
                        p { "Reading active providers, exposed model IDs, and API access before enabling a real routed request." }
                    }
                }
            } else if readiness_error {
                div { class: "card card-pad stack playground-readiness-error",
                    div { class: "product-section-head",
                        div {
                            h3 { class: "danger", "Playground readiness could not be verified" }
                            p { "One or more prerequisite sources failed to load. BurnCloud will not guess that the test path is ready." }
                        }
                    }
                    if let Some(message) = channel_error.clone() {
                        code { class: "terminal", "Providers: {message}" }
                    }
                    if let Some(message) = key_error.clone() {
                        code { class: "terminal", "API keys: {message}" }
                    }
                    div { class: "product-actions",
                        button {
                            class: "button button-primary",
                            onclick: move |_| {
                                channels_resource.restart();
                                keys_resource.restart();
                            },
                            "Retry readiness check"
                        }
                    }
                }
            } else {
                div { class: "{test_status_class}",
                    span { class: "readiness-dot" }
                    div { class: "playground-status-copy",
                        strong { "{test_status_title}" }
                        span { class: "small muted", "{test_status_copy}" }
                    }
                    if ready {
                        span { class: "badge badge-neutral playground-status-meta", "3/3 prerequisites" }
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
                            p { "Your active provider exists, but it does not expose any model IDs. Add models to the provider configuration before testing traffic." }
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
                    div { class: "playground-workspace",
                        div { class: "card stack playground-conversation",
                            div { class: "card-pad row between playground-toolbar",
                                div { class: "field playground-model-field",
                                    label { "Model to test" }
                                    select {
                                        class: "select mono",
                                        value: "{model}",
                                        disabled: loading(),
                                        onchange: move |event| {
                                            model.set(event.value());
                                            error.set(String::new());
                                        },
                                        option { value: "", "Select a configured model…" }
                                        for available_model in available_models.iter() {
                                            option { value: "{available_model}", "{available_model}" }
                                        }
                                    }
                                }
                                button {
                                    class: "button button-secondary button-sm",
                                    disabled: loading(),
                                    onclick: move |_| {
                                        messages.set(Vec::new());
                                        trace.set(RouteTrace::default());
                                        usage.set(String::new());
                                        error.set(String::new());
                                        last_requested_model.set(String::new());
                                    },
                                    "Clear conversation"
                                }
                            }

                            div { class: "card-pad stack playground-thread",
                                if messages().is_empty() {
                                    div { class: "product-empty playground-thread-empty",
                                        div { class: "product-empty-inner",
                                            div { class: "product-empty-icon", Icon { name: "play" } }
                                            h3 { "Run a controlled routing test" }
                                            p { "Choose a model, send a representative prompt, then verify which provider served it and inspect the resulting request log." }
                                        }
                                    }
                                } else {
                                    for (index, message) in messages().iter().enumerate() {
                                        div {
                                            key: "{index}",
                                            class: if message.role == "user" { "playground-message playground-message-user" } else { "playground-message playground-message-assistant" },
                                            div { class: "tiny subtle mono playground-message-role", "{message.role}" }
                                            div { class: "playground-message-content", "{message.content}" }
                                        }
                                    }
                                }
                            }

                            if !error().is_empty() {
                                div { class: "terminal auth-status auth-status-error playground-request-error", "{error}" }
                            }
                            if !usage().is_empty() {
                                div { class: "tiny muted mono playground-usage", "{usage}" }
                            }

                            div { class: "card-pad stack playground-composer",
                                textarea {
                                    class: "textarea",
                                    rows: "4",
                                    value: "{prompt}",
                                    placeholder: "Enter a prompt that represents real traffic…",
                                    disabled: loading(),
                                    oninput: move |event| {
                                        prompt.set(event.value());
                                        error.set(String::new());
                                    },
                                }
                                div { class: "playground-composer-footer",
                                    span { class: "tiny subtle", "Request goes through the same BurnCloud router and API-key path used by external clients." }
                                    button {
                                        class: "button button-primary",
                                        disabled: loading() || model().trim().is_empty() || prompt().trim().is_empty(),
                                        onclick: move |_| {
                                            let model_id = model().trim().to_string();
                                            let text = prompt().trim().to_string();
                                            if model_id.is_empty() || text.is_empty() {
                                                error.set("Choose a model and enter a prompt.".to_string());
                                                return;
                                            }

                                            let prior_messages = messages();
                                            let mut request_messages = prior_messages.clone();
                                            request_messages.push(ChatMessage {
                                                role: "user".to_string(),
                                                content: text.clone(),
                                            });
                                            messages.set(request_messages.clone());
                                            prompt.set(String::new());
                                            trace.set(RouteTrace::default());
                                            last_requested_model.set(model_id.clone());
                                            loading.set(true);
                                            error.set(String::new());
                                            usage.set("Routing request through BurnCloud…".to_string());
                                            let temp = temperature();
                                            let max = max_tokens();
                                            let console_auth = console_token.clone();

                                            spawn(async move {
                                                let result = async {
                                                    let token_ref = first_active_api_token().await?;
                                                    playground_chat_completion(
                                                        &request_messages,
                                                        &model_id,
                                                        &token_ref,
                                                        &console_auth,
                                                        temp,
                                                        max,
                                                    )
                                                    .await
                                                }
                                                .await;

                                                match result {
                                                    Ok(response) => {
                                                        let mut next = request_messages;
                                                        next.push(ChatMessage {
                                                            role: "assistant".to_string(),
                                                            content: response.content,
                                                        });
                                                        messages.set(next);
                                                        trace.set(response.trace);
                                                        usage.set(format!(
                                                            "Prompt {} • Completion {} • Total {} tokens",
                                                            response.usage.prompt_tokens,
                                                            response.usage.completion_tokens,
                                                            response.usage.total_tokens
                                                        ));
                                                    }
                                                    Err(message) => {
                                                        messages.set(prior_messages);
                                                        prompt.set(text);
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

                        div { class: "stack-lg playground-sidebar",
                            div { class: "card card-pad stack",
                                div { class: "product-section-head",
                                    div { h3 { "Request settings" } p { "Keep defaults unless your test needs specific generation behavior." } }
                                }
                                div { class: "field",
                                    label { "Temperature: {temperature}" }
                                    input {
                                        r#type: "range",
                                        min: "0",
                                        max: "2",
                                        step: "0.1",
                                        value: "{temperature}",
                                        disabled: loading(),
                                        oninput: move |event| temperature.set(event.value().parse().unwrap_or(0.7))
                                    }
                                }
                                div { class: "field",
                                    label { "Max output tokens" }
                                    input {
                                        class: "input",
                                        r#type: "number",
                                        min: "1",
                                        value: "{max_tokens}",
                                        disabled: loading(),
                                        oninput: move |event| max_tokens.set(event.value().parse().unwrap_or(1024))
                                    }
                                }
                            }

                            div { class: "card card-pad stack playground-evidence-card",
                                div { class: "product-section-head",
                                    div { h3 { "Last route" } p { "Confirm what the last successful test proved." } }
                                }
                                div { class: "receipt-row", label { "Requested model" } strong { class: "mono", if last_requested_model().is_empty() { "-" } else { "{last_requested_model}" } } }
                                div { class: "receipt-row", label { "Channel trace" } strong { class: "mono", "{trace_channel}" } }
                                div { class: "receipt-row", label { "Model trace" } strong { class: "mono", "{trace_model}" } }
                                if has_response {
                                    Link { class: "button button-secondary", to: Route::Logs {}, "Inspect request evidence in Logs" }
                                } else {
                                    p { class: "tiny subtle", "Route metadata appears after the first successful request when the router emits trace headers." }
                                }
                            }

                            div { class: "product-note",
                                "Playground is an operational smoke test, not a separate inference path. A successful response proves that provider configuration, model exposure, API access and routing worked together for that request. The bearer secret remains server-side during the test."
                            }
                        }
                    }
                }
            }
        }
    }
}
