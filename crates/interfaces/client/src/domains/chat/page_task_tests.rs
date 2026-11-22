//! 控制旧任务收尾时序，验证真实页面任务句柄的运行编号保护。
use super::super::{
    backend::{BackendFuture, Callback, LocalBackend, LOCAL_DIGEST},
    catalog::{CatalogSelection, CatalogVersion, VersionStatus},
    model::{
        ChatError, ChatModel, ErrorKind, GenerationEvent, GenerationRequest, Hardware,
        HardwareEvent, InferenceMetrics, InitializationRequest, ModelResolution, ModelVariant,
        Operation, Platform, PreparationEvent, PreparedModel, GIB,
    },
};
use super::*;
use std::cell::{Cell, RefCell};
use tokio::sync::oneshot;

type CapturedTask = Rc<RefCell<Option<Signal<Option<(u64, Task)>>>>>;

#[derive(Clone)]
struct Harness {
    initial: Rc<RefCell<Option<ChatState>>>,
    backend: Rc<DelayedBackend>,
    state: Rc<RefCell<Option<Signal<ChatState>>>>,
    task: CapturedTask,
    scope: Rc<Cell<Option<ScopeId>>>,
    old_id: u64,
    started: Rc<Cell<bool>>,
}

struct DelayedBackend {
    receiver: RefCell<Option<oneshot::Receiver<Result<(), ChatError>>>>,
    returned: Rc<Cell<bool>>,
}
impl ChatBackend for DelayedBackend {
    fn search(&self, keyword: String) -> BackendFuture<Vec<ChatModel>> {
        LocalBackend.search(keyword)
    }
    fn model_status(&self, id: String) -> BackendFuture<Vec<PreparedModel>> {
        LocalBackend.model_status(id)
    }
    fn detect_hardware(&self, callback: Callback<HardwareEvent>) -> BackendFuture<Hardware> {
        LocalBackend.detect_hardware(callback)
    }
    fn initialize(
        &self,
        request: InitializationRequest,
        mut callback: Callback<PreparationEvent>,
    ) -> BackendFuture<PreparedModel> {
        Box::pin(async move {
            callback(PreparationEvent::Downloaded);
            callback(PreparationEvent::Verified {
                expected: LOCAL_DIGEST.into(),
                actual: LOCAL_DIGEST.into(),
                path: "model.gguf".into(),
            });
            Ok(PreparedModel {
                model: request.model,
                variant: request.variant,
                actual_sha256: LOCAL_DIGEST.into(),
                local_path: "model.gguf".into(),
            })
        })
    }
    fn deploy_model(&self, _: String) -> BackendFuture<()> {
        let receiver = self.receiver.borrow_mut().take().expect("一次部署");
        let returned = self.returned.clone();
        Box::pin(async move {
            let result = receiver.await.expect("测试发送部署结果");
            returned.set(true);
            result
        })
    }
    fn generate(
        &self,
        _: GenerationRequest,
        _: Callback<GenerationEvent>,
    ) -> BackendFuture<Option<InferenceMetrics>> {
        Box::pin(async { Ok(None) })
    }
}

#[component]
fn TaskHarness() -> Element {
    let harness = consume_context::<Harness>();
    let initial = harness.initial.borrow_mut().take().expect("初始状态");
    let state = use_signal(|| initial);
    let task = use_signal(|| None::<(u64, Task)>);
    let scope = Runtime::current().current_scope_id();
    *harness.state.borrow_mut() = Some(state);
    *harness.task.borrow_mut() = Some(task);
    harness.scope.set(Some(scope));
    use_context_provider(|| BackendHandle(harness.backend.clone()));
    use_context_provider(|| PreparationTask(task));
    use_context_provider(|| ChatScope(scope));
    use_effect(move || {
        if !harness.started.replace(true) {
            spawn_preparation(state, harness.old_id);
        }
    });
    rsx! { div { "任务测试" } }
}

fn downloading() -> (ChatState, u64, CatalogSelection, ModelVariant, Hardware) {
    let selection = CatalogSelection {
        name: "模型".into(),
        preset: None,
        version: CatalogVersion {
            verified_variants: Vec::new(),
            name: "版本".into(),
            model: "test/model".into(),
            source: "modelscope".into(),
            revision: "master".into(),
            resource_url: "https://modelscope.cn/models/test/model/files?Revision=master".into(),
            variants: Vec::new(),
        },
    };
    let variant = ModelVariant {
        id: "q4".into(),
        quantization: "q4".into(),
        runtime: "llama.cpp".into(),
        artifact_source: "https://example.invalid/model.gguf".into(),
        expected_sha256: LOCAL_DIGEST.into(),
        ram_bytes: GIB,
        vram_bytes: 0,
        disk_bytes: GIB,
        file_size_bytes: None,
    };
    let hardware = Hardware {
        platform: Some(Platform::Windows),
        cpu: Some("测试".into()),
        gpus: Some(vec![]),
        ram_bytes: Some(8 * GIB),
        disk_bytes: Some(8 * GIB),
    };
    let mut state = ChatState::default();
    let id = state
        .accept_catalog(selection.clone(), VersionStatus::Downloadable, Vec::new())
        .expect("启动");
    state.finish_model_resolution(
        id,
        Ok(ModelResolution {
            hardware: hardware.clone(),
            variants: vec![variant.clone()],
            reason: None,
        }),
    );
    assert!(state.choose_variant(id, "q4"));
    (state, id, selection, variant, hardware)
}

#[tokio::test]
async fn stale_task_cleanup_cannot_clear_new_run_handle() {
    for outcome in [
        Ok(()),
        Err(ChatError {
            kind: ErrorKind::Timeout,
            operation: Operation::Initialize(PreparationStage::Deploying),
            reason: "旧任务错误".into(),
        }),
    ] {
        let (initial, old_id, selection, variant, hardware) = downloading();
        let (sender, receiver) = oneshot::channel();
        let returned = Rc::new(Cell::new(false));
        let harness = Harness {
            initial: Rc::new(RefCell::new(Some(initial))),
            backend: Rc::new(DelayedBackend {
                receiver: RefCell::new(Some(receiver)),
                returned: returned.clone(),
            }),
            state: Rc::new(RefCell::new(None)),
            task: Rc::new(RefCell::new(None)),
            scope: Rc::new(Cell::new(None)),
            old_id,
            started: Rc::new(Cell::new(false)),
        };
        let mut dom = VirtualDom::new(TaskHarness).with_root_context(harness.clone());
        dom.rebuild_in_place();
        for _ in 0..20 {
            dom.render_immediate_to_vec();
            if harness.backend.receiver.borrow().is_none() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(
            harness.backend.receiver.borrow().is_none(),
            "旧任务必须等待部署结果"
        );
        let mut state = harness.state.borrow().expect("状态信号");
        let mut task = harness.task.borrow().expect("任务信号");
        let scope = harness.scope.get().expect("页面作用域");
        let new_id = dom.in_runtime(|| {
            state.with_mut(|state| {
                assert_eq!(state.request_preparation_cancel(old_id), None);
                state.set_draft("新草稿".into());
                let new_id = state
                    .accept_catalog(
                        selection.clone(),
                        VersionStatus::Downloadable,
                        vec![variant.clone()],
                    )
                    .expect("重启");
                state.finish_model_resolution(
                    new_id,
                    Ok(ModelResolution {
                        hardware: hardware.clone(),
                        variants: vec![variant.clone()],
                        reason: None,
                    }),
                );
                assert!(state.choose_variant(new_id, "q4"));
                new_id
            })
        });
        assert_ne!(new_id, old_id);
        dom.in_runtime(|| {
            // 保留旧 future 不取消，以强制复现旧完成晚于新句柄登记的竞态。
            let _old_handle = task.take().expect("旧句柄");
            let new_handle =
                Runtime::current().spawn(scope, async { std::future::pending::<()>().await });
            task.set(Some((new_id, new_handle)));
        });
        let before = dom.in_runtime(|| state.read().clone());
        sender.send(outcome).expect("唤醒旧任务");
        for _ in 0..20 {
            dom.render_immediate_to_vec();
            if returned.get() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(returned.get(), "旧任务必须实际返回并执行收尾");
        dom.in_runtime(|| {
            assert_eq!(*state.read(), before);
            assert_eq!(state.read().draft(), "新草稿");
            assert_eq!(task.peek().as_ref().map(|(id, _)| *id), Some(new_id));
            if let Some((_, handle)) = task.take() {
                handle.cancel();
            }
        });
    }
}

#[test]
fn route_status_labels_cover_the_stable_router_contract() {
    for (status, label) in [
        ("routable", "模型已部署，可路由"),
        ("unroutable", "模型已启动，等待 Router 准入"),
        ("saturated", "模型运行中，当前容量已满"),
        ("quarantined", "模型暂不可路由，需要重新挂载或恢复"),
        ("lost", "模型暂不可路由，需要重新挂载或恢复"),
        ("detaching", "模型正在停止新请求并清理挂载"),
        ("detached", "模型已完成摘除"),
        ("failed", "模型部署或运行时失败"),
    ] {
        assert_eq!(super::route_status_label(status), label);
    }
    assert_eq!(super::route_status_label("unknown"), "Router 状态未知");
}
