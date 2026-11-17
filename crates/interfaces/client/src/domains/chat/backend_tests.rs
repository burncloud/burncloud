//! 可控后端、异步边界与真实组件事件的确定性回归，不访问网络或等待计时。
use super::super::{
    backend::{BackendFuture, Callback},
    model::{GenerationRequest, InferenceMetrics, InitializationRequest},
    page::BackendHandle,
};
use super::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn backend_error(kind: ErrorKind, operation: Operation) -> ChatError {
    ChatError {
        kind,
        operation,
        reason: "受控后端失败".into(),
    }
}

#[test]
fn owned_model_names_and_unknown_dates_are_not_replaced_with_ids() {
    let mut state = ChatState::default();
    let mut models = catalog();
    models[0].name = "显示名称不同于 ID".into();
    models[0].updated_at = None;
    let id = models[0].id.clone();
    state.set_test_models(models);
    let selection = state.begin_selection(&id).expect("选择");
    state.finish_selection(selection, Ok(Vec::new()));
    assert_eq!(
        state.selected_model().expect("已选模型").name,
        "显示名称不同于 ID"
    );
    assert_eq!(state.selected_model_id(), Some(id.as_str()));
    assert_eq!(state.selected_model().expect("已选模型").updated_at, None);
}

#[test]
fn selection_queries_are_numbered_block_send_and_expose_failures() {
    let mut state = chat();
    state.set_draft("保留草稿".into());
    let old = state.begin_selection(MODEL).expect("选择");
    assert!(!state.can_send());
    assert!(state.send_message().is_none());
    let current = state.begin_selection(READY_MODEL).expect("重新选择");
    state.finish_selection(
        old,
        Err(backend_error(ErrorKind::Timeout, Operation::Status)),
    );
    assert!(state.status_loading());
    state.finish_selection(
        current,
        Err(backend_error(ErrorKind::ExecutionFailed, Operation::Status)),
    );
    assert!(state.status_error().is_some());
    assert!(!state.can_send());
    assert_eq!(state.selected_model_id(), Some(READY_MODEL));
    assert!(select(&mut state, READY_MODEL));
    assert!(state.can_send());
    assert_eq!(state.draft(), "保留草稿");
}

#[test]
fn status_rejects_mismatched_models_and_unverified_ready_versions() {
    for corruption in 0..4 {
        let mut state = chat();
        let request = state.begin_selection(MODEL).expect("选择");
        let mut ready = immediate(LocalBackend.model_status(READY_MODEL.into()))
            .expect("预置就绪")
            .remove(0);
        match corruption {
            0 => {}
            1 => ready.local_path.clear(),
            2 => ready.actual_sha256.clear(),
            _ => ready.variant.id = "other-version".into(),
        }
        state.finish_selection(request, Ok(vec![ready]));
        assert!(!state.is_model_ready(MODEL));
        assert!(state.status_error().is_some());
    }
}

#[test]
fn unknown_resources_and_multiple_gpus_never_add_vram_together() {
    let model = catalog().into_iter().find(|m| m.id == MODEL).expect("模型");
    let variant = &model.variants[1];
    let mut detected = hardware(DemoPlatform::Windows);
    detected.gpus = Some(vec![
        Gpu {
            name: "A".into(),
            available_bytes: Some(4 * GIB),
        },
        Gpu {
            name: "B".into(),
            available_bytes: Some(4 * GIB),
        },
    ]);
    assert_eq!(detected.max_vram(), Some(4 * GIB));
    assert!(!variant.fits(&detected));
    for unknown in 0..3 {
        let mut detected = hardware(DemoPlatform::Windows);
        match unknown {
            0 => detected.ram_bytes = None,
            1 => detected.gpus = None,
            _ => detected.disk_bytes = None,
        }
        assert!(!variant.fits(&detected));
        assert!(variant.shortfall(&detected).contains("未知"));
    }
}

#[test]
fn contradictory_hardware_callbacks_and_final_snapshot_fail_closed() {
    for final_result in [false, true] {
        let mut state = selected_chat();
        let id = send_pending(&mut state);
        state.hardware_event(id, HardwareEvent::Memory(Some(1)));
        state.hardware_event(id, HardwareEvent::Memory(Some(1)));
        assert_eq!(state.preparation().expect("流程").hardware_events.len(), 1);
        if final_result {
            state.finish_hardware(id, Ok(hardware(DemoPlatform::Windows)));
        } else {
            state.hardware_event(id, HardwareEvent::Memory(Some(2)));
        }
        assert_eq!(
            state.preparation().expect("流程").stage,
            PreparationStage::Failed
        );
        assert_eq!(state.draft(), "请介绍这个模型");
    }
}

#[test]
fn byte_progress_unknown_totals_duplicates_and_invalid_progress_are_explicit() {
    for invalid in [
        PreparationEvent::DownloadProgress {
            completed: 4,
            total: None,
        },
        PreparationEvent::DownloadProgress {
            completed: 20,
            total: Some(10),
        },
        PreparationEvent::DownloadProgress {
            completed: 5,
            total: Some(0),
        },
    ] {
        let mut state = selected_chat();
        let id = start(&mut state);
        reach(&mut state, id, PreparationStage::Downloading);
        let progress = PreparationEvent::DownloadProgress {
            completed: 5,
            total: None,
        };
        state.preparation_event(id, progress.clone());
        state.preparation_event(id, progress);
        assert_eq!(state.preparation().expect("流程").progress(), None);
        state.preparation_event(id, invalid);
        assert_eq!(
            state.preparation().expect("流程").stage,
            PreparationStage::Failed
        );
    }
}

#[test]
fn incomplete_download_and_changed_totals_are_not_verification_success() {
    for event in [
        PreparationEvent::Downloaded,
        PreparationEvent::DownloadProgress {
            completed: 6,
            total: Some(11),
        },
    ] {
        let mut state = selected_chat();
        let id = start(&mut state);
        reach(&mut state, id, PreparationStage::Downloading);
        state.preparation_event(
            id,
            PreparationEvent::DownloadProgress {
                completed: 5,
                total: Some(10),
            },
        );
        state.preparation_event(id, event);
        assert_eq!(
            state.preparation().expect("流程").stage,
            PreparationStage::Failed
        );
    }
}

#[test]
fn missing_digest_wrong_expected_digest_and_empty_paths_cannot_mark_ready() {
    for (expected, actual, path) in [
        ("", "", "file"),
        (DEMO_DIGEST, "", "file"),
        (
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "file",
        ),
        (DEMO_DIGEST, DEMO_DIGEST, "  "),
    ] {
        let mut state = selected_chat();
        let id = send_pending(&mut state);
        reach(&mut state, id, PreparationStage::Verifying);
        state.preparation_event(
            id,
            PreparationEvent::Verified {
                expected: expected.into(),
                actual: actual.into(),
                path: path.into(),
            },
        );
        assert_eq!(
            state.preparation().expect("流程").stage,
            PreparationStage::Failed
        );
        assert!(state.preparation().expect("流程").local_path.is_none());
        assert!(!state.is_model_ready(MODEL));
        assert_eq!(state.draft(), "请介绍这个模型");
    }
}

#[test]
fn early_success_skipped_stages_and_mismatched_final_results_fail() {
    for corruption in 0..8 {
        let mut state = selected_chat();
        let id = send_pending(&mut state);
        reach(&mut state, id, PreparationStage::Downloading);
        let mut ready = prepared(&state);
        match corruption {
            0 => {}
            1 => state.preparation_event(id, PreparationEvent::Deployed),
            _ => {
                reach(&mut state, id, PreparationStage::Reporting);
                if corruption != 2 {
                    state.preparation_event(id, PreparationEvent::Reported);
                }
                match corruption {
                    3 => ready.variant.id = "wrong".into(),
                    4 => ready.model.id = "wrong".into(),
                    5 => ready.local_path = "wrong.gguf".into(),
                    6 => ready.actual_sha256.clear(),
                    _ => ready.variant.runtime = "wrong".into(),
                }
            }
        }
        assert!(state.finish_preparation(id, Ok(ready)).is_none());
        assert_eq!(
            state.preparation().expect("流程").stage,
            PreparationStage::Failed
        );
        assert!(!state.is_model_ready(MODEL));
        assert_eq!(state.draft(), "请介绍这个模型");
    }
}

#[test]
fn duplicate_success_and_stale_final_returns_resume_only_once() {
    let mut state = selected_chat();
    let id = send_pending(&mut state);
    reach(&mut state, id, PreparationStage::Reporting);
    let ready = prepared(&state);
    state.preparation_event(id, PreparationEvent::Reported);
    state.preparation_event(id, PreparationEvent::Reported);
    let plan = state
        .finish_preparation(id, Ok(ready.clone()))
        .expect("续聊一次");
    assert!(state.finish_preparation(id, Ok(ready)).is_none());
    assert!(state
        .finish_preparation(
            id,
            Err(backend_error(
                ErrorKind::Timeout,
                Operation::Initialize(PreparationStage::Reporting)
            ))
        )
        .is_none());
    assert_eq!(plan.request.messages.len(), 1);
    assert_eq!(plan.request.messages[0].content, "请介绍这个模型");
}

#[test]
fn generation_receives_real_history_effort_chunks_optional_metrics_and_errors() {
    let mut state = chat();
    assert!(select(&mut state, READY_MODEL));
    state.set_draft("第一问".into());
    let first = state.begin_generation().expect("首轮");
    state.generation_event(&first, GenerationEvent::Thinking("思考一".into()));
    state.generation_event(&first, GenerationEvent::Thinking("思考二".into()));
    chunk(&mut state, &first, "回答一");
    chunk(&mut state, &first, "回答二");
    state.complete_generation(&first, Ok(None));
    assert!(state.active_conversation().expect("会话").messages[1]
        .metrics
        .is_none());
    assert_eq!(
        state.active_conversation().expect("会话").messages[1]
            .thinking
            .as_deref(),
        Some("思考一思考二")
    );
    state.set_reasoning_effort(ReasoningEffort::ExtraHigh);
    state.set_draft("第二问".into());
    let second = state.begin_generation().expect("次轮");
    assert_eq!(second.request.reasoning_effort, ReasoningEffort::ExtraHigh);
    assert_eq!(
        second
            .request
            .messages
            .iter()
            .map(|m| (m.role, m.content.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (MessageRole::User, "第一问"),
            (MessageRole::Assistant, "回答一回答二"),
            (MessageRole::User, "第二问")
        ]
    );
    chunk(&mut state, &second, "部分回答");
    state.complete_generation(
        &second,
        Err(backend_error(ErrorKind::Timeout, Operation::Generate)),
    );
    let messages = &state.active_conversation().expect("会话").messages;
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[3].content, "部分回答");
    assert!(messages[3].metrics.is_none());
    assert_eq!(
        messages[3].error.as_ref().expect("原消息错误").kind,
        ErrorKind::Timeout
    );
    assert!(!chunk(&mut state, &second, "迟到内容"));
}

#[test]
fn local_backend_rejects_external_invalid_requests_without_waiting() {
    assert_eq!(
        immediate(LocalBackend.model_status(" ".into()))
            .expect_err("空 ID")
            .kind,
        ErrorKind::InvalidParameter
    );
    assert_eq!(
        immediate(LocalBackend.model_status("missing".into()))
            .expect_err("未知 ID")
            .kind,
        ErrorKind::NotFound
    );
    let mut state = selected_chat();
    let id = start(&mut state);
    reach(&mut state, id, PreparationStage::Downloading);
    let mut request = state.initialization_request(id).expect("输入快照");
    request.run_id = 0;
    assert_eq!(
        immediate(LocalBackend.initialize(request, Box::new(|_| panic!("无效参数不得回调"))))
            .expect_err("参数错误")
            .kind,
        ErrorKind::InvalidParameter
    );
}

#[derive(Default)]
struct ControlledBackend {
    calls: RefCell<Vec<String>>,
    requests: RefCell<Vec<GenerationRequest>>,
    hold_generation: bool,
    generation_callback: Rc<RefCell<Option<Callback<GenerationEvent>>>>,
    generation_dropped: Rc<Cell<bool>>,
    status_gate: RefCell<Option<tokio::sync::oneshot::Receiver<()>>>,
    fail_search: Cell<bool>,
    fail_initialization: Cell<bool>,
    fail_generation: bool,
    invalid_pending_initialization: bool,
    initialization_dropped: Rc<Cell<bool>>,
}
struct DropNotice(Rc<Cell<bool>>);
impl Drop for DropNotice {
    fn drop(&mut self) {
        self.0.set(true);
    }
}
impl ChatBackend for ControlledBackend {
    fn search(&self, keyword: String) -> BackendFuture<Vec<ChatModel>> {
        self.calls.borrow_mut().push(format!("search:{keyword}"));
        let fail = self.fail_search.replace(false);
        Box::pin(async move {
            if fail {
                Err(backend_error(ErrorKind::Timeout, Operation::Search))
            } else {
                Ok(catalog().into_iter().filter(|m| m.id == MODEL).collect())
            }
        })
    }
    fn model_status(&self, model: String) -> BackendFuture<Vec<PreparedModel>> {
        self.calls.borrow_mut().push(format!("status:{model}"));
        let gate = self.status_gate.borrow_mut().take();
        Box::pin(async move {
            if let Some(gate) = gate {
                gate.await.expect("测试释放状态查询");
            }
            Ok(Vec::new())
        })
    }
    fn detect_hardware(
        &self,
        mut callback: Callback<HardwareEvent>,
    ) -> BackendFuture<DemoHardware> {
        self.calls.borrow_mut().push("hardware".into());
        Box::pin(async move {
            let hardware = hardware(DemoPlatform::Windows);
            callback(HardwareEvent::Cpu(hardware.cpu.clone().expect("CPU")));
            callback(HardwareEvent::Gpus(hardware.gpus.clone().expect("GPU")));
            callback(HardwareEvent::Memory(hardware.ram_bytes));
            callback(HardwareEvent::Disk(hardware.disk_bytes));
            Ok(hardware)
        })
    }
    fn initialize(
        &self,
        request: InitializationRequest,
        mut callback: Callback<PreparationEvent>,
    ) -> BackendFuture<PreparedModel> {
        self.calls
            .borrow_mut()
            .push(format!("initialize:{}", request.run_id));
        let fail = self.fail_initialization.get();
        let invalid_pending = self.invalid_pending_initialization;
        let notice = DropNotice(self.initialization_dropped.clone());
        Box::pin(async move {
            let _notice = notice;
            if invalid_pending {
                callback(PreparationEvent::Deployed);
                std::future::pending::<()>().await;
            }
            callback(PreparationEvent::DownloadProgress {
                completed: 5,
                total: None,
            });
            if fail {
                return Err(backend_error(
                    ErrorKind::ExecutionFailed,
                    Operation::Initialize(PreparationStage::Downloading),
                ));
            }
            callback(PreparationEvent::Downloaded);
            callback(PreparationEvent::Verified {
                expected: request.variant.expected_sha256.clone(),
                actual: DEMO_DIGEST.into(),
                path: "controlled.gguf".into(),
            });
            for event in [
                PreparationEvent::PackageLoaded,
                PreparationEvent::Deployed,
                PreparationEvent::Healthy,
                PreparationEvent::Reported,
            ] {
                callback(event);
            }
            Ok(PreparedModel {
                model: request.model,
                variant: request.variant,
                actual_sha256: DEMO_DIGEST.into(),
                local_path: "controlled.gguf".into(),
            })
        })
    }
    fn generate(
        &self,
        request: GenerationRequest,
        mut callback: Callback<GenerationEvent>,
    ) -> BackendFuture<Option<InferenceMetrics>> {
        self.calls.borrow_mut().push("generate".into());
        self.requests.borrow_mut().push(request);
        callback(GenerationEvent::Content("接口分段一".into()));
        callback(GenerationEvent::Thinking("接口思考".into()));
        callback(GenerationEvent::Content("接口分段二".into()));
        self.generation_callback.borrow_mut().replace(callback);
        let hold = self.hold_generation;
        let fail = self.fail_generation;
        let notice = DropNotice(self.generation_dropped.clone());
        Box::pin(async move {
            let _notice = notice;
            if hold {
                std::future::pending::<()>().await;
            }
            if fail {
                Err(backend_error(ErrorKind::Timeout, Operation::Generate))
            } else {
                Ok(None)
            }
        })
    }
}

#[path = "ui_tests.rs"]
mod ui_tests;

#[test]
fn changed_artifact_identity_invalidates_only_that_version_and_generation_uses_current_one() {
    let mut state = chat();
    let mut model = catalog().into_iter().find(|m| m.id == MODEL).expect("模型");
    let selection = state.begin_selection(MODEL).expect("选择");
    let ready: Vec<_> = model
        .variants
        .iter()
        .map(|variant| PreparedModel {
            model: model.clone(),
            variant: variant.clone(),
            actual_sha256: DEMO_DIGEST.into(),
            local_path: "valid.gguf".into(),
        })
        .collect();
    state.finish_selection(selection, Ok(ready));
    model.variants[0].expected_sha256 =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
    state.set_test_models(vec![model.clone()]);
    assert!(state.is_model_ready(MODEL));
    state.set_draft("不得使用旧摘要版本".into());
    let plan = state.begin_generation().expect("另一版本仍可用");
    assert_eq!(plan.request.prepared.variant.id, "q8");
    state.interrupt_generation();
    model.variants[1].expected_sha256 = model.variants[0].expected_sha256.clone();
    state.set_test_models(vec![model]);
    assert!(!state.is_model_ready(MODEL));
}

#[test]
fn local_generation_rejects_empty_history_system_cards_and_unsupported_runtime() {
    let mut state = chat();
    assert!(select(&mut state, READY_MODEL));
    state.set_draft("问题".into());
    let plan = state.begin_generation().expect("生成");
    for invalid in 0..4 {
        let mut request = *plan.request.clone();
        match invalid {
            0 => request.messages.clear(),
            1 => request.messages[0].role = MessageRole::System,
            2 => request.messages[0].content.clear(),
            _ => {
                request.prepared.variant.runtime = "unsupported".into();
                request.prepared.model.variants[0].runtime = "unsupported".into();
            }
        }
        let result =
            immediate(LocalBackend.generate(request, Box::new(|_| panic!("非法请求不能生成增量"))));
        assert!(result.is_err());
    }
}

#[test]
fn reselecting_changed_model_snapshot_creates_a_new_preparation_card() {
    let mut state = selected_chat();
    let id = start(&mut state);
    complete(&mut state, id);
    let mut model = state.selected_model().expect("模型");
    model.variants[0].expected_sha256 =
        "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
    state.set_test_models(vec![model.clone()]);
    assert!(!state.is_model_ready(MODEL));
    let selection = state.begin_selection(MODEL).expect("重新选择");
    state.finish_selection(selection, Ok(Vec::new()));
    assert_eq!(
        state.preparation().expect("新卡片").stage,
        PreparationStage::AwaitingStart
    );
    assert_eq!(state.preparation().expect("新卡片").model, model);
    assert!(state
        .start_preparation(state.preparation().expect("新卡片").run_id)
        .is_some());
}
