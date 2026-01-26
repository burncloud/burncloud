use dioxus::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Channel {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub match_path: String,
    pub auth_type: String,
    pub priority: i32,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub param_override: Option<String>,
    pub header_override: Option<String>,
}

fn default_protocol() -> String {
    "openai".to_string()
}

#[component]
pub fn ChannelManager() -> Element {
    let mut channels = use_signal::<Vec<Channel>>(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(String::new);

    // Form state
    let mut form_name = use_signal(String::new);
    let mut form_base_url = use_signal(|| "https://api.openai.com".to_string());
    let mut form_api_key = use_signal(String::new);
    let mut form_match_path = use_signal(|| "/v1/chat/completions".to_string());
    let mut form_auth_type = use_signal(|| "Bearer".to_string());
    let mut form_protocol = use_signal(|| "openai".to_string());
    let mut form_param = use_signal(String::new);
    let mut form_header = use_signal(String::new);

    // Fetch channels
    use_resource(move || async move {
        let client = Client::new();
        match client
            .get("http://127.0.0.1:3000/console/api/channels")
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(data) = resp.json::<Vec<Channel>>().await {
                    channels.set(data);
                } else {
                    error_msg.set("Failed to parse channels".to_string());
                }
            }
            Err(e) => error_msg.set(format!("Failed to fetch: {}", e)),
        }
        loading.set(false);
    });

    let handle_create = move |_| async move {
        let client = Client::new();
        let new_channel = Channel {
            id: uuid::Uuid::new_v4().to_string(),
            name: form_name(),
            base_url: form_base_url(),
            api_key: form_api_key(),
            match_path: form_match_path(),
            auth_type: form_auth_type(),
            priority: 0,
            protocol: form_protocol(),
            param_override: if form_param().is_empty() {
                None
            } else {
                Some(form_param())
            },
            header_override: if form_header().is_empty() {
                None
            } else {
                Some(form_header())
            },
        };

        if let Ok(resp) = client
            .post("http://127.0.0.1:3000/console/api/channels")
            .json(&new_channel)
            .send()
            .await
        {
            if resp.status().is_success() {
                // Refresh list locally
                channels.write().push(new_channel);
                // Clear form
                form_name.set(String::new());
                form_api_key.set(String::new());
            } else {
                error_msg.set("Failed to create channel".to_string());
            }
        }
    };

    let handle_delete = move |id: String| async move {
        let client = Client::new();
        let url = format!("http://127.0.0.1:3000/console/api/channels/{}", id);
        if let Ok(resp) = client.delete(&url).send().await {
            if resp.status().is_success() {
                channels.write().retain(|c| c.id != id);
            }
        }
    };

    rsx! {
        div { class: "flex flex-col gap-lg",
            // Create Form
            div { class: "card p-lg",
                h3 { class: "text-subtitle font-semibold mb-md", "添加新渠道" }
                div { class: "grid gap-md", style: "grid-template-columns: 1fr 1fr;",
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "名称" }
                        input { class: "input",
                            value: "{form_name}",
                            oninput: move |e| form_name.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "协议类型" }
                        select { class: "input",
                            value: "{form_protocol}",
                            onchange: move |e| form_protocol.set(e.value()),
                            option { value: "openai", "OpenAI / Compatible" }
                            option { value: "vertex", "Vertex AI" }
                            option { value: "gemini", "Google Gemini" }
                            option { value: "claude", "Anthropic Claude" }
                        }
                    }
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "鉴权类型" }
                        select { class: "input",
                            value: "{form_auth_type}",
                            onchange: move |e| form_auth_type.set(e.value()),
                            option { value: "Bearer", "Bearer Token" }
                            option { value: "XApiKey", "X-Api-Key" }
                            option { value: "AwsSigV4", "AWS SigV4" }
                        }
                    }
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "Base URL" }
                        input { class: "input",
                            value: "{form_base_url}",
                            oninput: move |e| form_base_url.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "匹配路径" }
                        input { class: "input",
                            value: "{form_match_path}",
                            oninput: move |e| form_match_path.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-sm", style: "grid-column: span 2;",
                        label { class: "text-caption text-secondary",
                            if form_protocol() == "vertex" { "Service Account JSON" } else { "API Key / Token" }
                        }
                        input { class: "input", type: "password",
                            value: "{form_api_key}",
                            oninput: move |e| form_api_key.set(e.value())
                        }
                    }
                    // Advanced Config (Collapsible or just extra fields)
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "参数覆写 (JSON)" }
                        input { class: "input", placeholder: "e.g. project_id: my-project",
                            value: "{form_param}",
                            oninput: move |e| form_param.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "Header 覆写 (JSON)" }
                        input { class: "input",
                            value: "{form_header}",
                            oninput: move |e| form_header.set(e.value())
                        }
                    }
                }
                button { class: "btn btn-primary mt-md", onclick: handle_create,
                    "添加渠道"
                }
            }

            // List
            div { class: "card",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "渠道列表" }
                    if !error_msg().is_empty() {
                        div { class: "text-error mb-md", "{error_msg}" }
                    }
                    if loading() {
                        div { "加载中..." }
                    } else {
                        div { class: "flex flex-col gap-sm",
                            for channel in channels() {
                                div { class: "flex items-center justify-between p-sm bg-hover rounded",
                                    div {
                                        div { class: "font-medium", "{channel.name}" }
                                        div { class: "text-caption text-secondary",
                                            "{channel.protocol} | {channel.base_url} ({channel.match_path})"
                                        }
                                    }
                                    div { class: "flex gap-sm",
                                        span { class: "tag", "{channel.auth_type}" }
                                        button { class: "btn-icon text-error",
                                            onclick: move |_| handle_delete(channel.id.clone()),
                                            "🗑️"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
