// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{FormMode, SchemaForm};
use burncloud_client_shared::schema::deploy_schema;
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn DeployConfig() -> Element {
    let form_data = use_signal(|| serde_json::Value::Object(serde_json::Map::new()));
    let mut is_deploying = use_signal(|| false);
    let nav = use_navigator();
    let toast = use_toast();

    let schema = deploy_schema();

    let handle_deploy = move |value: serde_json::Value| {
        let model_id = value["model_id"].as_str().unwrap_or("").to_string();
        if model_id.is_empty() {
            return;
        }
        spawn(async move {
            is_deploying.set(true);
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            is_deploying.set(false);

            toast.success("Deployment Successful");
            nav.push("/console/models");
        });
    };

    rsx! {
        div { class: "flex flex-col h-full p-lg",
            // Header
            div { class: "mb-xl",
                h1 { class: "text-title font-bold text-primary mb-sm", "Model Deployment" }
                p { class: "text-secondary", "Deploy new models from various sources." }
            }

            // Form
            div { class: "max-w-2xl p-xl rounded-xl bc-card-solid",
                SchemaForm {
                    schema: schema.clone(),
                    data: form_data,
                    mode: FormMode::Create,
                    on_submit: handle_deploy,
                }
            }
        }
    }
}
