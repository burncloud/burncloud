use dioxus::prelude::*;
use burncloud_service_models::{ModelInfo, HfApiModel};

#[component]
pub fn ModelManagement() -> Element {
    let mut models = use_signal(Vec::<ModelInfo>::new);
    let mut show_search_dialog = use_signal(|| false);

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
            div { class: "flex justify-between items-center",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0",
                        "模型管理"
                    }
                    p { class: "text-secondary m-0 mt-sm",
                        "管理和查看已下载的AI模型"
                    }
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| show_search_dialog.set(true),
                    "➕ 添加模型"
                }
            }
        }

        if show_search_dialog() {
            SearchDialog {
                on_close: move |_| show_search_dialog.set(false),
                on_model_added: move |_| {
                    // 重新加载模型列表
                    spawn(async move {
                        if let Ok(service) = burncloud_service_models::ModelService::new().await {
                            if let Ok(list) = service.list().await {
                                models.set(list);
                            }
                        }
                    });
                }
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

            // 模型列表标题
            div { class: "mb-lg",
                h2 { class: "text-title font-semibold m-0", "模型列表" }
            }

            // 模型列表
            if models.read().is_empty() {
                // 空状态
                div { class: "card",
                    div { class: "p-xxxl text-center",
                        div { class: "flex flex-col items-center gap-lg",
                            div { class: "text-display", "📦" }
                            h3 { class: "text-title font-semibold m-0 text-secondary", "暂无模型数据" }
                            p { class: "text-secondary m-0", "当前还没有任何AI模型,点击上方"添加模型"按钮开始添加" }
                        }
                    }
                }
            } else {
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
        div { class: "card",
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

#[component]
fn SearchDialog(on_close: EventHandler<()>, on_model_added: EventHandler<()>) -> Element {
    let mut search_results = use_signal(Vec::<HfApiModel>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);

    // 自动加载模型列表
    use_effect(move || {
        spawn(async move {
            match burncloud_service_models::ModelService::fetch_from_huggingface().await {
                Ok(results) => {
                    search_results.set(results);
                    error_msg.set(None);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(format!("加载失败: {}", e)));
                    loading.set(false);
                }
            }
        });
    });

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); z-index: 9999; display: flex; align-items: center; justify-content: center;",
            onclick: move |_| on_close.call(()),

            div {
                class: "card",
                style: "width: 800px; max-height: 80vh; overflow: hidden; display: flex; flex-direction: column; position: relative; background: white;",
                onclick: move |e| e.stop_propagation(),

                // 标题栏
                div { class: "p-lg flex justify-between items-center",
                    style: "border-bottom: 1px solid var(--color-border);",
                    h2 { class: "text-title font-semibold m-0", "添加模型" }
                    button {
                        class: "btn btn-secondary",
                        onclick: move |_| on_close.call(()),
                        "✕"
                    }
                }

                // 内容区域
                div { class: "p-lg", style: "flex: 1; overflow-y: auto;",
                    if loading() {
                        div { class: "text-center p-xxxl",
                            div { class: "text-xl", "加载中..." }
                        }
                    } else if let Some(err) = error_msg() {
                        div { class: "card", style: "background: var(--color-danger-bg); border: 1px solid var(--color-danger);",
                            div { class: "p-lg",
                                p { class: "m-0 text-danger", "{err}" }
                            }
                        }
                    } else if search_results.read().is_empty() {
                        div { class: "text-center p-xxxl text-secondary",
                            "暂无搜索结果"
                        }
                    } else {
                        div { class: "flex flex-col gap-md",
                            for result in search_results.read().iter() {
                                SearchResultItem {
                                    key: "{result.id}",
                                    model: result.clone(),
                                    on_download: move |model| {
                                        spawn(async move {
                                            if let Err(e) = import_model_to_database(model).await {
                                                error_msg.set(Some(format!("导入失败: {}", e)));
                                            } else {
                                                on_model_added.call(());
                                                on_close.call(());
                                            }
                                        });
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

#[component]
fn SearchResultItem(model: HfApiModel, on_download: EventHandler<HfApiModel>) -> Element {
    rsx! {
        div { class: "card",
            div { class: "p-md flex justify-between items-center",
                div { class: "flex-1",
                    h3 { class: "text-body font-semibold m-0 mb-xs", "{model.id}" }
                    div { class: "flex gap-sm items-center",
                        if let Some(pipeline) = &model.pipeline_tag {
                            span { class: "badge badge-secondary text-caption", "{pipeline}" }
                        }
                        if let Some(library) = &model.library_name {
                            span { class: "badge badge-info text-caption", "{library}" }
                        }
                        if model.private.unwrap_or(false) {
                            span { class: "badge badge-warning text-caption", "🔒 私有" }
                        }
                    }
                    div { class: "flex gap-md mt-sm text-caption text-secondary",
                        if let Some(downloads) = model.downloads {
                            span { "⬇️ {format_number(downloads)}" }
                        }
                        if let Some(likes) = model.likes {
                            span { "❤️ {format_number(likes)}" }
                        }
                    }
                }
                button {
                    class: "btn btn-primary",
                    onclick: move |_| on_download.call(model.clone()),
                    "⬇️ 下载"
                }
            }
        }
    }
}

// 将 HuggingFace API 模型导入到本地数据库
async fn import_model_to_database(hf_model: HfApiModel) -> Result<(), Box<dyn std::error::Error>> {
    let service = burncloud_service_models::ModelService::new().await?;

    let model_info = ModelInfo {
        model_id: hf_model.id.clone(),
        private: hf_model.private.unwrap_or(false),
        pipeline_tag: hf_model.pipeline_tag.clone(),
        library_name: hf_model.library_name.clone(),
        model_type: None,
        downloads: hf_model.downloads.unwrap_or(0),
        likes: hf_model.likes.unwrap_or(0),
        sha: None,
        last_modified: None,
        gated: false,
        disabled: false,
        tags: serde_json::to_string(&hf_model.tags.unwrap_or_default())?,
        config: "{}".to_string(),
        widget_data: "[]".to_string(),
        card_data: "{}".to_string(),
        transformers_info: "{}".to_string(),
        siblings: "[]".to_string(),
        spaces: "[]".to_string(),
        safetensors: "{}".to_string(),
        used_storage: 0,
        filename: None,
        size: 0,
        created_at: hf_model.created_at.unwrap_or_else(|| {
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()
        }),
        updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    service.create(&model_info).await?;
    Ok(())
}
