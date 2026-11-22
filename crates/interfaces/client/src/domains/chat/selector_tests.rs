//! 目录浏览竞态与聊天流程边界的独立回归测试。
use super::super::{
    catalog::{CatalogSelection, UnavailableReason},
    model::{ErrorKind, Hardware, ModelVariant, PreparationStage},
};
use super::*;
use serde_json::json;

fn version(source: &str, revision: &str) -> CatalogVersion {
    CatalogVersion {
        verified_variants: Vec::new(),
        name: "Qwen3-8B".into(),
        model: "Qwen/Qwen3-8B".into(),
        revision: revision.into(),
        source: source.into(),
        resource_url: "https://modelscope.cn/models/Qwen/Qwen3-8B/files?Revision=master".into(),
        variants: Vec::new(),
    }
}
fn inspection(version: CatalogVersion, status: VersionStatus) -> CatalogInspection {
    CatalogInspection {
        version,
        remote_status: status,
        downloadable: false,
        unavailable_reason: Some(UnavailableReason {
            code: "no_gguf_artifact".into(),
            message: "没有可下载工件".into(),
        }),
        identity_status: None,
        artifact_status: ArtifactStatus::NotDownloaded,
        downloaded: false,
        local_path: None,
        actual_sha256: None,
    }
}
fn selection() -> CatalogSelection {
    CatalogSelection {
        name: "Qwen3-8B".into(),
        preset: Some("qwen3-8b".into()),
        version: version("modelscope", "master"),
    }
}
fn model(name: &str) -> CatalogModel {
    CatalogModel {
        id: name.to_lowercase(),
        name: name.into(),
        preset: false,
        model: format!("Qwen/{name}"),
        source: "modelscope".into(),
        revision: "master".into(),
        resource_url: None,
        hardware: None,
        identity_status: None,
    }
}
fn page(names: &[&str], done: bool) -> CatalogPage<CatalogModel> {
    serde_json::from_value(json!({"type": if done {Some("models")} else {None},
        "items": names.iter().map(|name| json!({"id":name.to_lowercase(),"name":name,"model":format!("Qwen/{name}"),"source":"modelscope","revision":"master"})).collect::<Vec<_>>(),"done":done})).expect("测试列表")
}
fn preset_model(id: &str) -> CatalogModel {
    let mut model = model(id);
    model.id = id.to_owned();
    model.preset = true;
    model.identity_status = Some(IdentityStatus::Verified);
    model
}

fn downloadable_variant() -> ModelVariant {
    ModelVariant {
        id: "Qwen3-8B-Q4_K_M.gguf".into(),
        quantization: "Q4_K_M".into(),
        runtime: "llama.cpp".into(),
        artifact_source:
            "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Qwen3-8B-Q4_K_M.gguf".into(),
        expected_sha256: format!("sha256:{}", "a".repeat(64)),
        ram_bytes: 1024,
        vram_bytes: 0,
        disk_bytes: 1024,
        file_size_bytes: Some(1024),
    }
}

fn downloadable_picker() -> PickerState {
    let mut version = version("modelscope", "master");
    version.variants.push(downloadable_variant());
    PickerState {
        preview: Some(model("Qwen3-8B")),
        preview_version: Some(version),
        status: Some(VersionStatus::Developing),
        downloadable: true,
        ..Default::default()
    }
}

#[test]
fn empty_query_shows_featured_presets_in_order_and_expands_the_unique_remainder() {
    const EXPECTED: [&str; 18] = [
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
    let mut models = EXPECTED
        .iter()
        .rev()
        .map(|id| preset_model(id))
        .collect::<Vec<_>>();
    models.extend((0..370).map(|index| preset_model(&format!("additional-{index}"))));

    let featured = visible_models(&models, "", false);
    assert_eq!(featured.len(), 18);
    assert_eq!(
        featured
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        EXPECTED.to_vec()
    );

    let expanded = visible_models(&models, "", true);
    assert_eq!(expanded.len(), 388);
    assert_eq!(
        expanded[..18]
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        EXPECTED.to_vec()
    );
    let unique = expanded
        .iter()
        .map(|model| model.id.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), 388);
    assert_eq!(visible_models(&models, "", false).len(), 18);
}

#[test]
fn external_search_results_are_not_truncated_by_the_featured_limit() {
    let models = (0..25)
        .map(|index| model(&format!("Qwen3-{index}B")))
        .collect::<Vec<_>>();
    let visible = visible_models(&models, "qwen", false);
    assert_eq!(visible.len(), models.len());
    assert_eq!(
        visible
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>(),
        models
            .iter()
            .map(|model| model.id.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn confirmation_requires_a_verified_candidate_and_rejects_pending_or_loading_state() {
    let ready = downloadable_picker();
    assert!(can_confirm_selection(&ready, false));

    let mut unavailable = ready.clone();
    unavailable.downloadable = false;
    assert!(!can_confirm_selection(&unavailable, false));

    let mut loading = ready.clone();
    loading.status_loading = true;
    assert!(!can_confirm_selection(&loading, false));

    let mut failed = ready.clone();
    failed.status_error = Some("状态查询失败".into());
    assert!(!can_confirm_selection(&failed, false));

    let mut pending = ready.clone();
    pending.identity_status = Some(IdentityStatus::PendingValidation);
    assert!(!can_confirm_selection(&pending, false));

    let mut pending_model = ready.clone();
    pending_model
        .preview
        .as_mut()
        .expect("预览模型")
        .identity_status = Some(IdentityStatus::PendingValidation);
    assert!(!can_confirm_selection(&pending_model, false));

    assert!(!can_confirm_selection(&ready, true));
}

#[test]
fn status_snapshot_exposes_unavailability_reason_and_identity_state() {
    let mut picker = PickerState::default();
    let id = picker.begin_status(model("Qwen3-8B"));
    let mut result = inspection(version("modelscope", "master"), VersionStatus::Developing);
    result.unavailable_reason = Some(UnavailableReason {
        code: "repository_unavailable".into(),
        message: "模型仓库暂不可访问".into(),
    });
    result.identity_status = Some(IdentityStatus::Verified);
    picker.finish_status(id, Ok(result));

    assert!(!picker.downloadable);
    assert_eq!(picker.identity_status, Some(IdentityStatus::Verified));
    assert_eq!(
        picker
            .unavailable_reason
            .as_ref()
            .map(|reason| reason.code.as_str()),
        Some("repository_unavailable")
    );
}

#[test]
fn stale_search_results_and_errors_are_ignored() {
    let mut state = PickerState {
        query: "llama".into(),
        ..Default::default()
    };
    let old = state.begin_list();
    state.query = "Qwen".into();
    let current = state.begin_list();
    assert!(state.finish_list(old, Err("迟到错误".into())).is_none());
    assert!(state.finish_list(old, Ok(page(&["Llama"], true))).is_none());
    assert!(state.loading && state.models.is_empty() && state.feedback.is_none());
    assert!(state
        .finish_list(current, Err("当前请求失败".into()))
        .is_none());
    assert!(!state.loading);
    assert_eq!(state.feedback.as_deref(), Some("当前请求失败"));

    let retry = state.begin_list();
    assert!(state.loading && state.feedback.is_none());
    assert!(state
        .finish_list(current, Ok(page(&["Old-Qwen"], true)))
        .is_none());
    assert!(state.models.is_empty());
    assert!(state
        .finish_list(retry, Ok(page(&["Qwen3-8B"], true)))
        .is_none());
    assert!(state
        .finish_list(current, Err("重试后的迟到错误".into()))
        .is_none());
    assert_eq!(state.models, vec![model("Qwen3-8B")]);
    assert!(!state.loading && state.feedback.is_none());
}

#[test]
fn catalog_snapshots_keep_all_results_in_server_order() {
    let names: Vec<_> = (0..18)
        .rev()
        .map(|index| format!("Model-{index:02}"))
        .collect();
    let labels: Vec<_> = names.iter().map(String::as_str).collect();
    let mut picker = PickerState::default();
    let id = picker.begin_list();
    assert!(picker.finish_list(id, Ok(page(&labels, true))).is_none());
    assert_eq!(picker.models.len(), 18);
    assert_eq!(
        picker
            .models
            .iter()
            .map(|item| item.name.as_str())
            .collect::<Vec<_>>(),
        labels
    );
    assert!(!picker.loading);
}

#[test]
fn snapshots_replace_results_and_truncated_stream_reports_failure() {
    let mut state = PickerState {
        query: "Qwen".into(),
        ..Default::default()
    };
    let id = state.begin_list();
    assert!(state
        .finish_list(id, Ok(page(&["Qwen3-8B"], false)))
        .is_none());
    assert!(state.loading && state.models.len() == 1);
    assert!(state
        .finish_list(id, Ok(page(&["Qwen3-8B", "Qwen3-4B"], true)))
        .is_none());
    assert!(!state.loading);
    assert_eq!(state.models.len(), 2);
    assert!(state.finish_list(id, Err("流中断".into())).is_none());
    assert_eq!(state.models.len(), 2);
    assert_eq!(state.feedback.as_deref(), Some("流中断"));
    state.begin_list();
    assert!(state.loading && state.models.is_empty());
}
#[test]
fn switching_model_invalidates_late_status_without_committing() {
    let mut picker = PickerState::default();
    let first = picker.begin_status(model("Qwen3-8B"));
    let second = picker.begin_status(model("Qwen3-4B"));
    picker.finish_status(
        first,
        Ok(inspection(
            version("modelscope", "master"),
            VersionStatus::Available,
        )),
    );
    assert_eq!(picker.status, None);
    picker.finish_status(
        second,
        Ok(inspection(
            version("hf-mirror", "main"),
            VersionStatus::Downloadable,
        )),
    );
    assert_eq!(picker.status, Some(VersionStatus::Downloadable));
    picker.begin_list();
    picker.finish_status(
        second,
        Ok(inspection(
            version("modelscope", "master"),
            VersionStatus::Available,
        )),
    );
    assert!(picker.status.is_none() && picker.preview.is_none());
    assert!(ChatState::default().selected_model_id().is_none());
}
#[test]
fn source_replacement_invalidates_old_status_but_keeps_row_identity() {
    let mut picker = PickerState::default();
    let list = picker.begin_list();
    let mut mirror = model("Qwen3-8B");
    mirror.source = "hf-mirror".into();
    mirror.revision = "main".into();
    let status = picker.begin_status(mirror.clone());
    picker.finish_status(
        status,
        Ok(inspection(
            version("hf-mirror", "main"),
            VersionStatus::Downloadable,
        )),
    );
    let replacement = picker
        .finish_list(list, Ok(page(&["Qwen3-8B"], false)))
        .expect("重新查询高优先级资源");
    assert_eq!(replacement.id, mirror.id);
    assert_eq!(replacement.source, "modelscope");
    assert!(picker.status.is_none() && picker.preview_version.is_none());
    let new_status = picker.begin_status(replacement);
    picker.finish_status(
        status,
        Ok(inspection(
            version("hf-mirror", "main"),
            VersionStatus::Available,
        )),
    );
    assert!(picker.status.is_none());
    picker.finish_status(
        new_status,
        Ok(inspection(
            version("modelscope", "master"),
            VersionStatus::Downloadable,
        )),
    );
    assert!(picker
        .finish_list(list, Ok(page(&["Qwen3-8B"], true)))
        .is_none());
    assert_eq!(picker.status, Some(VersionStatus::Downloadable));
    assert_eq!(picker.preview.as_ref().expect("选中行").id, mirror.id);
}

#[test]
fn download_starts_detection_once_preserves_draft_and_stops_before_artifacts() {
    let mut chat = ChatState::default();
    chat.set_draft("保留草稿".into());
    let run = chat
        .accept_catalog(selection(), VersionStatus::Downloadable, Vec::new())
        .expect("启动检测");
    assert_eq!(
        chat.preparation().expect("卡片").stage,
        PreparationStage::Detecting
    );
    assert!(chat
        .accept_catalog(selection(), VersionStatus::Downloadable, Vec::new())
        .is_none());
    chat.close_menus();
    chat.finish_hardware(run, Ok(Hardware::default()));
    let card = chat.preparation().expect("卡片");
    assert_eq!(card.stage, PreparationStage::Failed);
    assert_eq!(
        card.error.as_ref().expect("缺失能力").kind,
        ErrorKind::NotConnected
    );
    assert!(card
        .error
        .as_ref()
        .expect("原因")
        .reason
        .contains("工件解析"));
    assert!(card.model.variants.is_empty());
    assert!(card.local_path.is_none());
    assert!(card.actual_digest.is_none());
    assert_eq!(chat.draft(), "保留草稿");
    assert!(!chat.can_send());
    assert!(chat.send_message().is_none());
    assert!(chat.initialization_request(run).is_none());
    assert!(chat.start_preparation(run).is_none());
    assert!(!chat.is_model_ready(&selection().id()));
}
#[test]
fn remote_available_without_credentials_never_enables_generation() {
    let mut chat = ChatState::default();
    chat.set_draft("问题".into());
    assert!(chat
        .accept_catalog(selection(), VersionStatus::Available, Vec::new())
        .is_none());
    assert_eq!(chat.selected_model_id(), Some(selection().id().as_str()));
    let card = chat.preparation().expect("状态卡片");
    assert!(card
        .error
        .as_ref()
        .expect("未接入")
        .reason
        .contains("运行后端"));
    assert!(!chat.can_send());
    assert!(chat.begin_generation().is_none());
    assert_eq!(chat.draft(), "问题");
}
#[test]
fn unavailable_selection_is_rejected_and_conversation_switch_keeps_snapshot() {
    let mut chat = ChatState::default();
    for status in [VersionStatus::Developing, VersionStatus::Preparing] {
        assert!(chat
            .accept_catalog(selection(), status, Vec::new())
            .is_none());
        assert!(chat.selected_model_id().is_none());
    }
    let first = chat.active_conversation().expect("会话").id.clone();
    chat.accept_catalog(selection(), VersionStatus::Available, Vec::new());
    chat.new_conversation();
    assert_eq!(chat.selected_model_id(), Some(selection().id().as_str()));
    assert!(chat.select_conversation(&first));
    assert_eq!(
        chat.selected_model().expect("恢复模型").catalog_selection,
        Some(selection())
    );
}

#[test]
fn catalog_variants_download_verify_then_enter_artifact_ready() {
    use super::super::model::{Gpu, ModelVariant, PreparationEvent, PreparedModel, GIB};
    let digest = format!("sha256:{}", "c".repeat(64));
    let variant = ModelVariant {
        id: "q4".into(),
        quantization: "Q4_K_M".into(),
        runtime: "llama.cpp".into(),
        artifact_source: "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/q4.gguf".into(),
        expected_sha256: digest.clone(),
        ram_bytes: 1,
        vram_bytes: 1,
        disk_bytes: 10,
        file_size_bytes: None,
    };
    let mut chat = ChatState::default();
    chat.set_draft("问题".into());
    let run = chat
        .accept_catalog(selection(), VersionStatus::Downloadable, vec![variant])
        .expect("启动检测");
    chat.finish_hardware(
        run,
        Ok(Hardware {
            ram_bytes: Some(GIB),
            disk_bytes: Some(GIB),
            gpus: Some(vec![Gpu {
                name: "gpu".into(),
                available_bytes: Some(GIB),
            }]),
            ..Default::default()
        }),
    );
    assert_eq!(
        chat.preparation().expect("卡片").stage,
        PreparationStage::SelectingVariant
    );
    assert!(chat.choose_variant(run, "q4"));
    let request = chat.initialization_request(run).expect("下载快照");
    chat.preparation_event(
        run,
        PreparationEvent::DownloadProgress {
            completed: 0,
            total: None,
        },
    );
    chat.preparation_event(run, PreparationEvent::Downloaded);
    chat.preparation_event(
        run,
        PreparationEvent::Verified {
            expected: digest.clone(),
            actual: digest.clone(),
            path: "q4.gguf".into(),
        },
    );
    assert_eq!(
        chat.preparation().expect("卡片").stage,
        PreparationStage::ArtifactReady
    );

    let prepared = PreparedModel {
        model: request.model,
        variant: request.variant,
        actual_sha256: digest,
        local_path: "q4.gguf".into(),
    };
    assert!(chat.finish_preparation(run, Ok(prepared)).is_none());
    let card = chat.preparation().expect("下载完成卡片");
    assert_eq!(card.stage, PreparationStage::ArtifactReady);
    assert_eq!(card.local_path.as_deref(), Some("q4.gguf"));
    assert_eq!(
        card.actual_digest.as_deref(),
        Some(card.model.variants[0].expected_sha256.as_str())
    );
    assert!(!chat.is_model_ready(&selection().id()));
    assert!(!chat.can_send());
    assert_eq!(chat.draft(), "问题");
}
