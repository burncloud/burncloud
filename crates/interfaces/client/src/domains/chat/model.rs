//! 聊天模块内部的接口数据契约与纯资源比较；不生成外部能力结果。

use super::state::ReasoningEffort;

/// 字节与 GiB 的展示换算基数。
pub(super) const GIB: u64 = 1024 * 1024 * 1024;

/// 模型身份与显示名称分离，更新时间采用可排序的 ISO 日期或时间。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatModel {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) updated_at: Option<String>,
    pub(super) variants: Vec<ModelVariant>,
    /// 真实目录选择快照，不等同于量化工件。
    pub(super) catalog_selection: Option<super::catalog::CatalogSelection>,
}
impl ChatModel {
    /// 返回稳定模型标识，不以显示名称代替。
    pub(super) fn id(&self) -> &str {
        &self.id
    }
}

/// 版本及不可变工件快照；资源要求统一使用字节。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ModelVariant {
    pub(super) id: String,
    pub(super) quantization: String,
    pub(super) runtime: String,
    pub(super) artifact_source: String,
    pub(super) expected_sha256: String,
    pub(super) ram_bytes: u64,
    pub(super) vram_bytes: u64,
    pub(super) disk_bytes: u64,
}
impl ModelVariant {
    /// 未知资源不视为充足；显存只比较单卡最大可用量。
    pub(super) fn fits(&self, hardware: &Hardware) -> bool {
        [
            (self.ram_bytes, hardware.ram_bytes),
            (self.vram_bytes, hardware.max_vram()),
            (self.disk_bytes, hardware.disk_bytes),
        ]
        .into_iter()
        .all(|(required, available)| available.is_some_and(|value| value >= required))
    }
    /// 直接列出不足差额或未知项，不计算推荐评分。
    pub(super) fn shortfall(&self, hardware: &Hardware) -> String {
        [
            ("内存", self.ram_bytes, hardware.ram_bytes),
            ("显存", self.vram_bytes, hardware.max_vram()),
            ("磁盘", self.disk_bytes, hardware.disk_bytes),
        ]
        .into_iter()
        .filter_map(|(name, required, available)| match available {
            None => Some(format!("{name}要求 {} GiB，可用量未知", gib(required))),
            Some(value) if value < required => Some(format!(
                "{name}要求 {} GiB，可用 {} GiB，差 {} GiB",
                gib(required),
                gib(value),
                gib(required - value)
            )),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("；")
    }
}
/// 只在展示边界转换单位。
pub(super) fn gib(bytes: u64) -> String {
    format!("{}", bytes as f64 / GIB as f64)
}
/// 未知资源使用明确的文字，而非零或充足。
pub(super) fn resource_label(bytes: Option<u64>) -> String {
    bytes
        .map(|value| format!("{} GiB", gib(value)))
        .unwrap_or_else(|| "未知".into())
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Platform {
    Windows,
    MacOs,
    Linux,
}
impl Platform {
    /// 平台名称仅用于呈现后端结果。
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
            Self::Linux => "Linux",
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Gpu {
    pub(super) name: String,
    pub(super) available_bytes: Option<u64>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Hardware {
    pub(super) platform: Option<Platform>,
    pub(super) cpu: Option<String>,
    pub(super) gpus: Option<Vec<Gpu>>,
    pub(super) ram_bytes: Option<u64>,
    pub(super) disk_bytes: Option<u64>,
}
impl Hardware {
    /// 已知无 GPU 时可用显存为零；未知 GPU 不推断为可用。
    pub(super) fn max_vram(&self) -> Option<u64> {
        self.gpus.as_ref().and_then(|gpus| {
            if gpus.is_empty() {
                Some(0)
            } else {
                gpus.iter().filter_map(|gpu| gpu.available_bytes).max()
            }
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) enum HardwareEvent {
    Cpu(String),
    Gpus(Vec<Gpu>),
    Memory(Option<u64>),
    Disk(Option<u64>),
}

/// 一张流程卡片的全部阶段，失败原因单独保存以避免多套状态机。
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PreparationStage {
    AwaitingStart,
    Detecting,
    SelectingVariant,
    Downloading,
    Verifying,
    LoadingPackage,
    Deploying,
    HealthChecking,
    Reporting,
    Ready,
    Failed,
    Cancelled,
}

impl PreparationStage {
    /// 返回描述当前流程的阶段名称。
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::AwaitingStart => "启用中…（等待点击下载或发送消息）",
            Self::Detecting => "正在进行硬件检测…",
            Self::SelectingVariant => "模型解析完成，请选择量化版本",
            Self::Downloading => "正在下载模型…",
            Self::Verifying => "下载完成，正在校验摘要…",
            Self::LoadingPackage => "正在加载安装包…",
            Self::Deploying => "正在自动部署…",
            Self::HealthChecking => "正在健康检查…",
            Self::Reporting => "正在上报模型状态…",
            Self::Ready => "部署成功，模型已就绪",
            Self::Failed => "初始化失败",
            Self::Cancelled => "已关闭初始化流程",
        }
    }

    /// 是否仍在等待后端结果；等待用户选择版本时必须暂停。
    pub(super) fn is_running(self) -> bool {
        matches!(
            self,
            Self::Detecting
                | Self::Downloading
                | Self::Verifying
                | Self::LoadingPackage
                | Self::Deploying
                | Self::HealthChecking
                | Self::Reporting
        )
    }
}

/// 流程事件必须按现有阶段顺序消费；重复的同值回调不再次产生副作用。
#[derive(Clone, Debug, PartialEq)]
pub(super) enum PreparationEvent {
    DownloadProgress {
        completed: u64,
        total: Option<u64>,
    },
    Downloaded,
    Verified {
        expected: String,
        actual: String,
        path: String,
    },
    PackageLoaded,
    Deployed,
    Healthy,
    Reported,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Preparation {
    pub(super) run_id: u64,
    pub(super) model: ChatModel,
    pub(super) stage: PreparationStage,
    pub(super) completed_bytes: u64,
    pub(super) total_bytes: Option<u64>,
    pub(super) hardware_events: Vec<HardwareEvent>,
    pub(super) hardware: Hardware,
    pub(super) variant_id: Option<String>,
    pub(super) events: Vec<PreparationEvent>,
    pub(super) actual_digest: Option<String>,
    pub(super) local_path: Option<String>,
    pub(super) error: Option<ChatError>,
    pub(super) collapsed: bool,
}
impl Preparation {
    /// 未知下载总量不生成百分比；检测项按已完成回调显示。
    pub(super) fn progress(&self) -> Option<u8> {
        if self.stage == PreparationStage::Detecting {
            return Some((self.hardware_events.len() * 25) as u8);
        }
        self.total_bytes
            .filter(|total| *total > 0)
            .map(|total| ((u128::from(self.completed_bytes) * 100) / u128::from(total)) as u8)
    }
    /// 获取已确认的版本快照。
    pub(super) fn variant(&self) -> Option<&ModelVariant> {
        self.model
            .variants
            .iter()
            .find(|v| Some(v.id.as_str()) == self.variant_id.as_deref())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct InitializationRequest {
    pub(super) run_id: u64,
    pub(super) model: ChatModel,
    pub(super) variant: ModelVariant,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreparedModel {
    pub(super) model: ChatModel,
    pub(super) variant: ModelVariant,
    pub(super) actual_sha256: String,
    pub(super) local_path: String,
}
impl PreparedModel {
    /// 校验后端成功凭据，不访问磁盘；真实文件校验属于后端职责。
    pub(super) fn is_valid(&self) -> bool {
        !self.model.id.trim().is_empty()
            && self.model.variants.contains(&self.variant)
            && valid_digest(&self.variant.expected_sha256)
            && self.actual_sha256 == self.variant.expected_sha256
            && !self.local_path.trim().is_empty()
            && !self.local_path.contains('\0')
    }
}
/// 接受 SHA-256 十六进制摘要及显式算法前缀。
pub(super) fn valid_digest(value: &str) -> bool {
    let digest = value.strip_prefix("sha256:").unwrap_or(value);
    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum ErrorKind {
    InvalidParameter,
    NotFound,
    NotConnected,
    InsufficientResources,
    VerificationFailed,
    #[allow(
        dead_code,
        reason = "接口契约要求保留超时错误，本地成功替身不主动产生超时"
    )]
    Timeout,
    #[allow(
        dead_code,
        reason = "接口契约要求保留执行错误，故障只由独立测试替身注入"
    )]
    ExecutionFailed,
    InvalidResult,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Operation {
    Search,
    Status,
    Hardware,
    Initialize(PreparationStage),
    Generate,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatError {
    pub(super) kind: ErrorKind,
    pub(super) operation: Operation,
    pub(super) reason: String,
}
impl std::fmt::Display for ChatError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}", self.reason)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct HistoryMessage {
    pub(super) role: MessageRole,
    pub(super) content: String,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) struct GenerationRequest {
    pub(super) prepared: PreparedModel,
    pub(super) messages: Vec<HistoryMessage>,
    pub(super) reasoning_effort: ReasoningEffort,
}
#[derive(Clone, Debug, PartialEq)]
pub(super) enum GenerationEvent {
    Content(String),
    Thinking(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum MessageRole {
    User,
    Assistant,
    System,
}

/// 后端可选返回的推理指标；缺失时不补造。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct InferenceMetrics {
    pub(super) tokens_per_second: f32,
    pub(super) ttft_ms: u32,
    pub(super) node: String,
    pub(super) cost_usd: f64,
    pub(super) prompt_tokens: u32,
    pub(super) completion_tokens: u32,
}

/// 复用消息列表承载流程卡片，关闭后仍保留结果摘要。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatMessage {
    pub(super) role: MessageRole,
    pub(super) content: String,
    pub(super) thinking: Option<String>,
    pub(super) thinking_open: bool,
    pub(super) streaming: bool,
    pub(super) copied: bool,
    pub(super) metrics: Option<InferenceMetrics>,
    pub(super) error: Option<ChatError>,
    pub(super) preparation: Option<Preparation>,
}

/// 当前客户端会话内的聊天记录，不落盘。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatConversation {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) model_id: Option<String>,
    pub(super) messages: Vec<ChatMessage>,
}
