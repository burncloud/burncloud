use burncloud_client_shared::components::{BCBadge, BCTable, BadgeVariant};
use burncloud_service_inference::InstanceStatus;
use burncloud_service_models::{HfApiModel, ModelInfo};
use dioxus::prelude::*;
use std::collections::HashMap;

#[component]
pub fn ModelManagement() -> Element {
    // In a real app, this would be AccountService. For now, we alias ModelInfo as our data source.
    let mut models = use_signal(Vec::<ModelInfo>::new);
    let mut statuses = use_signal(HashMap::<String, InstanceStatus>::new);
    let mut show_search_dialog = use_signal(|| false);
    let mut active_model_id = use_signal(|| None::<String>);
    let mut active_deploy_model_id = use_signal(|| None::<String>);
    let mut active_delete_model_id = use_signal(|| None::<String>);

    // Load models (Simulating Account Groups)
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
                    h1 { class: "text-large-title font-bold text-base-content m-0",
                        "账号矩阵"
                    }
                    p { class: "text-gray-500 m-0 mt-sm tracking-wide",
                        "管理和监控云服务商账号资源池"
                    }
                }
                button {
                    class: "btn btn-neutral text-white rounded-lg",
                    onclick: move |_| show_search_dialog.set(true),
                    "➕ 采购资源"
                }
            }
        }

        // Dialogs (Kept functionality, renamed visually where possible)
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
                }
            }
        }

        if let Some(model_id) = active_delete_model_id() {
            DeleteConfirmDialog {
                model_id: model_id.clone(),
                on_close: move |_| active_delete_model_id.set(None),
                on_confirm: move |_| {
                    let id_clone = model_id.clone();
                    spawn(async move {
                        if let Ok(service) = burncloud_service_models::ModelService::new().await {
                            if service.delete(&id_clone).await.is_ok() {
                                if let Ok(list) = service.list().await {
                                    models.set(list);
                                }
                            }
                        }
                        active_delete_model_id.set(None);
                    });
                }
            }
        }

        div { class: "page-content",
            // Business Metrics - Liberated Numbers (No Borders, High Padding)
            div { class: "grid mb-12",
                style: "grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 2rem;",

                div { class: "flex flex-col p-4",
                    span { class: "text-xs font-bold text-gray-400 uppercase tracking-widest mb-3", "活跃账号组" }
                    span { class: "text-4xl font-bold text-base-content tracking-tight", "{models.read().len()}" }
                }
                div { class: "flex flex-col p-4 border-l border-base-200/50", // Subtle divider instead of box
                    span { class: "text-xs font-bold text-gray-400 uppercase tracking-widest mb-3", "总调用次数" }
                    span { class: "text-4xl font-bold text-base-content tracking-tight",
                        "{format_number(models.read().iter().map(|m| m.downloads).sum::<i64>() * 124)}"
                    }
                }
                div { class: "flex flex-col p-4 border-l border-base-200/50",
                    span { class: "text-xs font-bold text-gray-400 uppercase tracking-widest mb-3", "本月预估消耗" }
                    span { class: "text-4xl font-bold text-base-content tracking-tight",
                        "$ {format_number(models.read().iter().map(|m| m.size).sum::<i64>() / 1000000)}.00"
                    }
                }
            }

            // List Header with Separation
            div { class: "flex justify-between items-end pb-4 border-b border-base-200 mb-8",
                h2 { class: "text-lg font-bold text-base-content m-0", "资源池列表" }
                span { class: "text-xs font-bold text-gray-400 uppercase tracking-widest", "Live Status" }
            }

            // Account List
            if models.read().is_empty() {
                div { class: "card border border-dashed border-base-300 bg-base-100",
                    div { class: "p-16 text-center",
                        div { class: "flex flex-col items-center gap-6",
                            div { class: "text-7xl opacity-20 grayscale filter", "🗄️" }
                            div {
                                h3 { class: "text-xl font-bold text-base-content mb-2", "您的金库是空的" }
                                p { class: "text-sm text-gray-500 max-w-md mx-auto leading-relaxed",
                                    "立即点击上方 “采购资源” 填充您的资产矩阵，"
                                    br {}
                                    "开始构建您的自动化盈利引擎。"
                                }
                            }
                            button {
                                class: "btn btn-neutral btn-sm mt-2 rounded-lg",
                                onclick: move |_| show_search_dialog.set(true),
                                "立即采购"
                            }
                        }
                    }
                }
            } else {
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
                            on_details: move |id: String| active_model_id.set(Some(id)),
                            on_deploy: move |id: String| active_deploy_model_id.set(Some(id)),
                            on_stop: move |id: String| {
                                let id_clone = id.clone();
                                spawn(async move {
                                    if let Ok(service) = burncloud_service_inference::InferenceService::new().await {
                                        let _ = service.stop_instance(&id_clone).await;
                                    }
                                });
                            },
                            on_delete: move |id: String| {
                                active_delete_model_id.set(Some(id));
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
    let mid_details = model_id.clone();
    let _mid_stop = model_id.clone();
    let mid_deploy = model_id.clone();
    let mid_delete = model_id.clone();

    // Mocking Business Data from Technical Data
    let api_calls = downloads * 124;
    let success_rate = 99.9;
    let monthly_spend = size as f64 / 1000000.0; // Fake calculation

    rsx! {
        div { class: "card hover:shadow-md transition-all duration-200",
            div { class: "p-lg",
                // Header
                div { class: "flex justify-between items-start mb-md",
                    div { class: "flex-1",
                        div { class: "flex items-center gap-sm",
                            h3 { class: "text-subtitle font-bold m-0 mb-xs", "{model_id}" }
                        }
                        div { class: "flex items-center gap-2",
                            if status == InstanceStatus::Running {
                                div { class: "w-2 h-2 rounded-full bg-success animate-pulse" }
                                span { class: "text-xxs font-bold text-success uppercase tracking-wider", "Active" }
                            } else if status == InstanceStatus::Starting {
                                div { class: "w-2 h-2 rounded-full bg-warning animate-pulse" }
                                span { class: "text-xxs font-bold text-warning uppercase tracking-wider", "Auditing" }
                            } else {
                                div { class: "w-2 h-2 rounded-full bg-error" }
                                span { class: "text-xxs font-bold text-error uppercase tracking-wider", "Suspended" }
                            }
                        }
                    }
                    // Provider Badge (Mock)
                    div {
                        BCBadge { variant: BadgeVariant::Neutral, "AWS" }
                    }
                }

                // Business Stats Grid
                div { class: "grid grid-cols-2 gap-y-4 gap-x-2 mb-lg pt-2 border-t border-base-200",
                    div {
                        div { class: "text-xxs text-secondary uppercase tracking-widest mb-1", "API Calls (24h)" }
                        div { class: "font-mono font-medium text-sm", "{format_number(api_calls)}" }
                    }
                    div {
                        div { class: "text-xxs text-secondary uppercase tracking-widest mb-1", "Success Rate" }
                        div { class: "font-mono font-medium text-sm text-success", "{success_rate}%" }
                    }
                    div { class: "col-span-2",
                        div { class: "text-xxs text-secondary uppercase tracking-widest mb-1", "Monthly Spend" }
                        div { class: "font-mono font-bold text-lg", "$ {monthly_spend:.2}" }
                    }
                }

                // Action Buttons
                div { class: "flex gap-sm pt-2",
                    button {
                        class: "btn btn-sm btn-secondary flex-1",
                        onclick: move |_| on_details.call(mid_details.clone()),
                        "审计"
                    }

                    if status == InstanceStatus::Running {
                        button {
                            class: "btn btn-sm btn-neutral flex-1",
                            onclick: move |_| on_deploy.call(mid_deploy.clone()), // Reusing deploy as Scale/Edit
                            "扩容"
                        }
                    } else {
                        button {
                            class: "btn btn-sm btn-primary flex-1",
                            onclick: move |_| on_deploy.call(mid_deploy.clone()),
                            "激活"
                        }
                    }

                    button {
                        class: "btn btn-sm btn-error-outline px-3",
                        onclick: move |_| on_delete.call(mid_delete.clone()),
                        "清退"
                    }
                }
            }
        }
    }
}

#[component]
fn DeployDialog(
    model_id: String,
    on_close: EventHandler<()>,
    on_deploy_success: EventHandler<()>,
) -> Element {
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
                    let ggufs: Vec<String> =
                        burncloud_service_models::filter_gguf_files(&file_list)
                            .iter()
                            .map(|f| f[3].clone()) // path is index 3
                            .collect();

                    if !ggufs.is_empty() {
                        selected_file.set(Some(ggufs[0].clone()));
                    }
                    files.set(ggufs);
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(format!("Failed to load files: {}", e)));
                    loading.set(false);
                }
            }
        });
    });

    let m_id_start = model_id.clone();
    let on_start = move |_| {
        let m_id = m_id_start.clone();
        let f_path = match selected_file() {
            Some(f) => f,
            None => {
                error_msg.set(Some("请选择一个 GGUF 文件".to_string()));
                return;
            }
        };
        // ... logic ...
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
                Ok(service) => match service.start_instance(config).await {
                    Ok(_) => {
                        deploy_status.set(Some("服务启动成功!".to_string()));
                        on_deploy_success.call(());
                    }
                    Err(e) => {
                        error_msg.set(Some(format!("启动失败: {}", e)));
                        deploy_status.set(None);
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
                                {
                                    let current_files = files.read().clone();
                                    rsx! {
                                        for f in current_files.into_iter() {
                                            option { value: "{f}", "{f}" }
                                        }
                                    }
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
fn DeleteConfirmDialog(
    model_id: String,
    on_close: EventHandler<()>,
    on_confirm: EventHandler<()>,
) -> Element {
    rsx! {
        div {
            style: "position: fixed; top: 0; left: 0; right: 0; bottom: 0; background: rgba(0,0,0,0.5); z-index: 9999; display: flex; align-items: center; justify-content: center;",
            onclick: move |_| on_close.call(()),

            div {
                class: "card",
                style: "width: 400px; background: white;",
                onclick: move |e| e.stop_propagation(),

                div { class: "p-lg border-b",
                    h2 { class: "text-title font-semibold m-0", "确认删除?" }
                }

                div { class: "p-lg",
                    p { class: "mb-lg",
                        "您确定要删除模型 "
                        span { class: "font-bold", "{model_id}" }
                        " 吗？此操作无法撤销。"
                    }

                    div { class: "flex justify-end gap-sm",
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| on_close.call(()),
                            "取消"
                        }
                        button {
                            class: "btn btn-error",
                            onclick: move |_| on_confirm.call(()),
                            "确认删除"
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
                }
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
                        BCTable {
                            thead {
                                tr {
                                    th { "文件名" }
                                    th { "大小" }
                                    th { class: "text-right", "操作" }
                                }
                            }
                            tbody {
                                {
                                    let current_files = files.read().clone();
                                    rsx! {
                                        for file in current_files.into_iter() {
                                            // file format: [type, oid, size, path]
                                            tr { class: "border-b",
                                                td { "{file[3]}" }
                                                td { class: "text-secondary",
                                                    "{format_size(file[2].parse::<i64>().unwrap_or(0))}"
                                                }
                                                td { class: "text-right",
                                                    if file[3].ends_with(".gguf") {
                                                        {
                                                            let m_id_for_closure = model_id.clone();
                                                            let f_path_for_closure = file[3].clone();
                                                            rsx! {
                                                                button {
                                                                    class: "btn btn-sm btn-primary",
                                                                    onclick: move |_| {
                                                                        let m_id = m_id_for_closure.clone();
                                                                        let f_path = f_path_for_closure.clone();
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
                            BCBadge { variant: BadgeVariant::Neutral, "{pipeline}" }
                        }
                        if let Some(library) = &model.library_name {
                            BCBadge { variant: BadgeVariant::Info, "{library}" }
                        }
                        if model.private.unwrap_or(false) {
                            BCBadge { variant: BadgeVariant::Warning, "🔒 私有" }
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
        created_at: hf_model
            .created_at
            .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string()),
        updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };

    service.create(&model_info).await?;
    Ok(())
}
