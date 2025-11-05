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
            div { class: "page-header",
                h1 { class: "text-large-title font-bold text-primary m-0", "加载中..." }
            }
            div { class: "page-content",
                div { class: "card",
                    div { class: "p-xxxl text-center text-secondary",
                        "正在加载模型详情..."
                    }
                }
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
        div { class: "page-header",
            div { class: "flex justify-between items-start",
                div {
                    h1 { class: "text-large-title font-bold text-primary m-0", "{model_id}" }
                    div { class: "flex gap-xs mt-sm",
                        if is_private {
                            span { class: "badge badge-warning", "🔒 私有" }
                        }
                        if is_gated {
                            span { class: "badge badge-info", "🔑 需授权" }
                        }
                        if is_disabled {
                            span { class: "badge badge-danger", "⚠️ 已禁用" }
                        }
                        if let Some(pt) = &pipeline_tag {
                            span { class: "badge badge-secondary", "{pt}" }
                        }
                    }
                }
                div { class: "flex gap-sm",
                    button { class: "btn btn-primary", "🚀 部署模型" }
                    button { class: "btn btn-secondary", "✏️ 编辑" }
                    button { class: "btn btn-danger-outline", "🗑️ 删除" }
                }
            }
        }

        div { class: "page-content",
            // 统计概览
            div { class: "grid mb-xxxl",
                style: "grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: var(--spacing-lg);",

                div { class: "card metric-card",
                    div { class: "flex flex-col gap-sm",
                        span { class: "text-secondary text-caption", "下载量" }
                        span { class: "text-xxl font-bold text-primary", "{format_number(downloads)}" }
                    }
                }

                div { class: "card metric-card",
                    div { class: "flex flex-col gap-sm",
                        span { class: "text-secondary text-caption", "点赞数" }
                        span { class: "text-xxl font-bold text-primary", "❤️ {format_number(likes)}" }
                    }
                }

                div { class: "card metric-card",
                    div { class: "flex flex-col gap-sm",
                        span { class: "text-secondary text-caption", "文件大小" }
                        span { class: "text-xxl font-bold text-primary", "{format_size(size)}" }
                    }
                }

                div { class: "card metric-card",
                    div { class: "flex flex-col gap-sm",
                        span { class: "text-secondary text-caption", "已用存储" }
                        span { class: "text-xxl font-bold text-primary", "{format_size(used_storage)}" }
                    }
                }
            }

            // 详细信息
            div { class: "grid gap-lg",
                style: "grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));",

                // 基本信息
                div { class: "card",
                    div { class: "p-lg",
                        h2 { class: "text-title font-semibold m-0 mb-lg", "基本信息" }
                        div { class: "flex flex-col gap-md",
                            InfoRow { label: "模型ID", value: model_id.clone() }
                            if let Some(pt) = pipeline_tag {
                                InfoRow { label: "管道类型", value: pt }
                            }
                            if let Some(lib) = library_name {
                                InfoRow { label: "库名称", value: lib }
                            }
                            if let Some(mt) = model_type {
                                InfoRow { label: "模型类型", value: mt }
                            }
                        }
                    }
                }

                // 版本信息
                div { class: "card",
                    div { class: "p-lg",
                        h2 { class: "text-title font-semibold m-0 mb-lg", "版本信息" }
                        div { class: "flex flex-col gap-md",
                            if let Some(s) = sha {
                                InfoRow { label: "Git SHA", value: s }
                            }
                            if let Some(modified) = last_modified {
                                InfoRow { label: "最后修改", value: modified }
                            }
                            InfoRow { label: "创建时间", value: created_at.clone() }
                            InfoRow { label: "更新时间", value: updated_at.clone() }
                        }
                    }
                }
            }

            // 快速操作
            div { class: "mt-xxxl",
                h2 { class: "text-title font-semibold mb-lg", "快速操作" }
                div { class: "flex gap-lg flex-wrap",
                    button { class: "btn btn-primary",
                        span { "🚀" }
                        "立即部署"
                    }
                    button { class: "btn btn-secondary",
                        span { "📊" }
                        "查看性能"
                    }
                    button { class: "btn btn-secondary",
                        span { "📝" }
                        "查看日志"
                    }
                    button { class: "btn btn-secondary",
                        span { "⚙️" }
                        "配置参数"
                    }
                    button { class: "btn btn-secondary",
                        span { "📤" }
                        "导出配置"
                    }
                    button { class: "btn btn-secondary",
                        span { "🔄" }
                        "检查更新"
                    }
                }
            }
        }
    }
}

#[component]
fn InfoRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "flex justify-between items-center",
            span { class: "text-secondary", "{label}" }
            span { class: "font-medium", "{value}" }
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
