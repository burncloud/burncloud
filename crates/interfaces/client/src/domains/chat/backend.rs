//! 默认本地替身；目录、设备、路径、摘要、回复与延迟仅在这里生成。

use super::model::{
    valid_digest, ChatError, ChatModel, ErrorKind, GenerationEvent, GenerationRequest, Gpu,
    Hardware, HardwareEvent, InferenceMetrics, InitializationRequest, MessageRole, ModelVariant,
    Operation, Platform, PreparationEvent, PreparationStage, PreparedModel, GIB,
};
use std::{future::Future, pin::Pin};

/// 单线程界面使用标准库 boxed future；不要求替身引入执行器依赖。
pub(super) type BackendFuture<T> = Pin<Box<dyn Future<Output = Result<T, ChatError>>>>;
/// 回调按调用顺序同步交付，取消 future 后不得再主动交付。
pub(super) type Callback<T> = Box<dyn FnMut(T)>;
/// 聊天模块内部替换边界，不对客户端其他模块公开。
///
/// # Examples
/// 页面持有一个本地实例；测试以相同五个方法注入可控结果。
///
/// # Notes
/// 初始化成功必须包含完整阶段凭据，生成指标可以缺省。丢弃 future 表示取消请求。
pub(super) trait ChatBackend {
    /// 接受空关键词；返回持有的名称、ID、描述、可选更新时间和版本快照。
    #[allow(
        dead_code,
        reason = "保留现有五方法后端契约，目录选择已改用 HTTP，工件测试仍使用此方法"
    )]
    fn search(&self, keyword: String) -> BackendFuture<Vec<ChatModel>>;
    /// 查询本客户端可用的版本凭据；未知 ID 和空 ID 返回结构化错误。
    fn model_status(&self, model_id: String) -> BackendFuture<Vec<PreparedModel>>;
    /// 回调报告已完成项，最终资源快照与回调一致；未知量保留为 None。
    fn detect_hardware(&self, callback: Callback<HardwareEvent>) -> BackendFuture<Hardware>;
    /// 仅接受用户已确认版本；依次回调下载、校验、加载、部署、健康和上报。
    fn initialize(
        &self,
        request: InitializationRequest,
        callback: Callback<PreparationEvent>,
    ) -> BackendFuture<PreparedModel>;
    /// 传入实际用户/助手历史及推理强度；流式回调正文和思考，指标可缺省。
    fn generate(
        &self,
        request: GenerationRequest,
        callback: Callback<GenerationEvent>,
    ) -> BackendFuture<Option<InferenceMetrics>>;
}
/// 默认替身不执行网络、探测、下载、文件校验或推理。
#[derive(Default)]
pub(super) struct LocalBackend;

pub(super) const LOCAL_DIGEST: &str =
    "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

fn local_model(id: &str, description: &str, date: &str) -> ChatModel {
    let large = id == "qwen3.6-max";
    let versions = if large {
        vec![("q8", "Q8_0", 96, 48, 256)]
    } else {
        vec![("q4", "Q4_K_M", 8, 0, 4), ("q8", "Q8_0", 16, 8, 8)]
    };
    ChatModel {
        id: id.into(),
        name: id.into(),
        description: description.into(),
        updated_at: Some(date.into()),
        catalog_selection: None,
        variants: versions
            .into_iter()
            .map(|(id, quantization, ram, vram, disk)| ModelVariant {
                id: id.into(),
                quantization: quantization.into(),
                runtime: "local-placeholder".into(),
                artifact_source: "local-fixture://catalog".into(),
                expected_sha256: LOCAL_DIGEST.into(),
                ram_bytes: ram * GIB,
                vram_bytes: vram * GIB,
                disk_bytes: disk * GIB,
            })
            .collect(),
    }
}
fn catalog() -> Vec<ChatModel> {
    vec![
        local_model(
            "qwen3.6-max",
            "需要较高的内存、显存和磁盘资源。",
            "2026-10-24",
        ),
        local_model("qwen3:0.6b", "轻量对话与快速体验。", "2026-10-23"),
        local_model("qwen3:1.7b", "中文问答与日常助手。", "2026-10-23"),
        local_model("qwen3:4b", "本地对话与文本摘要。", "2026-10-22"),
        local_model("qwen3:8b", "通用对话与多语言任务。", "2026-10-21"),
        local_model("qwen3:14b", "长文本理解与知识问答。", "2026-10-20"),
        local_model("qwen3:30b", "多步骤任务与内容生成。", "2026-10-19"),
        local_model("qwen3:32b", "复杂指令与文本分析。", "2026-10-18"),
        local_model("qwen2.5:0.5b", "低资源对话体验。", "2026-10-17"),
        local_model("qwen2.5:3b", "短文本摘要与分类。", "2026-10-16"),
        local_model("qwen2.5:7b", "中文写作与通用问答。", "2026-10-15"),
        local_model("qwen2.5-coder:7b", "代码编写与解释。", "2026-10-14"),
        local_model("deepseek-r1:1.5b", "推理流程与分段回复体验。", "2026-10-13"),
        local_model("llama3.2:1b", "轻量对话与日常问答。", "2026-10-12"),
    ]
}
fn current_platform() -> Platform {
    if cfg!(all(feature = "desktop", target_os = "macos")) {
        Platform::MacOs
    } else if cfg!(all(feature = "desktop", target_os = "linux")) {
        Platform::Linux
    } else {
        Platform::Windows
    }
}
fn hardware(platform: Platform) -> Hardware {
    let (cpu, gpu) = match platform {
        Platform::Windows => ("x64 CPU · 8 核", "独立 GPU"),
        Platform::MacOs => ("Apple CPU · 8 核", "Apple GPU"),
        Platform::Linux => ("x64 CPU · 8 核", "Linux GPU"),
    };
    Hardware {
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
fn example_path(platform: Platform, model: &str, variant: &str) -> String {
    let name = model.replace(':', "-");
    match platform {
        Platform::Windows => format!(r"C:\BurnCloud\models\{name}-{variant}.gguf"),
        Platform::MacOs => format!("/Users/user/BurnCloud/models/{name}-{variant}.gguf"),
        Platform::Linux => format!("/home/user/BurnCloud/models/{name}-{variant}.gguf"),
    }
}
fn error(kind: ErrorKind, operation: Operation, reason: &str) -> ChatError {
    ChatError {
        kind,
        operation,
        reason: reason.into(),
    }
}
async fn pause() {
    tokio::time::sleep(std::time::Duration::from_millis(180)).await;
}

impl ChatBackend for LocalBackend {
    fn search(&self, keyword: String) -> BackendFuture<Vec<ChatModel>> {
        Box::pin(async move {
            let query = keyword.trim().to_lowercase();
            Ok(catalog()
                .into_iter()
                .filter(|model| {
                    [&model.id, &model.name, &model.description]
                        .into_iter()
                        .any(|value| value.to_lowercase().contains(&query))
                })
                .collect())
        })
    }
    fn model_status(&self, model_id: String) -> BackendFuture<Vec<PreparedModel>> {
        Box::pin(async move {
            if model_id.trim().is_empty() {
                return Err(error(
                    ErrorKind::InvalidParameter,
                    Operation::Status,
                    "模型 ID 不能为空",
                ));
            }
            let model = catalog()
                .into_iter()
                .find(|model| model.id == model_id)
                .ok_or_else(|| error(ErrorKind::NotFound, Operation::Status, "未找到模型"))?;
            if model.id != "llama3.2:1b" {
                return Ok(Vec::new());
            }
            let variant = model.variants[0].clone();
            Ok(vec![PreparedModel {
                local_path: example_path(current_platform(), &model.id, &variant.id),
                model,
                variant,
                actual_sha256: LOCAL_DIGEST.into(),
            }])
        })
    }
    fn detect_hardware(&self, mut callback: Callback<HardwareEvent>) -> BackendFuture<Hardware> {
        Box::pin(async move {
            let result = hardware(current_platform());
            for event in [
                HardwareEvent::Cpu(result.cpu.clone().unwrap_or_default()),
                HardwareEvent::Gpus(result.gpus.clone().unwrap_or_default()),
                HardwareEvent::Memory(result.ram_bytes),
                HardwareEvent::Disk(result.disk_bytes),
            ] {
                pause().await;
                callback(event);
            }
            Ok(result)
        })
    }
    fn initialize(
        &self,
        request: InitializationRequest,
        mut callback: Callback<PreparationEvent>,
    ) -> BackendFuture<PreparedModel> {
        Box::pin(async move {
            let operation = Operation::Initialize(PreparationStage::Downloading);
            if request.run_id == 0
                || request.model.id.trim().is_empty()
                || !request.model.variants.contains(&request.variant)
                || !valid_digest(&request.variant.expected_sha256)
            {
                return Err(error(
                    ErrorKind::InvalidParameter,
                    operation,
                    "初始化快照或摘要无效",
                ));
            }
            let known = catalog()
                .into_iter()
                .find(|model| model.id == request.model.id);
            if known.as_ref() != Some(&request.model) {
                return Err(error(
                    ErrorKind::NotConnected,
                    operation,
                    "尚未接入该工件来源或运行后端",
                ));
            }
            if !request.variant.fits(&hardware(current_platform())) {
                return Err(error(
                    ErrorKind::InsufficientResources,
                    operation,
                    "硬件资源不足",
                ));
            }
            let total = request.variant.disk_bytes;
            for step in 1..=5 {
                pause().await;
                callback(PreparationEvent::DownloadProgress {
                    completed: total / 5 * step + if step == 5 { total % 5 } else { 0 },
                    total: Some(total),
                });
            }
            callback(PreparationEvent::Downloaded);
            let result = PreparedModel {
                local_path: example_path(
                    current_platform(),
                    &request.model.id,
                    &request.variant.id,
                ),
                actual_sha256: request.variant.expected_sha256.clone(),
                model: request.model,
                variant: request.variant,
            };
            for event in [
                PreparationEvent::Verified {
                    expected: result.variant.expected_sha256.clone(),
                    actual: result.actual_sha256.clone(),
                    path: result.local_path.clone(),
                },
                PreparationEvent::PackageLoaded,
                PreparationEvent::Deployed,
                PreparationEvent::Healthy,
                PreparationEvent::Reported,
            ] {
                pause().await;
                callback(event);
            }
            Ok(result)
        })
    }
    fn generate(
        &self,
        request: GenerationRequest,
        mut callback: Callback<GenerationEvent>,
    ) -> BackendFuture<Option<InferenceMetrics>> {
        Box::pin(async move {
            if !request.prepared.is_valid()
                || !request.messages.last().is_some_and(|message| {
                    message.role == MessageRole::User && !message.content.trim().is_empty()
                })
                || request
                    .messages
                    .iter()
                    .any(|message| message.role == MessageRole::System)
            {
                return Err(error(
                    ErrorKind::InvalidParameter,
                    Operation::Generate,
                    "缺少就绪版本或会话消息",
                ));
            }
            if !catalog().iter().any(|model| {
                model.id == request.prepared.model.id
                    && model.variants.contains(&request.prepared.variant)
            }) {
                return Err(error(
                    ErrorKind::NotConnected,
                    Operation::Generate,
                    "尚未接入该模型版本的推理后端",
                ));
            }
            callback(GenerationEvent::Thinking("正在组织回复内容。".into()));
            for segment in SIMULATED_REPLY_SEGMENTS {
                pause().await;
                callback(GenerationEvent::Content(segment.into()));
            }
            Ok(Some(InferenceMetrics {
                tokens_per_second: 128.4,
                ttft_ms: 41,
                node: "本地节点".into(),
                cost_usd: 0.00014,
                prompt_tokens: 42,
                completion_tokens: 184,
            }))
        })
    }
}

pub(super) const SIMULATED_REPLY_SEGMENTS: [&str; 4] = [
    "你可以在输入框旁搜索并选择模型。",
    "\n\n模型初始化包含硬件检测、量化版本选择、下载、校验和部署。",
    "\n\n初始化完成后可继续对话；如果中途失败，原消息会保留在输入框中。",
    "\n\n本次客户端会话内会保留模型就绪状态，重新启动后会重置。",
];

#[cfg(test)]
#[path = "local_backend_tests.rs"]
mod tests;
