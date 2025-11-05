use dioxus::prelude::*;
use burncloud_service_models::ModelInfo;

#[component]
pub fn ModelManagement() -> Element {
    let mut models = use_signal(Vec::<ModelInfo>::new);

    use_effect(move || {
        spawn(async move {
            if let Ok(service) = burncloud_service_models::ModelService::new().await {
                if let Ok(list) = service.list().await {
                    models.set(list);
                }
            }
        });
    });

    rsx! {
        div { class: "page-container",
            div { class: "page-header",
                h1 { "模型管理" }
                button { class: "btn-primary", "添加模型" }
            }

            div { class: "models-grid",
                for model in models.read().iter() {
                    ModelCard { key: "{model.model_id}", model_id: model.model_id.clone(),
                        pipeline_tag: model.pipeline_tag.clone(),
                        downloads: model.downloads,
                        likes: model.likes,
                        size: model.size,
                        is_private: model.private }
                }
            }
        }
    }
}

#[component]
fn ModelCard(
    model_id: String,
    pipeline_tag: Option<String>,
    downloads: i64,
    likes: i64,
    size: i64,
    is_private: bool,
) -> Element {
    rsx! {
        div { class: "model-card",
            div { class: "model-header",
                h3 { "{model_id}" }
                if is_private {
                    span { class: "badge private", "私有" }
                }
            }

            div { class: "model-info",
                if let Some(pipeline) = pipeline_tag {
                    div { class: "info-row",
                        span { class: "label", "管道类型:" }
                        span { "{pipeline}" }
                    }
                }

                div { class: "info-row",
                    span { class: "label", "下载量:" }
                    span { "{downloads}" }
                }

                div { class: "info-row",
                    span { class: "label", "点赞数:" }
                    span { "{likes}" }
                }

                div { class: "info-row",
                    span { class: "label", "大小:" }
                    span { "{format_size(size)}" }
                }
            }

            div { class: "model-actions",
                button { class: "btn-secondary", "详情" }
                button { class: "btn-danger", "删除" }
            }
        }
    }
}

fn format_size(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}
