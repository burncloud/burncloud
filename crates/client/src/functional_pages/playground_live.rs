use std::collections::BTreeSet;

use dioxus::prelude::*;

use crate::{
    app::Route,
    backend::{chat_completion, ChannelService, ChatMessage, RouteTrace, TokenDto, TokenService},
    components::Icon,
};

#[component]
pub fn Playground() -> Element {
    let mut channels_resource = use_resource(move || async move { ChannelService::list(100).await });
    let mut keys_resource = use_resource(move || async move { TokenService::list().await });

    let channel_snapshot = channels_resource.read().clone();
    let key_snapshot = keys_resource.read().clone();
    let channels_loading = channel_snapshot.is_none();
    let keys_loading = key_snapshot.is_none();
    let channel_error = channel_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
    let key_error = key_snapshot
        .as_ref()
        .and_then(|result| result.as_ref().err().cloned());
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

    let readiness_loading = channels_loading || keys_loading;
    let readiness_failed = channel_error.is_some() || key_error.is_some();
    let active_channels = channels.iter().filter(|channel| channel.status == 1).count();
    let active_keys = keys.iter().filter(|key| key.status == "active").count();
    let mut model_set = BTreeSet::new();
    for channel in &channels {
        if channel.status == 1 {
            for model in channel
                .models
                .split(',')
                .map(str::trim)
                .filter(|model| !model.is_empty())
            {
                model_set.insert(model.to_string());
            }
        }
    }
    let available_models: Vec<String> = model_set.into_iter().collect();
    let model_count = available_models.len();
    let prerequisites_ready = !readiness_loading
        && !readiness_failed
        && active_channels > 0
        && model_count > 0
        && active_keys > 0;

    let mut model = use_signal(String::new);
    let mut api_key = use_signal(String::new);
    let mut prompt = use_signal(String::new);
    let mut messages = use_signal(|| Vec::<ChatMessage>::new());
    let mut temperature = use_signal(|| 0.7f64);
    let mut max_tokens = use_signal(|| 1024i64);
    let mut loading = use_signal(|| false);
    let mut error = use_signal(String::new);
    let mut trace = use_signal(RouteTrace::default);
    let mut usage = use_signal(String::new);
    let mut last_result = use_signal(String::new);
    let mut last_requested_model = use_signal(String::new);

    let model_value = model();
    let model_available = available_models
        .iter()
        .any(|available| available == model_value.trim());
    let credential_supplied = !api_key().trim().is_empty();
    let prompt_supplied = !prompt().trim().is_empty();
    let can_send = prerequisites_ready
        && model_available
        && credential_supplied
        && prompt_supplied
        && !loading();

    let trace_value = trace();
    let route_trace_available = trace_value.channel_id.is_some() || trace_value.model_id.is_some();
    let trace_provider = trace_value.channel_id.as_deref().and_then(|channel_id| {
        channels
            .iter()
            .find(|channel| channel.id.to_string() == channel_id)
            .map(|channel| {
                if channel.name.trim().is_empty() {
                    format!("Channel {}", channel.id)
                } else {
                    channel.name.clone()
                }
            })
    });
    let trace_provider_text = trace_provider
        .or_else(|| trace_value.channel_id.clone())
        .unwrap_or_else(|| "Trace unavailable".to_string());
    let trace_model_text = trace_value
        .model_id
        .clone()
        .unwrap_or_else(|| "Trace unavailable".to_string());
    let last_result_value = last_result();
    let last_success = last_result_value == "success";
    let last_failed = last_result_value == "failed";
    let last_running = last_result_value == "running";
    let last_requested_model_value = last_requested_model();

    let readiness_class = if readiness_loading {
        "readiness-strip checking"
    } else if readiness_failed {
        "readiness-strip error"
    } else if prerequisites_ready {
        "readiness-strip ready"
    } else {
        "readiness-strip blocked"
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
                        disabled: readiness_loading,
                        onclick: move |_| {
                            channels_resource.restart();
                            keys_resource.restart();
                        },
                        if readiness_loading { "Checking…" } else { "Refresh readiness" }
                    }
                    if last_success {
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

            div { class: readiness_class,
                span { class: "readiness-dot" }
                if readiness_loading {
                    strong { "Checking Playground readiness" }
                    span { class: "muted", "Loading provider, model and API-key prerequisites." }
                } else if readiness_failed {
                    strong { "Readiness could not be verified" }
                    span { class: "muted", "One or more prerequisite sources failed to load. Retry before trusting this test surface." }
                } else if prerequisites_ready {
                    strong { "Environment prerequisites are ready" }
                    span { class: "muted", "{active_channels} active providers • {model_count} models • {active_keys} active API keys. Paste a saved bearer secret below to run the test." }
                } else {
                    strong { "Playground is not ready yet" }
                    span { class: "muted", "Complete the missing setup item below before sending a request." }
                }
            }

            if readiness_loading {
                div { class: "card product-empty playground-loading-state",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "play" } }
                        h3 { "Checking the real request path" }
                        p { "BurnCloud is loading providers and API-key metadata before deciding whether this environment can be tested." }
                    }
                }
            } else if readiness_failed {
                div { class: "card product-empty",
                    div { class: "product-empty-inner",
                        div { class: "product-empty-icon", Icon { name: "warning" } }
                        h3 { "Readiness is unavailable" }
                        p { "The console cannot safely infer missing setup while provider or API-key data is unavailable. Retry the prerequisite checks first." }
                        button {
                            class: "button button-primary",
                            onclick: move |_| {
                                channels_resource.restart();
                                keys_resource.restart();
                            },
                            "Retry readiness"
                        }
                    }
                }
            } else if active_channels == 0 {
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
                        p { "Playground uses the same BurnCloud API access path as an external client. Create an active API key and save its one-time bearer secret before sending the request." }
                        Link { class: "button button-primary", to: Route::APIKeys {}, "Create API Key" }
                    }
                }
            } else {
                div { class: "playground-layout",
                    div { class: "card playground-workbench",
                        div { class: "playground-config",
                            div { class: "field",
                                label { "Model to test" }
                                select {
                                    class: "select mono",
                                    value: "{model}",
                                    disabled: loading(),
                                    onchange: move |event| model.set(event.value()),
                                    option { value: "", "Select a configured model…" }
                                    for available_model in available_models.iter() {
                                        option { value: "{available_model}", "{available_model}" }
                                    }
                                }
                                small { class: "muted", "Only models exposed by active providers are selectable." }
                            }
                            div { class: "field",
                                label { "BurnCloud API key" }
                                input {
                                    class: "input mono",
                                    r#type: "password",
                                    autocomplete: "off",
                                    value: "{api_key}",
                                    disabled: loading(),
                                    placeholder: "Paste a saved bc_live_… bearer secret",
                                    oninput: move |event| api_key.set(event.value()),
                                }
                                div { class: "playground-secret-help",
                                    small { class: "muted", "Paste a saved BurnCloud API key. The bearer secret stays in this page session and is not persisted or read back from the management API." }
                                    Link { class: "button button-secondary button-sm", to: Route::APIKeys {}, "Manage keys" }
                                }
                            }
                        }

                        div { class: "playground-conversation",
                            if messages().is_empty() {
                                div { class: "product-empty playground-conversation-empty",
                                    div { class: "product-empty-inner",
                                        div { class: "product-empty-icon", Icon { name: "play" } }
                                        h3 { "Run a controlled routing test" }
                                        p { "Choose a model, supply a saved API key, send a representative prompt, then verify which provider served the request." }
                                    }
                                }
                            } else {
                                for (index, message) in messages().iter().enumerate() {
                                    div {
                                        key: "{index}",
                                        class: if message.role == "user" { "playground-message playground-message-user" } else { "playground-message playground-message-assistant" },
                                        div { class: "playground-message-role mono", "{message.role}" }
                                        div { class: "playground-message-content", "{message.content}" }
                                    }
                                }
                            }
                        }

                        if !error().is_empty() {
                            div { class: "terminal auth-status auth-status-error playground-feedback", "{error}" }
                        }
                        if !usage().is_empty() {
                            div { class: "tiny muted mono playground-usage", "{usage}" }
                        }

                        div { class: "playground-composer",
                            textarea {
                                class: "textarea",
                                rows: "4",
                                value: "{prompt}",
                                placeholder: "Enter a prompt that represents real traffic…",
                                disabled: loading(),
                                oninput: move |event| prompt.set(event.value()),
                            }
                            div { class: "playground-composer-meta",
                                span { class: "tiny subtle", "This request uses the same BurnCloud data-plane API-key and routing path as an external client." }
                                div { class: "row", style: "gap:8px",
                                    button {
                                        class: "button button-secondary button-sm",
                                        disabled: loading() || messages().is_empty(),
                                        onclick: move |_| {
                                            messages.set(Vec::new());
                                            trace.set(RouteTrace::default());
                                            usage.set(String::new());
                                            error.set(String::new());
                                            last_result.set(String::new());
                                            last_requested_model.set(String::new());
                                        },
                                        "Clear"
                                    }
                                    button {
                                        class: "button button-primary",
                                        disabled: !can_send,
                                        onclick: move |_| {
                                            if !prerequisites_ready {
                                                error.set("Refresh readiness and resolve missing prerequisites before sending a test.".to_string());
                                                return;
                                            }
                                            let model_id = model().trim().to_string();
                                            if !model_available || model_id.is_empty() {
                                                error.set("Select a currently available model before sending the test.".to_string());
                                                return;
                                            }
                                            let bearer_token = api_key().trim().to_string();
                                            if bearer_token.is_empty() {
                                                error.set("Paste a saved BurnCloud API key before sending the test.".to_string());
                                                return;
                                            }
                                            let text = prompt().trim().to_string();
                                            if text.is_empty() {
                                                error.set("Enter a prompt before sending the test.".to_string());
                                                return;
                                            }

                                            let mut request_messages = messages();
                                            request_messages.push(ChatMessage { role: "user".to_string(), content: text });
                                            messages.set(request_messages.clone());
                                            prompt.set(String::new());
                                            loading.set(true);
                                            error.set(String::new());
                                            trace.set(RouteTrace::default());
                                            usage.set("Routing request through BurnCloud…".to_string());
                                            last_result.set("running".to_string());
                                            last_requested_model.set(model_id.clone());
                                            let temp = temperature();
                                            let max = max_tokens();
                                            spawn(async move {
                                                match chat_completion(&request_messages, &model_id, &bearer_token, temp, max).await {
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
                                                        last_result.set("success".to_string());
                                                    }
                                                    Err(message) => {
                                                        error.set(format!("Request failed: {message}"));
                                                        trace.set(RouteTrace::default());
                                                        usage.set(String::new());
                                                        last_result.set("failed".to_string());
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
                    }

                    div { class: "playground-sidebar",
                        div { class: "card card-pad stack",
                            div { class: "product-section-head",
                                div { h3 { "Request settings" } p { "Keep defaults unless this smoke test needs specific generation behavior." } }
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
                                    oninput: move |event| temperature.set(event.value().parse().unwrap_or(0.7)),
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
                                    oninput: move |event| max_tokens.set(event.value().parse().unwrap_or(1024)),
                                }
                            }
                        }

                        div { class: "card card-pad stack",
                            div { class: "product-section-head",
                                div { h3 { "Last test" } p { "The result below belongs only to the most recent send action." } }
                            }
                            if last_running {
                                div { class: "playground-route-state checking",
                                    strong { "Routing request…" }
                                    span { class: "muted", "Waiting for the current data-plane response." }
                                }
                            } else if last_success {
                                div { class: "playground-route-state success",
                                    strong { "Request succeeded" }
                                    span { class: "muted", "BurnCloud returned a successful model response." }
                                }
                                div { class: "receipt-row", label { "Requested model" } strong { class: "mono", "{last_requested_model_value}" } }
                                div { class: "receipt-row", label { "Provider / channel" } strong { class: "mono", "{trace_provider_text}" } }
                                div { class: "receipt-row", label { "Routed model" } strong { class: "mono", "{trace_model_text}" } }
                                if !route_trace_available {
                                    p { class: "tiny subtle", "The response succeeded, but route trace headers were not present, so this page does not claim which upstream served it." }
                                }
                                Link { class: "button button-secondary", to: Route::Logs {}, "Inspect request logs" }
                            } else if last_failed {
                                div { class: "playground-route-state failed",
                                    strong { "Last test failed" }
                                    span { class: "muted", "No previous route receipt is reused for this result. Review the request error and Logs before retrying." }
                                }
                                if !last_requested_model_value.is_empty() {
                                    div { class: "receipt-row", label { "Requested model" } strong { class: "mono", "{last_requested_model_value}" } }
                                }
                                Link { class: "button button-secondary", to: Route::Logs {}, "Open request logs" }
                            } else {
                                div { class: "playground-route-state",
                                    strong { "No test sent yet" }
                                    span { class: "muted", "Route evidence appears here only after this page sends a request." }
                                }
                            }
                        }

                        div { class: "product-note",
                            "Playground is an operational smoke test, not a separate inference path. The management API intentionally cannot retrieve stored bearer secrets after creation or rotation, so the test credential is supplied explicitly for this page session."
                        }
                    }
                }
            }
        }
    }
}
