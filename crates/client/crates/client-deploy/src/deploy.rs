use burncloud_client_shared::components::{BCButton, BCInput, ButtonVariant};
use burncloud_client_shared::use_toast;
use dioxus::prelude::*;

#[component]
pub fn DeployConfig() -> Element {
    let mut model_id = use_signal(|| String::new());
    let mut source = use_signal(|| "HuggingFace".to_string());
    let mut is_deploying = use_signal(|| false);
    let nav = use_navigator();
    let toast = use_toast();

    let is_form_valid = !model_id().is_empty() && !source().is_empty();

    let handle_deploy = move |_| {
        spawn(async move {
            is_deploying.set(true);
            // Simulate deployment delay
            tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
            is_deploying.set(false);
            
            toast.success("Deployment Successful");
            nav.push("/console/models");
        });
    };

    rsx! {
        div { class: "flex flex-col h-full p-6",
            // Header
            div { class: "mb-8",
                h1 { class: "text-2xl font-bold text-base-content mb-2", "Model Deployment" }
                p { class: "text-base-content/60", "Deploy new models from various sources." }
            }

            // Form
            div { class: "max-w-2xl bg-base-100 p-8 rounded-xl border border-base-200 shadow-sm",
                div { class: "flex flex-col gap-6",
                    
                    // Source Selection
                    div { class: "form-control w-full",
                        label { class: "label",
                            span { class: "label-text font-medium", "Source" }
                        }
                        select { 
                            class: "select select-bordered w-full",
                            value: "{source}",
                            onchange: move |e| source.set(e.value()),
                            option { value: "HuggingFace", "HuggingFace" }
                            option { value: "Local", "Local Path" }
                        }
                    }

                    // Model ID Input
                    BCInput {
                        label: Some("Model ID".to_string()),
                        value: "{model_id}",
                        placeholder: "e.g. gpt2 or organization/model".to_string(),
                        oninput: move |e: FormEvent| model_id.set(e.value())
                    }

                    // Deploy Button
                    div { class: "mt-4",
                        BCButton {
                            class: "w-full btn-primary",
                            disabled: !is_form_valid || is_deploying(),
                            loading: is_deploying(),
                            onclick: handle_deploy,
                            "Deploy"
                        }
                    }
                }
            }
        }
    }
}