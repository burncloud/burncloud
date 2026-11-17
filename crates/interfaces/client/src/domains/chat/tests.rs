use super::{
    backend::{ChatBackend, LocalBackend, LOCAL_DIGEST as DEMO_DIGEST, SIMULATED_REPLY_SEGMENTS},
    model::{
        ChatError, ChatModel, ErrorKind, GenerationEvent, Gpu, Hardware as DemoHardware,
        HardwareEvent, MessageRole, Operation, Platform as DemoPlatform, PreparationEvent,
        PreparationStage, PreparedModel, GIB,
    },
    state::{ChatAction, ChatState, GenerationPlan, ReasoningEffort},
};
use std::{
    future::Future,
    pin::pin,
    task::{Context, Poll, Waker},
};

fn immediate<T>(future: impl Future<Output = T>) -> T {
    let waker = Waker::noop();
    match pin!(future).as_mut().poll(&mut Context::from_waker(waker)) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("此测试替身不得依赖真实等待"),
    }
}
fn catalog() -> Vec<ChatModel> {
    immediate(LocalBackend.search(String::new())).expect("本地目录")
}
fn chat() -> ChatState {
    let mut state = ChatState::default();
    state.set_test_models(catalog());
    state
}
fn select(state: &mut ChatState, id: &str) -> bool {
    let Some(request) = state.begin_selection(id) else {
        return false;
    };
    state.finish_selection(request, immediate(LocalBackend.model_status(id.into())));
    true
}
fn hardware(platform: DemoPlatform) -> DemoHardware {
    let (cpu, gpu) = match platform {
        DemoPlatform::Windows => ("x64 CPU · 8 核", "独立 GPU"),
        DemoPlatform::MacOs => ("Apple CPU · 8 核", "Apple GPU"),
        DemoPlatform::Linux => ("x64 CPU · 8 核", "Linux GPU"),
    };
    DemoHardware {
        platform: Some(platform),
        cpu: Some(cpu.into()),
        gpus: Some(vec![Gpu {
            name: gpu.into(),
            available_bytes: Some(8 * GIB),
        }]),
        ram_bytes: Some(24 * GIB),
        disk_bytes: Some(128 * GIB),
    }
}
fn detected(state: &mut ChatState, id: u64, hardware: DemoHardware) {
    for event in [
        HardwareEvent::Cpu(hardware.cpu.clone().expect("CPU")),
        HardwareEvent::Gpus(hardware.gpus.clone().expect("GPU")),
        HardwareEvent::Memory(hardware.ram_bytes),
        HardwareEvent::Disk(hardware.disk_bytes),
    ] {
        state.hardware_event(id, event);
    }
    state.finish_hardware(id, Ok(hardware));
}
fn prepared(state: &ChatState) -> PreparedModel {
    let p = state.preparation().expect("卡片");
    PreparedModel {
        model: p.model.clone(),
        variant: p.variant().expect("版本").clone(),
        actual_sha256: DEMO_DIGEST.into(),
        local_path: path(
            p.hardware.platform.unwrap_or(DemoPlatform::Windows),
            p.model.id(),
            p.variant_id.as_deref().expect("版本"),
        ),
    }
}
fn path(platform: DemoPlatform, model: &str, variant: &str) -> String {
    let name = model.replace(':', "-");
    match platform {
        DemoPlatform::Windows => format!(r"C:\BurnCloud\models\{name}-{variant}.gguf"),
        DemoPlatform::MacOs => format!("/Users/user/BurnCloud/models/{name}-{variant}.gguf"),
        DemoPlatform::Linux => format!("/home/user/BurnCloud/models/{name}-{variant}.gguf"),
    }
}
fn tick(state: &mut ChatState, id: u64) -> Option<GenerationPlan> {
    let p = state.preparation().expect("卡片").clone();
    if p.run_id != id {
        return None;
    }
    let event = match p.stage {
        PreparationStage::Detecting => {
            detected(state, id, hardware(DemoPlatform::Windows));
            return None;
        }
        PreparationStage::Downloading => {
            state.preparation_event(
                id,
                PreparationEvent::DownloadProgress {
                    completed: 100,
                    total: Some(100),
                },
            );
            PreparationEvent::Downloaded
        }
        PreparationStage::Verifying => PreparationEvent::Verified {
            expected: DEMO_DIGEST.into(),
            actual: DEMO_DIGEST.into(),
            path: prepared(state).local_path,
        },
        PreparationStage::LoadingPackage => PreparationEvent::PackageLoaded,
        PreparationStage::Deploying => PreparationEvent::Deployed,
        PreparationStage::HealthChecking => PreparationEvent::Healthy,
        PreparationStage::Reporting => {
            state.preparation_event(id, PreparationEvent::Reported);
            return state.finish_preparation(id, Ok(prepared(state)));
        }
        _ => return None,
    };
    state.preparation_event(id, event);
    None
}
fn failure(state: &mut ChatState, id: u64, reason: &str) -> Option<GenerationPlan> {
    let stage = state.preparation().expect("卡片").stage;
    let error = ChatError {
        kind: ErrorKind::ExecutionFailed,
        operation: Operation::Initialize(stage),
        reason: reason.into(),
    };
    if stage == PreparationStage::Detecting {
        state.finish_hardware(id, Err(error));
        None
    } else {
        state.finish_preparation(id, Err(error))
    }
}
fn chunk(state: &mut ChatState, plan: &GenerationPlan, content: &str) -> bool {
    state.generation_event(plan, GenerationEvent::Content(content.into()))
}

use crate::app::router::routes::Route;

const MODEL: &str = "qwen3:8b";
const READY_MODEL: &str = "llama3.2:1b";

fn selected_chat() -> ChatState {
    let mut state = chat();
    assert!(select(&mut state, MODEL));
    state
}

fn start(state: &mut ChatState) -> u64 {
    let id = state.preparation().expect("已选演示模型应有卡片").run_id;
    state.start_preparation(id).expect("等待或失败任务可以开始")
}

fn send_pending(state: &mut ChatState) -> u64 {
    state.set_draft("请介绍这个模型".into());
    match state.send_message() {
        Some(ChatAction::Prepare(id)) => id,
        _ => panic!("未就绪模型应进入初始化"),
    }
}

fn reach(state: &mut ChatState, id: u64, target: PreparationStage) {
    for _ in 0..24 {
        let stage = state.preparation().expect("流程应存在").stage;
        if stage == target {
            return;
        }
        if stage == PreparationStage::SelectingVariant {
            assert!(state.choose_variant(id, "q4"));
        } else {
            assert!(tick(state, id).is_none());
        }
    }
    panic!("演示流程没有到达目标阶段 {target:?}");
}

fn complete(state: &mut ChatState, id: u64) -> Option<GenerationPlan> {
    reach(state, id, PreparationStage::Reporting);
    let generation = tick(state, id);
    assert_eq!(
        state.preparation().expect("流程应存在").stage,
        PreparationStage::Ready
    );
    generation
}

#[test]
fn initial_chat_is_empty_and_does_not_select_a_model() {
    let state = ChatState::default();
    assert_eq!(state.conversations().len(), 1);
    assert!(state
        .active_conversation()
        .expect("初始新会话")
        .messages
        .is_empty());
    assert_eq!(state.selected_model_id(), None);
    assert_eq!(state.selected_model(), None);
    assert_eq!(state.reasoning_effort(), ReasoningEffort::High);
    assert!(!state.can_send());
    assert!(!state.is_busy());
    assert!(!state.is_generating());
    assert!(!state.model_menu_open());
    assert_eq!(state.preparation(), None);
}

#[test]
fn message_without_a_model_opens_search_and_preserves_draft() {
    let mut state = chat();
    state.set_draft("先保留我的问题".into());
    assert!(state.can_send());
    assert!(state.send_message().is_none());
    assert!(state.model_menu_open());
    assert_eq!(state.draft(), "先保留我的问题");
    assert_eq!(state.selected_model_id(), None);
    assert!(state
        .active_conversation()
        .expect("新会话")
        .messages
        .is_empty());
}

#[test]
fn model_and_hardware_display_copy_has_no_demo_labels() {
    let models = catalog();
    let mut labels: Vec<_> = models
        .iter()
        .flat_map(|model| [model.name.as_str(), model.description.as_str()])
        .collect();
    labels.extend(SIMULATED_REPLY_SEGMENTS);
    for platform in [
        DemoPlatform::Windows,
        DemoPlatform::MacOs,
        DemoPlatform::Linux,
    ] {
        let hardware = hardware(platform);
        assert!(!hardware.cpu.as_deref().expect("CPU").contains("演示"));
        labels.push(platform.label());
        assert!(!path(platform, MODEL, "q4").to_lowercase().contains("demo"));
    }
    for label in labels {
        for marker in ["演示", "示例", "模拟"] {
            assert!(!label.contains(marker), "界面文案不应包含标签：{label}");
        }
    }
}

#[test]
fn selecting_unready_model_waits_and_repeated_selection_does_not_add_cards() {
    let mut state = selected_chat();
    let before = state.preparation().expect("待启动卡片").clone();
    assert_eq!(before.stage, PreparationStage::AwaitingStart);
    assert_eq!(before.hardware_events.len(), 0);
    assert!(!state.is_busy());
    assert!(!state.is_model_ready(MODEL));
    assert!(select(&mut state, MODEL));
    assert_eq!(state.preparation(), Some(&before));
    assert_eq!(state.active_conversation().expect("会话").messages.len(), 1);
    assert!(!select(&mut state, "unknown"));
    assert_eq!(state.selected_model_id(), Some(MODEL));
}

#[test]
fn ready_model_does_not_create_a_preparation_card() {
    let mut state = chat();
    assert!(select(&mut state, READY_MODEL));
    assert!(state.is_model_ready(READY_MODEL));
    assert!(state.preparation().is_none());
    state.set_draft("直接聊天".into());
    assert!(matches!(
        state.send_message(),
        Some(ChatAction::Generate(_))
    ));
    assert!(state.is_generating());
}

#[test]
fn hardware_callbacks_are_deterministic_and_version_selection_waits_for_user() {
    let mut state = selected_chat();
    let id = start(&mut state);
    let hardware = hardware(DemoPlatform::Windows);
    let events = [
        HardwareEvent::Cpu(hardware.cpu.clone().expect("CPU")),
        HardwareEvent::Gpus(hardware.gpus.clone().expect("GPU")),
        HardwareEvent::Memory(hardware.ram_bytes),
        HardwareEvent::Disk(hardware.disk_bytes),
    ];
    for (index, event) in events.into_iter().enumerate() {
        state.hardware_event(id, event);
        assert_eq!(
            state.preparation().expect("流程").progress(),
            Some(((index + 1) * 25) as u8)
        );
        assert_eq!(
            state.preparation().expect("流程").stage,
            PreparationStage::Detecting
        );
    }
    assert!(state.initialization_request(id).is_none());
    state.finish_hardware(id, Ok(hardware));
    assert_eq!(
        state.preparation().expect("流程").stage,
        PreparationStage::SelectingVariant
    );
    assert!(state.initialization_request(id).is_none());
    assert!(!state.choose_variant(id, "invalid"));
    assert!(!state.choose_variant(id + 1, "q4"));
    assert!(state.choose_variant(id, "q8"));
    assert!(!state.choose_variant(id, "q8"));
    for percent in [20, 40, 60, 80, 100] {
        state.preparation_event(
            id,
            PreparationEvent::DownloadProgress {
                completed: percent,
                total: Some(100),
            },
        );
        assert_eq!(
            state.preparation().expect("流程").progress(),
            Some(percent as u8)
        );
    }
    assert_eq!(
        state.preparation().expect("流程").stage,
        PreparationStage::Downloading
    );
    state.preparation_event(id, PreparationEvent::Downloaded);
    assert_eq!(
        state.preparation().expect("流程").stage,
        PreparationStage::Verifying
    );
}

#[test]
fn resources_match_at_the_boundary_and_report_each_shortfall() {
    let models = catalog();
    let model = models
        .iter()
        .find(|model| model.id() == "qwen3.6-max")
        .expect("不足夹具");
    let variant = &model.variants[0];
    let mut hardware = hardware(DemoPlatform::Windows);
    hardware.ram_bytes = Some(variant.ram_bytes);
    hardware.gpus.as_mut().expect("GPU")[0].available_bytes = Some(variant.vram_bytes);
    hardware.disk_bytes = Some(variant.disk_bytes);
    assert!(variant.fits(&hardware));
    assert!(variant.shortfall(&hardware).is_empty());
    for resource in ["内存", "显存", "磁盘"] {
        let mut insufficient = hardware.clone();
        match resource {
            "内存" => insufficient.ram_bytes = Some(variant.ram_bytes - GIB),
            "显存" => {
                insufficient.gpus.as_mut().expect("GPU")[0].available_bytes =
                    Some(variant.vram_bytes - GIB)
            }
            _ => insufficient.disk_bytes = Some(variant.disk_bytes - GIB),
        }
        assert!(!variant.fits(&insufficient));
        let reason = variant.shortfall(&insufficient);
        assert!(reason.contains(resource));
        assert!(reason.contains("差 1 GiB"));
    }
    let mut state = chat();
    assert!(select(&mut state, model.id()));
    let id = start(&mut state);
    detected(&mut state, id, hardware);
    assert!(state.choose_variant(id, "q8"));
}

#[test]
fn insufficient_hardware_stops_and_restores_the_pending_message() {
    let mut state = chat();
    assert!(select(&mut state, "qwen3.6-max"));
    let id = send_pending(&mut state);
    detected(&mut state, id, hardware(DemoPlatform::Windows));
    let preparation = state.preparation().expect("失败卡片");
    assert_eq!(preparation.stage, PreparationStage::Failed);
    let error = &preparation.error.as_ref().expect("不足原因").reason;
    for field in [
        "内存要求 96 GiB",
        "差 72 GiB",
        "显存要求 48 GiB",
        "差 40 GiB",
        "磁盘要求 256 GiB",
        "差 128 GiB",
    ] {
        assert!(error.contains(field), "{error}");
    }
    assert!(preparation.local_path.is_none());
    assert_eq!(state.draft(), "请介绍这个模型");
    assert!(!state.choose_variant(id, "q8"));
    assert!(state.close_preparation(id));
    assert!(state.preparation().expect("摘要仍在").collapsed);
}

#[test]
fn filtered_versions_cannot_be_chosen_even_with_a_forged_action() {
    let mut hardware = hardware(DemoPlatform::Linux);
    hardware.gpus = Some(Vec::new());
    let mut state = chat();
    assert!(select(&mut state, MODEL));
    let id = start(&mut state);
    detected(&mut state, id, hardware);
    assert!(!state.choose_variant(id, "q8"));
    assert!(state.choose_variant(id, "q4"));
}

#[test]
fn every_platform_has_a_complete_demo_flow_and_platform_specific_path() {
    for (platform, prefix) in [
        (DemoPlatform::Windows, r"C:\BurnCloud\models\"),
        (DemoPlatform::MacOs, "/Users/user/BurnCloud/models/"),
        (DemoPlatform::Linux, "/home/user/BurnCloud/models/"),
    ] {
        let mut state = chat();
        assert!(select(&mut state, MODEL));
        let id = start(&mut state);
        detected(&mut state, id, hardware(platform));
        assert!(complete(&mut state, id).is_none());
        let preparation = state.preparation().expect("成功卡片");
        assert_eq!(preparation.variant_id.as_deref(), Some("q4"));
        assert_eq!(preparation.actual_digest.as_deref(), Some(DEMO_DIGEST));
        assert!(preparation
            .local_path
            .as_deref()
            .expect("示例路径")
            .starts_with(prefix));
        assert!(!preparation
            .local_path
            .as_deref()
            .expect("示例路径")
            .contains("qwen3:8b"));
        assert!(state.is_model_ready(MODEL));
        assert!(state.close_preparation(id));
        assert!(state.preparation().expect("结果摘要").collapsed);
    }
}

#[test]
fn verification_requires_an_explicit_matching_digest() {
    let mut state = selected_chat();
    let id = send_pending(&mut state);
    reach(&mut state, id, PreparationStage::Verifying);
    assert!(!state.is_model_ready(MODEL));
    state.preparation_event(
        id,
        PreparationEvent::Verified {
            expected: DEMO_DIGEST.into(),
            actual: "sha256:wrong".into(),
            path: "test.gguf".into(),
        },
    );
    let preparation = state.preparation().expect("校验失败");
    assert_eq!(preparation.stage, PreparationStage::Failed);
    assert_eq!(preparation.local_path, None);
    assert!(preparation
        .error
        .as_ref()
        .expect("错误")
        .reason
        .contains("摘要不一致"));
    assert_eq!(state.draft(), "请介绍这个模型");
    assert!(!state.is_model_ready(MODEL));
    let failed = state.clone();
    state.preparation_event(
        id,
        PreparationEvent::Verified {
            expected: DEMO_DIGEST.into(),
            actual: DEMO_DIGEST.into(),
            path: "test.gguf".into(),
        },
    );
    assert_eq!(state, failed);
}

#[test]
fn failure_at_each_running_stage_stops_and_does_not_mark_ready() {
    for stage in [
        PreparationStage::Detecting,
        PreparationStage::Downloading,
        PreparationStage::Verifying,
        PreparationStage::LoadingPackage,
        PreparationStage::Deploying,
        PreparationStage::HealthChecking,
        PreparationStage::Reporting,
    ] {
        let mut state = selected_chat();
        let id = send_pending(&mut state);
        reach(&mut state, id, stage);
        assert!(!state.is_model_ready(MODEL));
        assert!(failure(&mut state, id, "演示阶段故障").is_none());
        let preparation = state.preparation().expect("失败卡片");
        assert_eq!(preparation.stage, PreparationStage::Failed);
        assert_eq!(
            preparation.error.as_ref().expect("错误阶段").operation,
            Operation::Initialize(stage)
        );
        assert_eq!(preparation.local_path, None);
        assert!(!state.is_model_ready(MODEL));
        assert!(!state.is_busy());
        assert_eq!(state.draft(), "请介绍这个模型");
        assert!(state
            .active_conversation()
            .expect("会话")
            .messages
            .iter()
            .all(|message| message.role == MessageRole::System));
    }
}

#[test]
fn only_reporting_success_marks_ready_and_resumes_pending_message_once() {
    let mut state = selected_chat();
    let id = send_pending(&mut state);
    assert!(state.draft().is_empty());
    let generation = complete(&mut state, id).expect("自动续聊");
    assert!(state.is_generation_active(&generation));
    assert_eq!(state.conversations()[0].title, "请介绍这个模型");
    let conversation = state.active_conversation().expect("会话");
    assert_eq!(
        conversation
            .messages
            .iter()
            .filter(|message| message.preparation.is_some())
            .count(),
        1
    );
    assert_eq!(
        conversation
            .messages
            .iter()
            .filter(|message| message.role == MessageRole::User)
            .count(),
        1
    );
    assert!(tick(&mut state, id).is_none());
    assert!(state.send_message().is_none());
    state.generation_event(
        &generation,
        GenerationEvent::Thinking("正在组织回复内容。".into()),
    );
    for segment in SIMULATED_REPLY_SEGMENTS {
        assert!(chunk(&mut state, &generation, segment));
    }
    assert!(state.complete_generation(&generation, Ok(Some(metrics()))));
    let reply = state
        .active_conversation()
        .expect("会话")
        .messages
        .last()
        .expect("回复");
    assert_eq!(reply.content, SIMULATED_REPLY_SEGMENTS.concat());
    assert_eq!(reply.thinking.as_deref(), Some("正在组织回复内容。"));
    assert_eq!(reply.metrics.as_ref().expect("回复指标").node, "本地节点");
    assert!(!state.is_busy());
}

#[test]
fn download_without_pending_message_does_not_send_an_unsent_draft() {
    let mut state = selected_chat();
    state.set_draft("尚未点击发送".into());
    let id = start(&mut state);
    assert!(complete(&mut state, id).is_none());
    assert_eq!(state.draft(), "尚未点击发送");
    assert!(!state.is_generating());
    assert_eq!(state.active_conversation().expect("会话").messages.len(), 1);
}

#[test]
fn busy_task_blocks_duplicate_start_model_changes_and_conversation_mutations() {
    let mut state = selected_chat();
    let conversation_id = state.active_conversation().expect("会话").id.clone();
    let id = send_pending(&mut state);
    assert!(state.start_preparation(id).is_none());
    assert!(state.send_message().is_none());
    assert!(!state.choose_variant(id, "q4"));
    assert!(!state.close_preparation(id));
    assert!(!select(&mut state, READY_MODEL));
    assert!(!state.select_conversation(&conversation_id));
    assert!(!state.delete_conversation(&conversation_id));
    state.new_conversation();
    assert_eq!(state.conversations().len(), 1);
    state.set_draft("不应覆盖待发消息".into());
    assert!(state.draft().is_empty());
    state.toggle_model_menu();
    assert!(!state.model_menu_open());
    state.set_reasoning_effort(ReasoningEffort::Low);
    assert_eq!(state.reasoning_effort(), ReasoningEffort::High);
    reach(&mut state, id, PreparationStage::SelectingVariant);
    assert!(state.is_busy());
    assert!(!select(&mut state, READY_MODEL));
}

#[test]
fn closing_variant_choice_restores_draft_and_preserves_a_summary() {
    let mut state = selected_chat();
    let id = send_pending(&mut state);
    reach(&mut state, id, PreparationStage::SelectingVariant);
    assert!(state.close_preparation(id));
    assert_eq!(state.draft(), "请介绍这个模型");
    let preparation = state.preparation().expect("摘要");
    assert!(preparation.collapsed);
    assert_eq!(preparation.stage, PreparationStage::Cancelled);
    assert!(!state.is_busy());
    assert!(!state.choose_variant(id, "q4"));
    assert!(tick(&mut state, id).is_none());
    assert_eq!(state.active_conversation().expect("会话").messages.len(), 1);
}

#[test]
fn retry_resets_progress_and_rejects_late_events_from_the_previous_run() {
    let mut state = selected_chat();
    let old_id = send_pending(&mut state);
    reach(&mut state, old_id, PreparationStage::Downloading);
    assert!(failure(&mut state, old_id, "网络失败").is_none());
    let new_id = start(&mut state);
    assert_ne!(old_id, new_id);
    let restarted = state.clone();
    assert!(tick(&mut state, old_id).is_none());
    assert!(failure(&mut state, old_id, "迟到错误").is_none());
    assert_eq!(state, restarted);
    assert_eq!(state.preparation().expect("重试卡片").progress(), Some(0));
    assert!(state.preparation().expect("重试卡片").variant_id.is_none());
    assert!(complete(&mut state, new_id).is_none());
    assert_eq!(state.draft(), "请介绍这个模型");
    assert_eq!(
        state
            .active_conversation()
            .expect("同一卡片")
            .messages
            .len(),
        1
    );
}

#[test]
fn ready_state_is_shared_by_conversations_but_reset_in_a_new_client_session() {
    let mut state = selected_chat();
    let id = start(&mut state);
    assert!(complete(&mut state, id).is_none());
    let original = state.active_conversation().expect("会话").id.clone();
    state.new_conversation();
    assert_eq!(state.selected_model_id(), Some(MODEL));
    assert!(state.preparation().is_none());
    state.set_draft("无需再次部署".into());
    assert!(matches!(
        state.send_message(),
        Some(ChatAction::Generate(_))
    ));
    state.interrupt_generation();
    assert!(state.select_conversation(&original));
    assert!(state.is_model_ready(MODEL));
    assert!(!ChatState::default().is_model_ready(MODEL));
}

#[test]
fn empty_message_generation_interruption_and_stale_chunks_are_safe() {
    let mut state = chat();
    assert!(select(&mut state, READY_MODEL));
    state.set_draft(" \n ".into());
    assert!(!state.can_send());
    assert!(state.begin_generation().is_none());
    state.set_draft("第一条消息".into());
    let plan = state.begin_generation().expect("就绪模型");
    assert!(chunk(&mut state, &plan, "部分内容"));
    state.interrupt_generation();
    assert!(!chunk(&mut state, &plan, "迟到分段"));
    assert!(!state.complete_generation(&plan, Ok(None)));
    assert!(!state.is_generating());
    let message = state
        .active_conversation()
        .expect("会话")
        .messages
        .last()
        .expect("回复");
    assert_eq!(message.content, "部分内容");
    assert!(!message.streaming);
}

#[test]
fn sidebar_reasoning_and_menus_still_work() {
    let mut state = chat();
    state.toggle_sidebar();
    assert!(state.sidebar_collapsed());
    state.toggle_sidebar();
    state.collapse_sidebar();
    assert!(state.sidebar_collapsed());
    for (effort, label, position) in [
        (ReasoningEffort::Low, "低", 0),
        (ReasoningEffort::Medium, "中", 33),
        (ReasoningEffort::High, "高", 67),
        (ReasoningEffort::ExtraHigh, "极高", 100),
    ] {
        state.set_reasoning_effort(effort);
        assert_eq!(state.reasoning_effort(), effort);
        assert_eq!(effort.label(), label);
        assert_eq!(effort.position_percent(), position);
    }
    assert_eq!(ReasoningEffort::ALL.len(), 4);
    state.toggle_model_menu();
    assert!(state.model_menu_open());
    state.close_menus();
    assert!(!state.model_menu_open());
}

#[test]
fn copy_and_thinking_only_modify_the_target_assistant_message() {
    let mut state = chat();
    assert!(select(&mut state, READY_MODEL));
    state.set_draft("测试复制".into());
    let plan = state.begin_generation().expect("生成计划");
    assert!(chunk(&mut state, &plan, "演示回复"));
    assert!(state.generation_event(&plan, GenerationEvent::Thinking("思考".into())));
    assert!(state.complete_generation(&plan, Ok(None)));
    let id = state.active_conversation().expect("会话").id.clone();
    assert!(state.mark_message_copied(&id, 1, true));
    assert!(state.active_conversation().expect("会话").messages[1].copied);
    assert!(!state.mark_message_copied(&id, 0, true));
    assert!(!state.mark_message_copied("missing", 1, true));
    assert!(!state.mark_message_copied(&id, 99, true));
    assert!(!state.generation_event(&plan, GenerationEvent::Thinking("思考".into())));
    assert!(state.toggle_thinking(&id, 1));
    assert!(!state.active_conversation().expect("会话").messages[1].thinking_open);
    assert!(!state.toggle_thinking(&id, 0));
}

#[test]
fn conversation_switch_and_delete_keep_a_valid_empty_workspace() {
    let mut state = chat();
    assert!(select(&mut state, READY_MODEL));
    let first = state.active_conversation().expect("会话").id.clone();
    state.new_conversation();
    let second = state.active_conversation().expect("会话").id.clone();
    assert_ne!(first, second);
    assert_eq!(state.selected_model_id(), Some(READY_MODEL));
    assert!(state.select_conversation(&first));
    assert!(!state.select_conversation("missing"));
    assert!(state.delete_conversation(&first));
    assert!(state.is_active_conversation(&second));
    assert!(state.delete_conversation(&second));
    assert_eq!(state.conversations().len(), 1);
    assert_eq!(state.selected_model_id(), None);
    assert!(state
        .active_conversation()
        .expect("新会话")
        .messages
        .is_empty());
    assert!(!state.delete_conversation("missing"));
}

#[tokio::test]
async fn public_chat_home_component_builds_without_external_services() {
    let mut dom = dioxus::prelude::VirtualDom::new(super::ChatHome);
    dom.rebuild_in_place();
}

#[test]
fn routes_keep_chat_aliases_and_buyer_workbench() {
    assert!(matches!("/".parse::<Route>(), Ok(Route::Home {})));
    assert!(matches!("/home".parse::<Route>(), Ok(Route::PublicHome {})));
    assert!(matches!("/buyer".parse::<Route>(), Ok(Route::Buyer {})));
    assert!(matches!(
        "/buyer/playground".parse::<Route>(),
        Ok(Route::Playground {})
    ));
    assert!(matches!(
        "/buyer/chat".parse::<Route>(),
        Ok(Route::BuyerChatRoute {})
    ));
}

#[test]
fn an_old_waiting_card_cannot_restart_a_model_prepared_in_another_conversation() {
    let mut state = selected_chat();
    let old_conversation = state.active_conversation().expect("原会话").id.clone();
    let old_run = state.preparation().expect("等待卡片").run_id;
    state.new_conversation();
    let new_run = start(&mut state);
    assert!(complete(&mut state, new_run).is_none());
    assert!(state.select_conversation(&old_conversation));
    assert!(state.is_model_ready(MODEL));
    assert!(state.start_preparation(old_run).is_none());
    state.set_draft("已经就绪，不重新下载".into());
    assert!(matches!(
        state.send_message(),
        Some(ChatAction::Generate(_))
    ));
}

#[test]
fn historical_cards_and_out_of_phase_digest_events_cannot_drive_the_current_task() {
    let mut state = selected_chat();
    let old_id = state.preparation().expect("首个卡片").run_id;
    assert!(select(&mut state, "qwen3:4b"));
    let current_id = start(&mut state);
    let before = state.clone();
    assert!(failure(&mut state, old_id, "迟到错误").is_none());
    state.preparation_event(
        current_id,
        PreparationEvent::Verified {
            expected: DEMO_DIGEST.into(),
            actual: DEMO_DIGEST.into(),
            path: "test.gguf".into(),
        },
    );
    assert!(!state.close_preparation(old_id));
    assert_ne!(state, before);
    assert_eq!(
        state.preparation().expect("流程").stage,
        PreparationStage::Failed
    );
}

fn metrics() -> super::model::InferenceMetrics {
    super::model::InferenceMetrics {
        tokens_per_second: 10.0,
        ttft_ms: 1,
        node: "本地节点".into(),
        cost_usd: 0.0,
        prompt_tokens: 1,
        completion_tokens: 2,
    }
}

#[path = "backend_tests.rs"]
mod backend_tests;
