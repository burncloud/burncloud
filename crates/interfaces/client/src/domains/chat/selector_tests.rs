//! 目录浏览竞态与聊天流程边界的独立回归测试。
use super::super::{
    catalog::CatalogSelection,
    model::{ErrorKind, Hardware, PreparationStage},
};
use super::*;
use serde_json::json;

fn version(source: &str, revision: &str) -> CatalogVersion {
    CatalogVersion {
        name: "Qwen3-8B".into(),
        model: "Qwen/Qwen3-8B".into(),
        revision: revision.into(),
        source: source.into(),
        resource_url: "https://modelscope.cn/models/Qwen/Qwen3-8B/files?Revision=master".into(),
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
    }
}
fn page(names: &[&str], done: bool) -> CatalogPage<CatalogModel> {
    serde_json::from_value(json!({"type": if done {Some("models")} else {None},
        "items": names.iter().map(|name| json!({"id":name.to_lowercase(),"name":name,"model":format!("Qwen/{name}"),"source":"modelscope","revision":"master"})).collect::<Vec<_>>(),"done":done})).expect("测试列表")
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
        Ok((
            Some(version("modelscope", "master")),
            VersionStatus::Available,
        )),
    );
    assert_eq!(picker.status, None);
    picker.finish_status(
        second,
        Ok((
            Some(version("hf-mirror", "main")),
            VersionStatus::Downloadable,
        )),
    );
    assert_eq!(picker.status, Some(VersionStatus::Downloadable));
    picker.begin_list();
    picker.finish_status(second, Ok((None, VersionStatus::Available)));
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
        Ok((
            Some(version("hf-mirror", "main")),
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
        Ok((Some(version("hf-mirror", "main")), VersionStatus::Available)),
    );
    assert!(picker.status.is_none());
    picker.finish_status(
        new_status,
        Ok((
            Some(version("modelscope", "master")),
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
fn catalog_variants_download_verify_then_stop_without_ready() {
    use super::super::{
        catalog::deployment_missing,
        model::{Gpu, ModelVariant, PreparationEvent, GIB},
    };
    let digest = format!("sha256:{}", "c".repeat(64));
    let variant = ModelVariant {
        id: "q4".into(),
        quantization: "Q4_K_M".into(),
        runtime: "llama.cpp".into(),
        artifact_source: "https://example.com/q4.gguf".into(),
        expected_sha256: digest.clone(),
        ram_bytes: 1,
        vram_bytes: 1,
        disk_bytes: 10,
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
    assert!(chat.initialization_request(run).is_some());
    chat.preparation_event(
        run,
        PreparationEvent::DownloadProgress {
            completed: 10,
            total: Some(10),
        },
    );
    chat.preparation_event(run, PreparationEvent::Downloaded);
    chat.preparation_event(
        run,
        PreparationEvent::Verified {
            expected: digest.clone(),
            actual: digest,
            path: "q4.gguf".into(),
        },
    );
    assert_eq!(
        chat.preparation().expect("卡片").stage,
        PreparationStage::LoadingPackage
    );
    // 部署未接入：校验通过后以 NotConnected 停止，不记录就绪、不续聊。
    assert!(chat
        .finish_preparation(run, Err(deployment_missing("q4.gguf")))
        .is_none());
    let card = chat.preparation().expect("卡片");
    assert_eq!(card.stage, PreparationStage::Failed);
    let error = card.error.as_ref().expect("停止原因");
    assert_eq!(error.kind, ErrorKind::NotConnected);
    assert!(error.reason.contains("尚未接入部署"));
    assert!(!chat.is_model_ready(&selection().id()));
    assert!(!chat.can_send());
    assert_eq!(chat.draft(), "问题");
}

#[test]
fn downloaded_catalog_artifact_stops_before_deployment_and_never_becomes_ready() {
    use super::super::{
        catalog::deployment_missing,
        model::{Gpu, ModelVariant, PreparationEvent, GIB},
    };
    let digest = format!("sha256:{}", "c".repeat(64));
    let variant = ModelVariant {
        id: "q4".into(),
        quantization: "Q4_K_M".into(),
        runtime: "llama.cpp".into(),
        artifact_source: "https://example.com/q4.gguf".into(),
        expected_sha256: digest.clone(),
        ram_bytes: GIB,
        vram_bytes: GIB,
        disk_bytes: 10,
    };
    let mut chat = ChatState::default();
    chat.set_draft("问题".into());
    let run = chat
        .accept_catalog(selection(), VersionStatus::Downloadable, vec![variant])
        .expect("启动检测");
    chat.finish_hardware(
        run,
        Ok(Hardware {
            gpus: Some(vec![Gpu {
                name: "GPU".into(),
                available_bytes: Some(8 * GIB),
            }]),
            ram_bytes: Some(8 * GIB),
            disk_bytes: Some(8 * GIB),
            ..Hardware::default()
        }),
    );
    assert_eq!(
        chat.preparation().expect("卡片").stage,
        PreparationStage::SelectingVariant
    );
    assert!(chat.choose_variant(run, "q4"));
    for event in [
        PreparationEvent::DownloadProgress {
            completed: 10,
            total: Some(10),
        },
        PreparationEvent::Downloaded,
        PreparationEvent::Verified {
            expected: digest.clone(),
            actual: digest,
            path: "model.gguf".into(),
        },
    ] {
        chat.preparation_event(run, event);
    }
    assert!(chat
        .finish_preparation(run, Err(deployment_missing("model.gguf")))
        .is_none());
    let card = chat.preparation().expect("卡片");
    assert_eq!(card.stage, PreparationStage::Failed);
    let error = card.error.as_ref().expect("部署未接入");
    assert_eq!(error.kind, ErrorKind::NotConnected);
    assert!(error.reason.contains("尚未接入部署"));
    assert!(!chat.is_model_ready(&selection().id()));
    assert!(!chat.can_send());
    assert_eq!(chat.draft(), "问题");
}
