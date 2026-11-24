//! ChatHome 内的原生模型目录选择器。

use super::{
    catalog::{
        ArtifactStatus, CatalogClient, CatalogInspection, CatalogModel, CatalogPage,
        CatalogSelection, CatalogVersion, IdentityStatus, UnavailableReason, VersionStatus,
    },
    model::{gib, ChatError},
    state::ChatState,
};
use crate::shared::ui::{Icon, IconName};
use dioxus::{core::Task, prelude::*};
use std::collections::HashSet;

const FEATURED_PRESET_IDS: [&str; 18] = [
    "qwen3-8b",
    "deepseek-r1-distill-qwen-7b",
    "meta-llama-3.1-8b-instruct",
    "gemma-3-4b-it",
    "mistral-7b-instruct-v0.2",
    "gpt-oss-20b",
    "phi-4-mini-reasoning",
    "granite-3.3-8b-instruct",
    "c4ai-command-r7b-12-2024",
    "qwen3-30b",
    "deepseek-r1-distill-qwen-32b",
    "meta-llama-3.1-70b-instruct",
    "gemma-3-27b-it",
    "gpt-oss-120b",
    "c4ai-command-r-plus",
    "nvidia-nemotron-3-super-120b-a12b-bf16",
    "olmo-3.1-32b-instruct-row-342",
    "granite-4.2-30b-gguf",
];

fn visible_models<'a>(
    models: &'a [CatalogModel],
    query: &str,
    show_more: bool,
) -> Vec<&'a CatalogModel> {
    if !query.trim().is_empty() {
        return models.iter().collect();
    }

    let capacity = if show_more {
        models.len()
    } else {
        FEATURED_PRESET_IDS.len()
    };
    let mut visible = Vec::with_capacity(capacity);
    let mut seen = HashSet::with_capacity(capacity);
    for id in FEATURED_PRESET_IDS {
        if let Some(model) = models.iter().find(|model| model.preset && model.id == id) {
            if seen.insert(model.id.as_str()) {
                visible.push(model);
            }
        }
    }
    if show_more {
        for model in models {
            if seen.insert(model.id.as_str()) {
                visible.push(model);
            }
        }
    }
    visible
}

const SEARCH_KEYS: &str = "if(event.key==='ArrowDown'){event.preventDefault();document.querySelector('#chat-model-results [role=option]')?.focus();}if(event.key==='Enter'&&!event.isComposing){event.preventDefault();document.querySelector('#chat-model-results [role=option]')?.click();}";
const OPTION_KEYS: &str = "if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();const options=Array.from(this.querySelectorAll('[role=option]'));const i=options.indexOf(document.activeElement);options[(i+(event.key==='ArrowDown'?1:-1)+options.length)%options.length]?.focus();}";

#[derive(Clone, Default)]
struct PickerState {
    query: String,
    models: Vec<CatalogModel>,
    preview: Option<CatalogModel>,
    preview_version: Option<CatalogVersion>,
    status: Option<VersionStatus>,
    downloadable: bool,
    unavailable_reason: Option<UnavailableReason>,
    identity_status: Option<IdentityStatus>,
    artifact_status: Option<ArtifactStatus>,
    downloaded: bool,
    show_more: bool,
    loading: bool,
    status_loading: bool,
    feedback: Option<String>,
    status_error: Option<String>,
    confirmed: bool,
    list_id: u64,
    status_id: u64,
    list_task: Option<Task>,
    status_task: Option<Task>,
}
impl PickerState {
    fn clear_preview(&mut self) {
        if let Some(task) = self.status_task.take() {
            task.cancel();
        }
        self.status_id = self.status_id.wrapping_add(1);
        self.preview = None;
        self.preview_version = None;
        self.status = None;
        self.downloadable = false;
        self.unavailable_reason = None;
        self.identity_status = None;
        self.artifact_status = None;
        self.downloaded = false;
        self.status_error = None;
        self.status_loading = false;
        self.confirmed = false;
    }

    fn begin_list(&mut self) -> u64 {
        if let Some(task) = self.list_task.take() {
            task.cancel();
        }
        self.list_id = self.list_id.wrapping_add(1);
        self.loading = true;
        self.feedback = None;
        self.models.clear();
        self.clear_preview();
        self.list_id
    }

    fn finish_list(
        &mut self,
        id: u64,
        result: Result<CatalogPage<CatalogModel>, String>,
    ) -> Option<CatalogModel> {
        if id != self.list_id {
            return None;
        }
        match result {
            Ok(page) => {
                self.loading = !page.done;
                self.feedback = page.feedback();
                // 同一个去重行可能换成更高优先级来源，不能沿用旧来源的状态凭据。
                let changed = self.preview.as_ref().and_then(|previous| {
                    page.items
                        .iter()
                        .find(|model| model.id == previous.id && model.preset == previous.preset)
                        .filter(|model| *model != previous)
                        .cloned()
                });
                let removed = self.preview.as_ref().is_some_and(|previous| {
                    !page
                        .items
                        .iter()
                        .any(|model| model.id == previous.id && model.preset == previous.preset)
                });
                if removed || changed.is_some() {
                    self.clear_preview();
                }
                self.models = page.items;
                changed
            }
            Err(error) => {
                self.loading = false;
                // 已有部分结果也必须显示截断或传输错误，不能假装搜索完整成功。
                self.feedback = Some(error);
                None
            }
        }
    }

    fn begin_status(&mut self, model: CatalogModel) -> u64 {
        self.clear_preview();
        self.preview = Some(model);
        self.status_loading = true;
        self.status_id
    }

    fn finish_status(&mut self, id: u64, result: Result<CatalogInspection, String>) {
        if id != self.status_id {
            return;
        }
        self.status_loading = false;
        match result {
            Ok(inspection) => {
                self.preview_version = Some(inspection.version);
                self.status = Some(inspection.remote_status);
                self.downloadable = inspection.downloadable;
                self.unavailable_reason = inspection.unavailable_reason;
                self.identity_status = inspection.identity_status;
                self.artifact_status = Some(inspection.artifact_status);
                self.downloaded = inspection.downloaded;
            }
            Err(error) => self.status_error = Some(error),
        }
    }
}

fn can_confirm_selection(choice: &PickerState, chat_busy: bool) -> bool {
    if choice.confirmed
        || choice.status_loading
        || choice.status_error.is_some()
        || chat_busy
        || choice.identity_status == Some(IdentityStatus::PendingValidation)
    {
        return false;
    }
    let (Some(model), Some(version), Some(_status)) =
        (&choice.preview, &choice.preview_version, choice.status)
    else {
        return false;
    };
    if model.identity_status == Some(IdentityStatus::PendingValidation) {
        return false;
    }
    let has_verified_local = version
        .variants
        .iter()
        .any(|variant| version.verified_path(variant).is_some());
    let has_remote_candidate = choice.downloadable && !version.variants.is_empty();
    has_verified_local || has_remote_candidate
}

/// Chat 弹层内部组件；预览不会提交模型或启动硬件检测。
///
/// # Examples
/// 确认下载入口后通过 `on_confirm` 将检测任务交给仍然挂载的 ChatHome。
///
/// # Notes
/// 目录状态不是运行凭据，`available` 仍需由聊天状态验证本地凭据。
#[component]
pub(super) fn CatalogPicker(
    mut state: Signal<ChatState>,
    on_confirm: EventHandler<Option<u64>>,
) -> Element {
    let mut picker = use_signal(PickerState::default);
    let client = use_hook(|| {
        try_consume_context::<CatalogClient>()
            .map(Ok)
            .unwrap_or_else(CatalogClient::from_env)
    });
    let initial_client = client.clone();
    use_effect(move || {
        load_list(picker, initial_client.clone());
    });
    let snapshot = picker.read().clone();
    let refresh_client = client.clone();
    let search_client = client.clone();
    let retry_client = client.clone();
    let status_client = client.clone();
    let empty_initial =
        snapshot.loading && snapshot.models.is_empty() && snapshot.query.trim().is_empty();
    // 行内状态需要的字段先取出，避免与列表迭代的部分移动冲突。
    let status = snapshot.status;
    let artifact_status = snapshot.artifact_status;
    let status_loading = snapshot.status_loading;
    let status_error = snapshot.status_error.clone();
    let unavailable_reason = snapshot.unavailable_reason.clone();
    let identity_status = snapshot.identity_status;
    let downloadable = snapshot.downloadable;
    let downloaded = snapshot.downloaded;
    let preview_version = snapshot.preview_version.clone();
    let busy = state.read().is_busy() || snapshot.confirmed;
    let confirm_ready = can_confirm_selection(&snapshot, busy);
    let more_count = snapshot
        .models
        .len()
        .saturating_sub(FEATURED_PRESET_IDS.len());
    rsx! {
        div { class: "chat-catalog-heading",
            strong { "选择模型" }
            button { r#type: "button", class: "chat-inline-button", aria_label: "刷新模型列表", disabled: snapshot.loading,
                onclick: move |_| {
                    load_list(picker, refresh_client.clone());
                }, "刷新" }
        }
        input { class: "chat-model-search-input", r#type: "search", placeholder: "请输入模型名称", aria_label: "请输入模型名称",
            role: "combobox", aria_autocomplete: "list", aria_controls: "chat-model-results", aria_expanded: "true", value: snapshot.query,
            "onkeydown": SEARCH_KEYS,
            oninput: move |event| { picker.write().query = event.value(); load_list(picker, search_client.clone()); },
        }
        if let Some(feedback) = snapshot.feedback.as_deref() {
            div { class: "chat-catalog-feedback", role: "status", "{feedback}"
                button { r#type: "button", class: "chat-inline-button", aria_label: "重试模型查询", disabled: snapshot.loading,
                    onclick: move |_| load_list(picker, retry_client.clone()), "重试" }
            }
        }
        div { id: "chat-model-results", class: "chat-model-options", role: "listbox", aria_label: "模型搜索结果", "onkeydown": OPTION_KEYS,
            for model in visible_models(&snapshot.models, &snapshot.query, snapshot.show_more) {
                { let selected = snapshot.preview.as_ref().is_some_and(|preview| preview.id == model.id && preview.preset == model.preset); let key = format!("{}:{}", model.preset, model.id); let next = model.clone(); let retry = model.clone(); let client = status_client.clone(); let retry_client = status_client.clone(); let row_error = status_error.clone(); let resource_preview = selected && preview_version.as_ref().is_some_and(|version| !version.variants.is_empty()); let identity_pending = model.identity_status == Some(IdentityStatus::PendingValidation) || (selected && identity_status == Some(IdentityStatus::PendingValidation)); rsx! {
                    // 复用原弹层行内状态，避免额外页面或第二套界面。
                    div { key: "{key}", class: if selected { "chat-model-row selected" } else { "chat-model-row" }, role: "presentation",
                        button { class: if selected { "chat-model-option selected" } else { "chat-model-option" }, r#type: "button", role: "option", aria_selected: selected,
                            onclick: move |_| load_status(picker, client.clone(), next.clone()),
                            span { class: "chat-model-option-copy",
                                strong { class: "chat-model-option-name", "{model.name}" }
                                if model.identity_status == Some(IdentityStatus::PendingValidation) {
                                    span { class: "chat-row-status failed", role: "status", "待校验" }
                                }
                                if let Some(hardware) = &model.hardware {
                                    span { class: "chat-model-hardware",
                                        span { "最低" }
                                        span { "显存 {hardware.vram_min}" }
                                        span { "内存 {hardware.ram_min}" }
                                        span { "磁盘 {hardware.disk_min}" }
                                    }
                                    span { class: "chat-model-hardware",
                                        span { "推荐" }
                                        span { "显存 {hardware.vram_recommended}" }
                                        span { "内存 {hardware.ram_recommended}" }
                                        span { "磁盘 {hardware.disk_recommended}" }
                                    }
                                }
                            }
                        }
                        if let Some(link) = &model.resource_url {
                            a { class: "chat-row-icon-button chat-model-link", href: "{link}", target: "_blank", rel: "noopener noreferrer",
                                aria_label: "{model.name} 模型链接", title: "模型链接：{link}",
                                Icon { name: IconName::Globe, size: 16 }
                            }
                        }
                        if resource_preview {
                            div { class: "chat-model-resource",
                                button { class: "chat-model-resource-trigger", r#type: "button", aria_label: "查看 GGUF 资源配置", title: "查看 GGUF 资源配置",
                                    "资源"
                                }
                            }
                        }
                        if selected && !status_loading {
                            div { class: "chat-model-row-state", aria_live: "polite",
                                if let Some(error) = row_error {
                                    span { class: "chat-row-status failed", role: "alert", title: "{error}", "查询失败" }
                                    button { class: "chat-row-text-button", r#type: "button", aria_label: "重试版本状态",
                                        onclick: move |_| load_status(picker, retry_client.clone(), retry.clone()), "重试" }
                                } else if identity_pending {
                                    span { class: "chat-row-status failed", role: "status", "身份待校验" }
                                } else if artifact_status == Some(ArtifactStatus::Downloading) {
                                    span { class: "chat-row-status preparing", role: "status", "下载中..." }
                                } else if artifact_status == Some(ArtifactStatus::Verifying) {
                                    span { class: "chat-row-status preparing", role: "status", "校验中..." }
                                } else if artifact_status == Some(ArtifactStatus::Failed) {
                                    span { class: "chat-row-status failed", role: "status", "下载失败" }
                                    if downloadable && confirm_ready {
                                        button { class: "chat-row-text-button", r#type: "button", disabled: !confirm_ready,
                                            onclick: move |_| confirm_selection(picker, state, on_confirm), "重试下载" }
                                    }
                                } else if confirm_ready {
                                    if downloaded {
                                        span { class: "chat-row-status available", role: "status", "已验证本地工件" }
                                    } else {
                                        span { class: "chat-row-status available", role: "status", "可下载" }
                                    }
                                    button { class: "chat-row-text-button", r#type: "button", aria_label: "选择并检查硬件兼容性", disabled: !confirm_ready,
                                        onclick: move |_| confirm_selection(picker, state, on_confirm), "选择" }
                                } else if let Some(reason) = unavailable_reason.as_ref() {
                                    span { class: "chat-row-status failed", role: "status", title: "{reason.code}", "{reason.message}" }
                                } else if status == Some(VersionStatus::Preparing) {
                                    span { class: "chat-row-status preparing", role: "status", "启用中..." }
                                } else {
                                    span { class: "chat-row-status", role: "status", "暂不可用" }
                                }
                            }
                        }
                    }
                } }
            }
        }
        if snapshot.query.trim().is_empty() && more_count > 0 {
            button { class: "chat-inline-button", r#type: "button", aria_expanded: snapshot.show_more,
                onclick: move |_| {
                    let show_more = picker.peek().show_more;
                    picker.write().show_more = !show_more;
                },
                if snapshot.show_more { "收起更多" } else { "显示更多（{more_count}）" }
            }
        }
        if let Some(reason) = unavailable_reason.as_ref() {
            div { class: "chat-catalog-feedback", role: "status", title: "{reason.code}", "{reason.message}" }
        }
        if status_loading {
            div { class: "chat-catalog-feedback", role: "status", "正在检查模型工件状态…" }
        }
        if let Some(version) = preview_version
            .as_ref()
            .filter(|version| !version.variants.is_empty()) {
            div { class: "chat-model-preview", aria_label: "GGUF 资源配置",
                p { class: "chat-model-preview-title", "GGUF 资源配置" }
                p { class: "chat-model-preview-source", "{version.model} · {version.source} · {version.revision}" }
                for variant in version.variants.iter() {
                    div { class: "chat-model-preview-variant",
                        strong { "{variant.quantization}" }
                        small {
                            "最低内存 {gib(variant.ram_bytes)} GiB · "
                            if variant.vram_bytes == 0 {
                                "显存要求未声明"
                            } else {
                                "显存 {gib(variant.vram_bytes)} GiB"
                            }
                            " · 磁盘 {gib(variant.disk_bytes)} GiB"
                        }
                        small { "GGUF 仓库：{variant.artifact_source}" }
                    }
                }
            }
        }
        if empty_initial {
            p { class: "chat-search-empty", role: "status", "正在载入预置模型…" }
        }

    }
}

fn confirm_selection(
    mut picker: Signal<PickerState>,
    mut state: Signal<ChatState>,
    on_confirm: EventHandler<Option<u64>>,
) {
    let choice = picker.peek().clone();
    if !can_confirm_selection(&choice, state.peek().is_busy()) {
        return;
    }
    let (Some(model), Some(version), Some(status)) =
        (choice.preview, choice.preview_version, choice.status)
    else {
        return;
    };
    let selection = CatalogSelection {
        name: model.name,
        preset: model.preset.then_some(model.id),
        version: version.clone(),
    };
    let variants = version.variants;
    picker.write().confirmed = true;
    // 选择模块将服务端清单快照交给初始化流程，硬件检测后再筛选兼容版本。
    let run_id = state.with_mut(|value| value.accept_catalog(selection, status, variants));
    on_confirm.call(run_id);
}

fn load_list(mut picker: Signal<PickerState>, client: Result<CatalogClient, ChatError>) {
    let id = picker.with_mut(PickerState::begin_list);
    let query = picker.peek().query.clone();
    let status_client = client.clone();
    // spawn 的任务归弹层作用域所有，卸载弹层时会同时释放 HTTP 响应。
    let task = spawn(async move {
        let result = match client {
            Ok(client) => {
                client
                    .models(&query, |snapshot| {
                        let changed = picker.with_mut(|value| value.finish_list(id, Ok(snapshot)));
                        if let Some(model) = changed {
                            load_status(picker, status_client.clone(), model);
                        }
                    })
                    .await
            }
            Err(error) => Err(error),
        };
        if picker.peek().list_id == id {
            picker.write().list_task = None;
            if let Err(error) = result {
                picker.with_mut(|value| value.finish_list(id, Err(error.to_string())));
            }
        }
    });
    picker.write().list_task = Some(task);
}

fn load_status(
    mut picker: Signal<PickerState>,
    client: Result<CatalogClient, ChatError>,
    model: CatalogModel,
) {
    let id = picker.with_mut(|value| value.begin_status(model.clone()));
    let task = spawn(async move {
        let result = match client {
            Ok(client) => client.inspect(&model).await,
            Err(error) => Err(error),
        };
        if picker.peek().status_id == id {
            picker.write().status_task = None;
            picker.with_mut(|value| {
                value.finish_status(id, result.map_err(|error| error.to_string()))
            });
        }
    });
    picker.write().status_task = Some(task);
}

#[cfg(test)]
#[path = "selector_tests.rs"]
mod tests;
