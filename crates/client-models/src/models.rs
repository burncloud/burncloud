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
        div { class: "page-header",
            h1 { class: "text-large-title font-bold text-primary m-0",
                "模型管理"
            }
            p { class: "text-secondary m-0 mt-sm",
                "管理和查看已下载的AI模型"
            }
        }

        div { class: "page-content",
            // 统计信息卡片
            div { class: "grid mb-xxxl",
                style: "grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--spacing-lg);",

                div { class: "card metric-card",
                    div { class: "flex flex-col gap-sm",
                        span { class: "text-secondary text-caption", "总模型数" }
                        span { class: "text-xxl font-bold text-primary", "{models.read().len()}" }
                    }
                }

                div { class: "card metric-card",
                    div { class: "flex flex-col gap-sm",
                        span { class: "text-secondary text-caption", "总下载量" }
                        span { class: "text-xxl font-bold text-primary",
                            "{format_number(models.read().iter().map(|m| m.downloads).sum::<i64>())}"
                        }
                    }
                }

                div { class: "card metric-card",
                    div { class: "flex flex-col gap-sm",
                        span { class: "text-secondary text-caption", "总存储空间" }
                        span { class: "text-xxl font-bold text-primary",
                            "{format_size(models.read().iter().map(|m| m.size).sum::<i64>())}"
                        }
                    }
                }
            }

            // 操作栏
            div { class: "flex justify-between items-center mb-lg",
                h2 { class: "text-title font-semibold m-0", "模型列表" }
                button { class: "btn btn-primary",
                    "➕ 添加模型"
                }
            }

            // 模型列表
            div { class: "grid",
                style: "grid-template-columns: repeat(auto-fill, minmax(320px, 1fr)); gap: var(--spacing-lg);",

                for model in models.read().iter() {
                    ModelCard {
                        key: "{model.model_id}",
                        model_id: model.model_id.clone(),
                        pipeline_tag: model.pipeline_tag.clone(),
                        downloads: model.downloads,
                        likes: model.likes,
                        size: model.size,
                        is_private: model.private,
                        is_gated: model.gated,
                        is_disabled: model.disabled,
                    }
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
    is_gated: bool,
    is_disabled: bool,
) -> Element {
    rsx! {
        div { class: "card model-card-static",
            div { class: "p-lg",
                // 头部
                div { class: "flex justify-between items-start mb-md",
                    div { class: "flex-1",
                        h3 { class: "text-subtitle font-semibold m-0 mb-xs", "{model_id}" }
                        if let Some(pipeline) = pipeline_tag {
                            span { class: "badge badge-secondary text-caption", "{pipeline}" }
                        }
                    }
                    div { class: "flex gap-xs",
                        if is_private {
                            span { class: "badge badge-warning text-caption", "🔒 私有" }
                        }
                        if is_gated {
                            span { class: "badge badge-info text-caption", "🔑 需授权" }
                        }
                        if is_disabled {
                            span { class: "badge badge-danger text-caption", "⚠️ 已禁用" }
                        }
                    }
                }

                // 统计信息
                div { class: "flex flex-col gap-sm mb-md",
                    div { class: "flex justify-between items-center",
                        span { class: "text-secondary text-caption", "下载量" }
                        span { class: "font-medium", "{format_number(downloads)}" }
                    }
                    div { class: "flex justify-between items-center",
                        span { class: "text-secondary text-caption", "点赞数" }
                        span { class: "font-medium", "❤️ {format_number(likes)}" }
                    }
                    div { class: "flex justify-between items-center",
                        span { class: "text-secondary text-caption", "文件大小" }
                        span { class: "font-medium", "{format_size(size)}" }
                    }
                }

                // 操作按钮
                div { class: "flex gap-sm pt-md",
                    button { class: "btn btn-secondary flex-1", "📄 详情" }
                    button { class: "btn btn-secondary flex-1", "🚀 部署" }
                    button { class: "btn btn-danger-outline", "🗑️" }
                }
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

fn format_number(num: i64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}K", num as f64 / 1_000.0)
    } else {
        format!("{}", num)
    }
}
