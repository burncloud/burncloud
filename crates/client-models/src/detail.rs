use dioxus::prelude::*;
use burncloud_service_models::ModelInfo;

#[component]
pub fn ModelDetail(model_id: String) -> Element {
    let mut model = use_signal(|| None::<ModelInfo>);

    use_effect(move || {
        let id = model_id.clone();
        spawn(async move {
            if let Ok(service) = burncloud_service_models::ModelService::new().await {
                if let Ok(Some(m)) = service.get(&id).await {
                    model.set(Some(m));
                }
            }
        });
    });

    let model_data = model.read();

    rsx! {
        div { class: "page-container",
            if let Some(m) = model_data.as_ref() {
                DetailView {
                    model_id: m.model_id.clone(),
                    is_private: m.private,
                    is_gated: m.gated,
                    is_disabled: m.disabled,
                    pipeline_tag: m.pipeline_tag.clone(),
                    library_name: m.library_name.clone(),
                    model_type: m.model_type.clone(),
                    downloads: m.downloads,
                    likes: m.likes,
                    size: m.size,
                    used_storage: m.used_storage,
                    sha: m.sha.clone(),
                    last_modified: m.last_modified.clone(),
                    created_at: m.created_at.clone(),
                    updated_at: m.updated_at.clone(),
                }
            } else {
                div { class: "loading", "加载中..." }
            }
        }
    }
}

#[component]
fn DetailView(
    model_id: String,
    is_private: bool,
    is_gated: bool,
    is_disabled: bool,
    pipeline_tag: Option<String>,
    library_name: Option<String>,
    model_type: Option<String>,
    downloads: i64,
    likes: i64,
    size: i64,
    used_storage: i64,
    sha: Option<String>,
    last_modified: Option<String>,
    created_at: String,
    updated_at: String,
) -> Element {
    rsx! {
        div { class: "model-detail",
            div { class: "detail-header",
                h1 { "{model_id}" }
                div { class: "badges",
                    if is_private { span { class: "badge private", "私有" } }
                    if is_gated { span { class: "badge gated", "需授权" } }
                    if is_disabled { span { class: "badge disabled", "已禁用" } }
                }
            }

            div { class: "detail-sections",
                section { class: "detail-section",
                    h2 { "基本信息" }
                    div { class: "info-grid",
                        InfoItem { label: "模型ID", value: model_id.clone() }
                        if let Some(pt) = pipeline_tag {
                            InfoItem { label: "管道类型", value: pt }
                        }
                        if let Some(lib) = library_name {
                            InfoItem { label: "库名称", value: lib }
                        }
                        if let Some(mt) = model_type {
                            InfoItem { label: "模型类型", value: mt }
                        }
                    }
                }

                section { class: "detail-section",
                    h2 { "统计信息" }
                    div { class: "info-grid",
                        InfoItem { label: "下载次数", value: format!("{downloads}") }
                        InfoItem { label: "点赞数", value: format!("{likes}") }
                        InfoItem { label: "文件大小", value: format_size(size) }
                        InfoItem { label: "已用存储", value: format_size(used_storage) }
                    }
                }

                section { class: "detail-section",
                    h2 { "版本信息" }
                    div { class: "info-grid",
                        if let Some(s) = sha {
                            InfoItem { label: "Git SHA", value: s }
                        }
                        if let Some(modified) = last_modified {
                            InfoItem { label: "最后修改", value: modified }
                        }
                        InfoItem { label: "创建时间", value: created_at.clone() }
                        InfoItem { label: "更新时间", value: updated_at.clone() }
                    }
                }
            }

            div { class: "detail-actions",
                button { class: "btn-primary", "编辑" }
                button { class: "btn-danger", "删除" }
                button { class: "btn-secondary", "返回" }
            }
        }
    }
}

#[component]
fn InfoItem(label: String, value: String) -> Element {
    rsx! {
        div { class: "info-item",
            span { class: "label", "{label}:" }
            span { class: "value", "{value}" }
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
