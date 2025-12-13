use dioxus::prelude::*;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Token {
    pub token: String,
    pub user_id: String,
    pub status: String,
    pub quota_limit: i64,
    pub used_quota: i64,
}

#[component]
pub fn TokenManager() -> Element {
    let mut tokens = use_signal::<Vec<Token>>(Vec::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(String::new);

    // Form
    let mut form_user_id = use_signal(String::new);
    let mut form_quota = use_signal(|| "-1".to_string()); // String for input

    let fetch_tokens = move || async move {
        loading.set(true);
        let client = Client::new();
        match client
            .get("http://127.0.0.1:3000/console/api/tokens")
            .send()
            .await
        {
            Ok(resp) => {
                if let Ok(data) = resp.json::<Vec<Token>>().await {
                    tokens.set(data);
                } else {
                    error_msg.set("Failed to parse tokens".to_string());
                }
            }
            Err(e) => error_msg.set(format!("Failed to fetch: {}", e)),
        }
        loading.set(false);
    };

    // Initial load
    use_effect(move || {
        spawn(async move {
            fetch_tokens().await;
        });
    });

    let handle_create = move |_| async move {
        let client = Client::new();
        let quota_parsed = form_quota().parse::<i64>().unwrap_or(-1);

        let body = serde_json::json!({
            "user_id": form_user_id(),
            "quota_limit": quota_parsed
        });

        if let Ok(resp) = client
            .post("http://127.0.0.1:3000/console/api/tokens")
            .json(&body)
            .send()
            .await
        {
            if resp.status().is_success() {
                // Refresh
                fetch_tokens().await;
                form_user_id.set(String::new());
                form_quota.set("-1".to_string());
            } else {
                error_msg.set("Failed to create token".to_string());
            }
        }
    };

    let handle_delete = move |token_str: String| async move {
        let client = Client::new();
        let url = format!("http://127.0.0.1:3000/console/api/tokens/{}", token_str);
        if let Ok(resp) = client.delete(&url).send().await {
            if resp.status().is_success() {
                tokens.write().retain(|t| t.token != token_str);
            }
        }
    };

    rsx! {
        div { class: "flex flex-col gap-lg",
            // Create Form
            div { class: "card p-lg",
                h3 { class: "text-subtitle font-semibold mb-md", "生成新令牌" }
                div { class: "grid gap-md", style: "grid-template-columns: 1fr 1fr auto;",
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "用户标识 (User ID)" }
                        input { class: "input",
                            value: "{form_user_id}",
                            placeholder: "e.g. user-123",
                            oninput: move |e| form_user_id.set(e.value())
                        }
                    }
                    div { class: "flex flex-col gap-sm",
                        label { class: "text-caption text-secondary", "额度限制 (-1 无限)" }
                        input { class: "input", type: "number",
                            value: "{form_quota}",
                            oninput: move |e| form_quota.set(e.value())
                        }
                    }
                    div { class: "flex items-end",
                        button { class: "btn btn-primary", onclick: handle_create,
                            "生成"
                        }
                    }
                }
            }

            // List
            div { class: "card",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "令牌列表" }
                     if !error_msg().is_empty() {
                        div { class: "text-error mb-md", "{error_msg}" }
                    }
                    if loading() {
                        div { "加载中..." }
                    } else {
                        div { class: "flex flex-col gap-sm",
                            for token in tokens() {
                                div { class: "flex items-center justify-between p-sm bg-hover rounded",
                                    div { class: "flex flex-col",
                                        div { class: "font-medium font-mono", "{token.token}" }
                                        div { class: "flex gap-md text-caption text-secondary",
                                            span { "User: {token.user_id}" }
                                            span { "Quota: {token.used_quota} / {token.quota_limit}" }
                                        }
                                    }
                                    div { class: "flex gap-sm",
                                        span { class: "tag", "{token.status}" }
                                        button { class: "btn-icon text-error",
                                            onclick: move |_| handle_delete(token.token.clone()),
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
