// UI settings — HTTP response parsing — Value required; no feasible typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    ActionDef, ActionEvent, FormMode, SchemaForm, SchemaTable,
};
use burncloud_client_shared::schema::token_schema;
use burncloud_client_shared::token_service::TokenService;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn TokenManager() -> Element {
    let mut tokens = use_signal::<Vec<serde_json::Value>>(Vec::new);
    let mut loading = use_signal(|| true);
    let mut form_data = use_signal(|| serde_json::Value::Object(serde_json::Map::new()));
    let toast = use_toast();

    let schema = token_schema();

    let fetch_tokens = move || {
        let toast = toast;
        async move {
            loading.set(true);
            match TokenService::list().await {
                Ok(list) => {
                    let values: Vec<serde_json::Value> = list
                        .into_iter()
                        .filter_map(|t| serde_json::to_value(t).ok())
                        .collect();
                    tokens.set(values);
                }
                Err(e) => {
                    toast.error(&format!("加载令牌失败: {}", e));
                }
            }
            loading.set(false);
        }
    };

    // Initial load
    use_effect(move || {
        spawn(fetch_tokens());
    });

    let handle_create = move |value: serde_json::Value| {
        let toast = toast;
        spawn(async move {
            let user_id = value["user_id"].as_str().unwrap_or("").to_string();
            let quota_limit = value["quota_limit"].as_i64();

            match TokenService::create(&user_id, quota_limit).await {
                Ok(_) => {
                    toast.success("令牌创建成功");
                    form_data.set(serde_json::Value::Object(serde_json::Map::new()));
                    // Refresh list
                    if let Ok(list) = TokenService::list().await {
                        let values: Vec<serde_json::Value> = list
                            .into_iter()
                            .filter_map(|t| serde_json::to_value(t).ok())
                            .collect();
                        tokens.set(values);
                    }
                }
                Err(e) => {
                    toast.error(&format!("创建失败: {}", e));
                }
            }
        });
    };

    let actions = vec![ActionDef {
        action_id: "delete".to_string(),
        label: "删除".to_string(),
        color: "var(--bc-danger)".to_string(),
    }];

    let handle_action = move |event: ActionEvent| {
        if event.action_id == "delete" {
            let token_str = event.row["token"].as_str().unwrap_or("").to_string();
            let toast = toast;
            let mut tokens = tokens;
            spawn(async move {
                match TokenService::delete(&token_str).await {
                    Ok(_) => {
                        toast.success("令牌已删除");
                        tokens.write().retain(|t| t["token"].as_str() != Some(&token_str));
                    }
                    Err(e) => {
                        toast.error(&format!("删除失败: {}", e));
                    }
                }
            });
        }
    };

    rsx! {
        div { class: "flex flex-col gap-lg",
            // 创建表单
            div { class: "bc-card-solid",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "生成新令牌" }
                    SchemaForm {
                        schema: schema.clone(),
                        data: form_data,
                        mode: FormMode::Create,
                        on_submit: handle_create,
                    }
                }
            }

            // 令牌列表
            div { class: "bc-card-solid",
                div { class: "p-lg",
                    h3 { class: "text-subtitle font-semibold mb-md", "令牌列表" }
                    SchemaTable {
                        schema: schema.clone(),
                        data: tokens(),
                        loading: loading(),
                        actions: actions,
                        on_action: handle_action,
                        on_row_click: move |_| {},
                    }
                }
            }
        }
    }
}
