use dioxus::prelude::*;
use burncloud_service_models::{ModelInfo, HfApiModel};

use std::collections::HashMap;
use dioxus::prelude::*;
use burncloud_service_models::{ModelInfo, HfApiModel};
use burncloud_service_inference::InstanceStatus;

#[component]
pub fn ModelManagement() -> Element {
    let mut models = use_signal(Vec::<ModelInfo>::new);
    let mut statuses = use_signal(HashMap::<String, InstanceStatus>::new);
    let mut show_search_dialog = use_signal(|| false);
    let mut active_model_id = use_signal(|| None::<String>);
    let mut active_deploy_model_id = use_signal(|| None::<String>);

    // Load models
    use_effect(move || {
        spawn(async move {
            if let Ok(service) = burncloud_service_models::ModelService::new().await {
                if let Ok(list) = service.list().await {
                    models.set(list);
                }
            }
        });
    });

    // Poll statuses
    use_effect(move || {
        spawn(async move {
            loop {
                if let Ok(service) = burncloud_service_inference::InferenceService::new().await {
                    let current_models = models.read().clone();
                    let mut new_statuses = HashMap::new();
                    for m in current_models {
                        let status = service.get_status(&m.model_id).await;
                        new_statuses.insert(m.model_id, status);
                    }
                    statuses.set(new_statuses);
                }
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
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
        // ... dialogs ...
        if show_search_dialog() {
            SearchDialog {
                on_close: move |_| show_search_dialog.set(false),
                on_model_added: move |_| {
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

        if let Some(model_id) = active_model_id() {
            FileDownloadDialog {
                model_id: model_id,
                on_close: move |_| active_model_id.set(None),
            }
        }

        if let Some(model_id) = active_deploy_model_id() {
            DeployDialog {
                model_id: model_id,
                on_close: move |_| active_deploy_model_id.set(None),
                on_deploy_success: move |_| {
                    active_deploy_model_id.set(None);
                    // Status will update via polling
                }
            }
        }

        div { class: "page-content",
            // ... metrics ...
            div { class: "grid mb-xxxl",
                style: "grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: var(--spacing-lg);",
                // ... (same metric cards)
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
                // ... empty state
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
                            status: statuses.read().get(&model.model_id).cloned().unwrap_or(InstanceStatus::Stopped),
                            on_details: move |id| active_model_id.set(Some(id)),
                            on_deploy: move |id| active_deploy_model_id.set(Some(id)),
                            on_stop: move |id| {
                                let id_clone = id.clone();
                                spawn(async move {
                                    if let Ok(service) = burncloud_service_inference::InferenceService::new().await {
                                        let _ = service.stop_instance(&id_clone).await;
                                    }
                                });
                            },
                            on_delete: move |id| {
                                let id_clone = id.clone();
                                spawn(async move {
                                    if let Ok(service) = burncloud_service_models::ModelService::new().await {
                                        if let Ok(_) = service.delete(&id_clone).await {
                                            if let Ok(list) = service.list().await {
                                                models.set(list);
                                            }
                                        }
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
    status: InstanceStatus,
    on_details: EventHandler<String>,
    on_deploy: EventHandler<String>,
    on_stop: EventHandler<String>,
    on_delete: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "card",
            div { class: "p-lg",
                // 头部
                div { class: "flex justify-between items-start mb-md",
                    div { class: "flex-1",
                        div { class: "flex items-center gap-sm",
                            h3 { class: "text-subtitle font-semibold m-0 mb-xs", "{model_id}" }
                            if status == InstanceStatus::Running {
                                span { class: "badge badge-success", "🟢 运行中" }
                            } else if status == InstanceStatus::Starting {
                                span { class: "badge badge-warning", "🟡 启动中..." }
                            }
                        }
                        if let Some(pipeline) = pipeline_tag {
                            span { class: "badge badge-secondary text-caption", "{pipeline}" }
                        }
                    }
                    // ... badges
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
                    button { 
                        class: "btn btn-secondary flex-1",
                        onclick: move |_| on_details.call(model_id.clone()),
                        "📄 详情" 
                    }
                    
                    if status == InstanceStatus::Running || status == InstanceStatus::Starting {
                        button { 
                            class: "btn btn-danger flex-1", 
                            onclick: move |_| on_stop.call(model_id.clone()),
                            "🛑 停止" 
                        }
                    } else {
                        button { 
                            class: "btn btn-secondary flex-1",
                            onclick: move |_| on_deploy.call(model_id.clone()),
                            "🚀 部署" 
                        }
                    }

                    button { 
                        class: "btn btn-danger-outline",
                        onclick: move |_| on_delete.call(model_id.clone()),
                        "🗑️" 
                    }
                }
            }
        }
    }
}

#[component]
fn DeployDialog(model_id: String, on_close: EventHandler<()>, on_deploy_success: EventHandler<()>) -> Element {
    let mut files = use_signal(Vec::<String>::new);
    let mut selected_file = use_signal(|| None::<String>);
    let mut port = use_signal(|| 8080);
    let mut context_size = use_signal(|| 2048);
    let mut gpu_layers = use_signal(|| 0); // Default 0 (CPU), -1 for all
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);
    let mut deploy_status = use_signal(|| None::<String>);

    let model_id_clone = model_id.clone();
    use_effect(move || {
        let id = model_id_clone.clone();
        spawn(async move {
            match burncloud_service_models::get_model_files(&id).await {
                Ok(file_list) => {
                    // Filter only .gguf files
                    let ggufs: Vec<String> = burncloud_service_models::filter_gguf_files(&file_list)
                        .iter()
                        .map(|f| f[3].clone()) // path is index 3
                        .collect();
                    
                    if !ggufs.is_empty() {
                        selected_file.set(Some(ggufs[0].clone()));
                    }
                    files.set(ggufs);
                    loading.set(false);
                },
                Err(e) => {
                    error_msg.set(Some(format!("Failed to load files: {}", e)));
                    loading.set(false);
                }
            }
        });
    });

    let on_start = move |_| {
        let m_id = model_id.clone();
        let f_path = match selected_file() {
             Some(f) => f,
             None => {
                 error_msg.set(Some("请选择一个 GGUF 文件".to_string()));
                 return;
             }
        };
        // Get full path for the file
        // Ideally we need a helper to resolve relative path to absolute path
        // Assuming service-models downloads to data/{model_id}/{file_path}
        // And we need absolute path for llama-server
        
        let p = port();
        let ctx = context_size();
        let gpu = gpu_layers();

        spawn(async move {
            deploy_status.set(Some("正在启动服务...".to_string()));
            
            // Resolve absolute path
            let base_dir = match burncloud_service_models::get_data_dir().await {
                Ok(d) => d,
                Err(e) => {
                    error_msg.set(Some(format!("Config Error: {}", e)));
                    return;
                }
            };
            
            // Construct absolute path (simple join for now, need to handle OS specific)
            let abs_path = if std::path::Path::new(&base_dir).is_absolute() {
                 std::path::Path::new(&base_dir).join(&m_id).join(&f_path)
            } else {
                 match std::env::current_dir() {
                     Ok(cwd) => cwd.join(&base_dir).join(&m_id).join(&f_path),
                     Err(e) => {
                         error_msg.set(Some(format!("Path Error: {}", e)));
                         return;
                     }
                 }
            };

            let config = burncloud_service_inference::InferenceConfig {
                model_id: m_id.clone(),
                file_path: abs_path.to_string_lossy().to_string(),
                port: p as u16,
                context_size: ctx as u32,
                gpu_layers: gpu as i32,
            };

            match burncloud_service_inference::InferenceService::new().await {
                Ok(service) => {
                    match service.start_instance(config).await {
                        Ok(_) => {
                             deploy_status.set(Some("服务启动成功!".to_string()));
                             on_deploy_success.call(());
                        },
                        Err(e) => {
                             error_msg.set(Some(format!("启动失败: {}", e)));
                             deploy_status.set(None);
                        }
                    }
                },
                Err(e) => {
                     error_msg.set(Some(format!("Service Init Failed: {}", e)));
                }
            }
        });
    };

    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); z-index: 9999; display: flex; align-items: center; justify-content: center;",
            onclick: move |_| on_close.call(()),

            div {
                class: "card",
                style: "width: 500px; background: white;",
                onclick: move |e| e.stop_propagation(),

                div { class: "p-lg flex justify-between items-center border-b",
                    h2 { class: "text-title font-semibold m-0", "部署模型: {model_id}" }
                    button { class: "btn btn-secondary", onclick: move |_| on_close.call(()), "✕" }
                }

                div { class: "p-lg flex flex-col gap-md",
                    if loading() {
                        div { class: "text-center", "加载文件列表..." }
                    } else if files.read().is_empty() {
                         div { class: "text-danger", "未找到 GGUF 文件，请先下载模型文件。" }
                    } else {
                        // File Selection
                        div {
                            label { class: "block text-sm font-medium mb-xs", "选择文件 (GGUF)" }
                            select { 
                                class: "input w-full",
                                onchange: move |evt| selected_file.set(Some(evt.value())),
                                for f in files.read().iter() {
                                    option { value: "{f}", "{f}" }
                                }
                            }
                        }

                        // Port
                        div {
                            label { class: "block text-sm font-medium mb-xs", "端口 (Port)" }
                            input { 
                                class: "input w-full",
                                r#type: "number",
                                value: "{port}",
                                oninput: move |evt| port.set(evt.value().parse().unwrap_or(8080))
                            }
                        }

                        // Context Size
                        div {
                            label { class: "block text-sm font-medium mb-xs", "上下文长度 (Context Size)" }
                            select {
                                class: "input w-full",
                                onchange: move |evt| context_size.set(evt.value().parse().unwrap_or(2048)),
                                option { value: "2048", "2048" }
                                option { value: "4096", "4096" }
                                option { value: "8192", "8192" }
                                option { value: "16384", "16384" }
                                option { value: "32768", "32768" }
                            }
                        }

                        // GPU Layers
                        div {
                            label { class: "block text-sm font-medium mb-xs", "GPU 层数 (-1 = 全部, 0 = 仅CPU)" }
                            input {
                                class: "input w-full",
                                r#type: "number",
                                value: "{gpu_layers}",
                                oninput: move |evt| gpu_layers.set(evt.value().parse().unwrap_or(0))
                            }
                        }
                        
                        if let Some(err) = error_msg() {
                            div { class: "text-danger text-sm", "{err}" }
                        }
                        
                        if let Some(status) = deploy_status() {
                            div { class: "text-info text-sm", "{status}" }
                        }

                        div { class: "flex justify-end gap-sm mt-md",
                            button { 
                                class: "btn btn-secondary", 
                                onclick: move |_| on_close.call(()),
                                "取消" 
                            }
                            button { 
                                class: "btn btn-primary",
                                onclick: on_start,
                                "启动服务" 
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn FileDownloadDialog(model_id: String, on_close: EventHandler<()>) -> Element {
    let mut files = use_signal(Vec::<Vec<String>>::new);
    let mut loading = use_signal(|| true);
    let mut error_msg = use_signal(|| None::<String>);
    let mut download_status = use_signal(|| None::<String>);

    // Load files
    let model_id_clone = model_id.clone();
    use_effect(move || {
        let id = model_id_clone.clone();
        spawn(async move {
            match burncloud_service_models::get_model_files(&id).await {
                Ok(f) => {
                    files.set(f);
                    loading.set(false);
                },
                Err(e) => {
                    error_msg.set(Some(format!("获取文件列表失败: {}", e)));
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

                div { class: "p-lg flex justify-between items-center",
                    style: "border-bottom: 1px solid var(--color-border);",
                    h2 { class: "text-title font-semibold m-0", "模型文件: {model_id}" }
                    button { class: "btn btn-secondary", onclick: move |_| on_close.call(()), "✕" }
                }

                div { class: "p-lg", style: "flex: 1; overflow-y: auto;",
                    if let Some(status) = download_status() {
                        div { class: "card mb-md bg-secondary-bg",
                            div { class: "p-md", "🚀 {status}" }
                        }
                    }

                    if loading() {
                        div { class: "text-center p-xxxl", "加载中..." }
                    } else if let Some(err) = error_msg() {
                        div { class: "text-danger", "{err}" }
                    } else {
                        table { class: "w-full",
                            thead {
                                tr {
                                    th { class: "text-left p-sm", "文件名" }
                                    th { class: "text-left p-sm", "大小" }
                                    th { class: "text-right p-sm", "操作" }
                                }
                            }
                            tbody {
                                for file in files.read().iter() {
                                    // file format: [type, oid, size, path]
                                    tr { class: "border-b",
                                        td { class: "p-sm", "{file[3]}" }
                                        td { class: "p-sm text-secondary", 
                                            "{format_size(file[2].parse::<i64>().unwrap_or(0))}" 
                                        }
                                        td { class: "text-right p-sm",
                                            if file[3].ends_with(".gguf") {
                                                button {
                                                    class: "btn btn-sm btn-primary",
                                                    onclick: move |_| {
                                                        let m_id = model_id.clone();
                                                        let f_path = file[3].clone();
                                                        spawn(async move {
                                                            download_status.set(Some(format!("开始下载 {}...", f_path)));
                                                            match burncloud_service_models::download_model_file(&m_id, &f_path).await {
                                                                Ok(_) => download_status.set(Some(format!("已加入下载队列: {}", f_path))),
                                                                Err(e) => download_status.set(Some(format!("下载失败: {}", e))),
                                                            }
                                                        });
                                                    },
                                                    "下载"
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
