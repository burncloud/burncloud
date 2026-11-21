//! ChatHome 内的原生模型目录选择器。

use super::{
    catalog::{
        ArtifactStatus, CatalogClient, CatalogInspection, CatalogModel, CatalogPage,
        CatalogSelection, CatalogVersion, VersionStatus,
    },
    model::{gib, ChatError},
    state::ChatState,
};
use crate::shared::ui::{Icon, IconName};
use dioxus::{core::Task, prelude::*};

const SEARCH_KEYS: &str = "if(event.key==='ArrowDown'){event.preventDefault();document.querySelector('#chat-model-results [role=option]')?.focus();}if(event.key==='Enter'&&!event.isComposing){event.preventDefault();document.querySelector('#chat-model-results [role=option]')?.click();}";
const OPTION_KEYS: &str = "if(event.key==='ArrowDown'||event.key==='ArrowUp'){event.preventDefault();const options=Array.from(this.querySelectorAll('[role=option]'));const i=options.indexOf(document.activeElement);options[(i+(event.key==='ArrowDown'?1:-1)+options.length)%options.length]?.focus();}";

#[derive(Clone, Default)]
struct PickerState {
    query: String,
    models: Vec<CatalogModel>,
    preview: Option<CatalogModel>,
    preview_version: Option<CatalogVersion>,
    status: Option<VersionStatus>,
    artifact_status: Option<ArtifactStatus>,
    downloaded: bool,
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
                self.artifact_status = Some(inspection.artifact_status);
                self.downloaded = inspection.downloaded;
            }
            Err(error) => self.status_error = Some(error),
        }
    }
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
    let preview_version = snapshot.preview_version.clone();
    let has_version = preview_version.is_some();
    let busy = state.read().is_busy() || snapshot.confirmed;
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
            for model in snapshot.models {
                { let selected = snapshot.preview.as_ref().is_some_and(|preview| preview.id == model.id && preview.preset == model.preset); let key = format!("{}:{}", model.preset, model.id); let next = model.clone(); let retry = model.clone(); let client = status_client.clone(); let retry_client = status_client.clone(); let row_error = status_error.clone(); let resource_preview = selected && preview_version.as_ref().is_some_and(|version| !version.variants.is_empty()); rsx! {
                    // 复用原弹层行内状态，避免额外页面或第二套界面。
                    div { key: "{key}", class: if selected { "chat-model-row selected" } else { "chat-model-row" }, role: "presentation",
                        button { class: if selected { "chat-model-option selected" } else { "chat-model-option" }, r#type: "button", role: "option", aria_selected: selected,
                            onclick: move |_| load_status(picker, client.clone(), next.clone()),
                            span { class: "chat-model-option-copy",
                                strong { class: "chat-model-option-name", "{model.name}" }
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
                                }
                                match (artifact_status, status) {
                                    (Some(ArtifactStatus::Downloaded), _) if has_version => rsx! {
                                        span { class: "chat-row-status available", role: "status", "已下载" }
                                        button { class: "chat-row-text-button", r#type: "button", aria_label: "使用已下载版本", disabled: busy,
                                            onclick: move |_| confirm_selection(picker, state, on_confirm), "使用" }
                                    },
                                    (Some(ArtifactStatus::Downloading), _) => rsx! {
                                        span { class: "chat-row-status preparing", role: "status", "下载中..." }
                                    },
                                    (Some(ArtifactStatus::Verifying), _) => rsx! {
                                        span { class: "chat-row-status preparing", role: "status", "校验中..." }
                                    },
                                    (Some(ArtifactStatus::Failed), _) => rsx! {
                                        span { class: "chat-row-status failed", role: "status", "下载失败" }
                                        if has_version && status == Some(VersionStatus::Downloadable) {
                                            button { class: "chat-row-text-button", r#type: "button", disabled: busy, onclick: move |_| confirm_selection(picker, state, on_confirm), "重试下载" }
                                        }
                                    },
                                    (_, Some(VersionStatus::Available)) if has_version => rsx! {
                                        span { class: "chat-row-status available", "可用" }
                                        button { class: "chat-row-text-button", r#type: "button", aria_label: "使用此版本", disabled: busy,
                                            onclick: move |_| confirm_selection(picker, state, on_confirm), "使用" }
                                    },
                                    (_, Some(VersionStatus::Available)) => rsx! {
                                        span { class: "chat-row-status", title: "服务报告可用，但未提供可验证的仓库凭据", "暂不可用" }
                                    },
                                    (_, Some(VersionStatus::Downloadable)) if has_version => rsx! {
                                        button { class: "chat-row-icon-button", r#type: "button", aria_label: "下载", title: "下载", disabled: busy,
                                            onclick: move |_| confirm_selection(picker, state, on_confirm),
                                            Icon { name: IconName::Download, size: 16 }
                                        }
                                    },
                                    (_, Some(VersionStatus::Preparing)) => rsx! { span { class: "chat-row-status preparing", role: "status", "启用中..." } },
                                    (_, Some(VersionStatus::Developing | VersionStatus::Downloadable)) => rsx! { span { class: "chat-row-status", role: "status", "正在开发中..." } },
                                    (_, None) => rsx! {},
                                }
                            }
                        }
                    }
                } }
            }
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
    if choice.confirmed || choice.status_loading || state.peek().is_busy() {
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
