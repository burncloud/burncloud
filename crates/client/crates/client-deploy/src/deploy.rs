// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{FormMode, PageHeader, SchemaForm};
use burncloud_client_shared::i18n::{t, use_i18n};
use burncloud_client_shared::schema::deploy_schema;
use burncloud_client_shared::services::deploy_service::{DeployRequest, DeployService};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn DeployConfig() -> Element {
    let i18n = use_i18n();
    let lang = i18n.language;
    let form_data = use_signal(|| serde_json::Value::Object(serde_json::Map::new()));
    let mut is_deploying = use_signal(|| false);
    let nav = use_navigator();
    let toast = use_toast();

    let schema = deploy_schema();

    let handle_deploy = move |value: serde_json::Value| {
        let type_ = value["type"]
            .as_str()
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(1);
        let key = value["key"].as_str().unwrap_or("").to_string();
        let name = value["name"].as_str().unwrap_or("").to_string();
        let model_id = value["model_id"].as_str().unwrap_or("").to_string();
        let group = value["group"].as_str().unwrap_or("default").to_string();

        if key.is_empty() || name.is_empty() || model_id.is_empty() {
            toast.error("Please fill in all required fields");
            return;
        }

        spawn(async move {
            is_deploying.set(true);

            let req = DeployRequest {
                type_,
                key,
                name,
                models: model_id,
                group,
                weight: 1,
                priority: 1,
            };

            match DeployService::deploy(&req).await {
                Ok(_) => {
                    is_deploying.set(false);
                    toast.success("Deployment Successful");
                    nav.push("/console/models");
                }
                Err(e) => {
                    is_deploying.set(false);
                    toast.error(&e);
                }
            }
        });
    };

    rsx! {
        PageHeader {
            title: t(*lang.read(), "deploy.title").to_string(),
            subtitle: Some(t(*lang.read(), "deploy.subtitle").to_string()),
            subtitle_class: None,
            actions: None,
        }

        div { class: "page-content",
            div { class: "max-w-2xl p-bc-5 pb-bc-8 rounded-bc-md bc-card-solid",
                SchemaForm {
                    schema: schema.clone(),
                    data: form_data,
                    mode: FormMode::Create,
                    on_submit: handle_deploy,
                    disabled: is_deploying(),
                }
            }
        }
    }
}
