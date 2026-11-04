//! Chat 首页的会话与结果驱动状态；不生成目录、设备、进度或成功凭据。

use super::catalog::{CatalogSelection, VersionStatus};
use super::model::{
    valid_digest, ChatConversation, ChatError, ChatMessage, ChatModel, ErrorKind, GenerationEvent,
    GenerationRequest, Hardware, HardwareEvent, HistoryMessage, InferenceMetrics,
    InitializationRequest, MessageRole, ModelVariant, Operation, Preparation, PreparationEvent,
    PreparationStage, PreparedModel,
};

#[derive(Clone, Debug, PartialEq)]
pub(super) struct GenerationPlan {
    pub(super) conversation_id: String,
    pub(super) generation_id: u64,
    pub(super) request: Box<GenerationRequest>,
}

/// 发送入口只调度一种动作，防止初始化与回复重复启动。
pub(super) enum ChatAction {
    Prepare(u64),
    Generate(GenerationPlan),
}

/// Chat 模型可使用的推理强度档位。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum ReasoningEffort {
    Low,
    Medium,
    #[default]
    High,
    ExtraHigh,
}

impl ReasoningEffort {
    /// 返回供选择器按从低到高渲染的固定档位。
    ///
    /// # Examples
    ///
    /// `ReasoningEffort::ALL` 可直接用于渲染四个单选档位。
    ///
    /// # Notes
    ///
    /// 档位顺序同时决定滑块的位置顺序。
    pub(super) const ALL: [Self; 4] = [Self::Low, Self::Medium, Self::High, Self::ExtraHigh];

    /// 返回当前档位在界面中显示的中文名称。
    ///
    /// # Examples
    ///
    /// `ReasoningEffort::High.label()` 返回 `"高"`。
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Low => "低",
            Self::Medium => "中",
            Self::High => "高",
            Self::ExtraHigh => "极高",
        }
    }

    /// 返回当前档位在离散滑块上的百分比位置。
    ///
    /// # Notes
    ///
    /// 返回值只用于本地界面布局，不代表远端模型接口的实际参数。
    pub(super) fn position_percent(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Medium => 33,
            Self::High => 67,
            Self::ExtraHigh => 100,
        }
    }
}

/// Chat 首页状态只在当前客户端会话存活，不写文件或数据库。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct ChatState {
    conversations: Vec<ChatConversation>,
    active_conversation_id: Option<String>,
    selected_model_id: Option<String>,
    reasoning_effort: ReasoningEffort,
    draft: String,
    pending_draft: Option<String>,
    is_generating: bool,
    sidebar_collapsed: bool,
    model_menu_open: bool,
    generation_id: u64,
    preparation_id: u64,
    next_conversation_number: u64,
    models: Vec<ChatModel>,
    selection_id: u64,
    status_loading: bool,
    status_error: Option<ChatError>,
    ready_models: Vec<PreparedModel>,
}

impl Default for ChatState {
    fn default() -> Self {
        Self {
            conversations: vec![ChatConversation {
                id: "conversation-1".into(),
                title: "新建会话".into(),
                model_id: None,
                messages: Vec::new(),
            }],
            active_conversation_id: Some("conversation-1".into()),
            selected_model_id: None,
            reasoning_effort: ReasoningEffort::default(),
            draft: String::new(),
            pending_draft: None,
            is_generating: false,
            sidebar_collapsed: false,
            model_menu_open: false,
            generation_id: 0,
            preparation_id: 0,
            next_conversation_number: 2,
            models: Vec::new(),
            selection_id: 0,
            status_loading: false,
            status_error: None,
            ready_models: Vec::new(),
        }
    }
}

impl ChatState {
    pub(super) fn conversations(&self) -> &[ChatConversation] {
        &self.conversations
    }

    pub(super) fn active_conversation(&self) -> Option<&ChatConversation> {
        let id = self.active_conversation_id.as_deref()?;
        self.conversations
            .iter()
            .find(|conversation| conversation.id == id)
    }

    pub(super) fn selected_model(&self) -> Option<ChatModel> {
        self.models
            .iter()
            .find(|model| Some(model.id()) == self.selected_model_id())
            .cloned()
    }

    pub(super) fn selected_model_id(&self) -> Option<&str> {
        self.selected_model_id.as_deref()
    }
    pub(super) fn reasoning_effort(&self) -> ReasoningEffort {
        self.reasoning_effort
    }
    pub(super) fn is_active_conversation(&self, id: &str) -> bool {
        self.active_conversation_id.as_deref() == Some(id)
    }
    pub(super) fn draft(&self) -> &str {
        &self.draft
    }
    pub(super) fn is_generating(&self) -> bool {
        self.is_generating
    }
    pub(super) fn sidebar_collapsed(&self) -> bool {
        self.sidebar_collapsed
    }
    pub(super) fn model_menu_open(&self) -> bool {
        self.model_menu_open
    }

    /// 当前只有一个初始化任务；等待选版本也锁定会话，直到继续或关闭。
    pub(super) fn is_busy(&self) -> bool {
        self.is_generating
            || self.preparation().is_some_and(|p| {
                p.stage.is_running() || p.stage == PreparationStage::SelectingVariant
            })
    }

    /// 就绪凭据绑定完整版本与摘要，模型目录变化不能复用旧工件。
    pub(super) fn is_model_ready(&self, model_id: &str) -> bool {
        self.ready_models.iter().any(|ready| {
            ready.model.id == model_id
                && self
                    .models
                    .iter()
                    .any(|model| model.id == model_id && model.variants.contains(&ready.variant))
        })
    }
    pub(super) fn status_loading(&self) -> bool {
        self.status_loading
    }
    pub(super) fn status_error(&self) -> Option<&ChatError> {
        self.status_error.as_ref()
    }
    /// 每次用户选择都查询本客户端状态，不使用模型市场的在线标志。
    pub(super) fn begin_selection(&mut self, model_id: &str) -> Option<u64> {
        if !self.set_model(model_id) {
            return None;
        }
        self.selection_id = self.selection_id.wrapping_add(1);
        self.status_loading = true;
        self.status_error = None;
        Some(self.selection_id)
    }
    /// 校验选择快照与就绪凭据，迟到或矛盾结果不能启用发送。
    pub(super) fn finish_selection(
        &mut self,
        id: u64,
        result: Result<Vec<PreparedModel>, ChatError>,
    ) {
        if id != self.selection_id || !self.status_loading {
            return;
        }
        self.status_loading = false;
        match result {
            Ok(versions) => {
                let model = self.selected_model();
                if versions
                    .iter()
                    .any(|ready| !ready.is_valid() || Some(&ready.model) != model.as_ref())
                {
                    self.status_error = Some(ChatError {
                        kind: ErrorKind::InvalidResult,
                        operation: Operation::Status,
                        reason: "模型状态返回了不匹配的版本或无效凭据".into(),
                    });
                    return;
                }
                for ready in versions {
                    self.record_ready(ready);
                }
                if self
                    .selected_model_id()
                    .is_some_and(|id| self.is_model_ready(id))
                {
                    if let Some(conversation) = self.active_conversation_mut() {
                        conversation.messages.retain(|message| {
                            !message
                                .preparation
                                .as_ref()
                                .is_some_and(|p| p.stage == PreparationStage::AwaitingStart)
                        });
                    }
                }
            }
            Err(error) => self.status_error = Some(error),
        }
    }

    pub(super) fn set_draft(&mut self, draft: String) {
        if !self.is_busy() {
            self.draft = draft;
        }
    }

    pub(super) fn set_model(&mut self, model_id: &str) -> bool {
        if self.is_busy() {
            return false;
        }
        let Some(model) = self
            .models
            .iter()
            .find(|model| model.id() == model_id)
            .cloned()
        else {
            return false;
        };
        if self.selected_model_id() == Some(model.id()) {
            if let Some(current) = self
                .preparation_mut()
                .filter(|current| current.model == model)
            {
                current.collapsed = false;
                if current.stage == PreparationStage::Cancelled {
                    current.stage = PreparationStage::AwaitingStart;
                }
                self.close_menus();
                return true;
            }
        }
        if let Some(previous) = self.preparation_mut() {
            previous.collapsed = true;
            if previous.stage == PreparationStage::AwaitingStart {
                previous.stage = PreparationStage::Cancelled;
            }
        }
        self.selected_model_id = Some(model.id().to_string());
        if let Some(conversation) = self.active_conversation_mut() {
            conversation.model_id = Some(model.id().to_string());
        }
        if !self.is_model_ready(model.id()) {
            self.preparation_id = self.preparation_id.wrapping_add(1);
            let preparation = Preparation {
                run_id: self.preparation_id,
                model,
                stage: PreparationStage::AwaitingStart,
                completed_bytes: 0,
                total_bytes: None,
                hardware_events: Vec::new(),
                events: Vec::new(),
                hardware: Hardware::default(),
                variant_id: None,
                actual_digest: None,
                local_path: None,
                error: None,
                collapsed: false,
            };
            if let Some(conversation) = self.active_conversation_mut() {
                conversation.messages.push(ChatMessage {
                    role: MessageRole::System,
                    content: String::new(),
                    thinking: None,
                    thinking_open: false,
                    streaming: false,
                    copied: false,
                    metrics: None,
                    error: None,
                    preparation: Some(preparation),
                });
            }
        }
        self.close_menus();
        true
    }

    /// 提交已确认的目录版本；远端可用状态不能伪造本客户端运行凭据。
    ///
    /// # Examples
    /// `Downloadable` 携带工件清单启动硬件检测，`Available` 缺少工件凭据时停止。
    ///
    /// # Notes
    /// 浏览预览不调用此入口；重复确认和忙碌状态不会启动第二个任务。
    pub(super) fn accept_catalog(
        &mut self,
        selection: CatalogSelection,
        status: VersionStatus,
        variants: Vec<ModelVariant>,
    ) -> Option<u64> {
        if self.is_busy()
            || self.status_loading
            || selection.name.trim().is_empty()
            || selection.version.validate().is_err()
            || !matches!(
                status,
                VersionStatus::Downloadable | VersionStatus::Available
            )
        {
            return None;
        }
        let id = selection.id();
        let model = ChatModel {
            id: id.clone(),
            name: selection.name.clone(),
            description: format!("{} · {}", selection.version.model, selection.version.source),
            updated_at: None,
            variants,
            catalog_selection: Some(selection),
        };
        if let Some(existing) = self.models.iter_mut().find(|model| model.id == id) {
            *existing = model;
        } else {
            self.models.push(model);
        }
        self.selection_id = self.selection_id.wrapping_add(1);
        self.status_error = None;
        if !self.set_model(&id) {
            return None;
        }
        if status == VersionStatus::Available {
            self.fail_preparation(ChatError {
                kind: ErrorKind::NotConnected,
                operation: Operation::Status,
                reason: "目录显示版本可用，但本客户端尚未接入该版本的运行后端与有效工件凭据".into(),
            });
            return None;
        }
        let run_id = self.preparation()?.run_id;
        self.start_preparation(run_id)
    }

    pub(super) fn set_reasoning_effort(&mut self, effort: ReasoningEffort) {
        if !self.is_busy() {
            self.reasoning_effort = effort;
        }
    }

    pub(super) fn new_conversation(&mut self) {
        if self.is_busy() || self.status_loading {
            return;
        }
        let id = format!("conversation-{}", self.next_conversation_number);
        self.next_conversation_number = self.next_conversation_number.wrapping_add(1);
        self.conversations.push(ChatConversation {
            id: id.clone(),
            title: "新建会话".into(),
            model_id: self.selected_model_id.clone(),
            messages: Vec::new(),
        });
        self.active_conversation_id = Some(id);
        self.draft.clear();
        self.status_loading = false;
        self.status_error = None;
        self.selection_id = self.selection_id.wrapping_add(1);
        self.close_menus();
        if let Some(model) = self.selected_model() {
            self.set_model(model.id());
        }
    }

    pub(super) fn select_conversation(&mut self, id: &str) -> bool {
        if self.is_busy() || self.status_loading {
            return false;
        }
        let Some(conversation) = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
        else {
            return false;
        };
        self.selected_model_id = conversation.model_id.clone();
        self.active_conversation_id = Some(id.to_string());
        self.draft.clear();
        self.close_menus();
        true
    }

    pub(super) fn delete_conversation(&mut self, id: &str) -> bool {
        if self.is_busy() || self.status_loading {
            return false;
        }
        let Some(index) = self
            .conversations
            .iter()
            .position(|conversation| conversation.id == id)
        else {
            return false;
        };
        let active = self.is_active_conversation(id);
        self.conversations.remove(index);
        if active {
            self.active_conversation_id = self
                .conversations
                .last()
                .map(|conversation| conversation.id.clone());
            self.selected_model_id = self
                .active_conversation()
                .and_then(|conversation| conversation.model_id.clone());
            self.draft.clear();
            self.close_menus();
            if self.conversations.is_empty() {
                self.new_conversation();
            }
        }
        true
    }

    pub(super) fn toggle_sidebar(&mut self) {
        self.sidebar_collapsed = !self.sidebar_collapsed;
    }
    pub(super) fn collapse_sidebar(&mut self) {
        self.sidebar_collapsed = true;
    }

    pub(super) fn toggle_thinking(&mut self, conversation_id: &str, message_index: usize) -> bool {
        let Some(message) = self
            .conversations
            .iter_mut()
            .find(|conversation| conversation.id == conversation_id)
            .and_then(|conversation| conversation.messages.get_mut(message_index))
        else {
            return false;
        };
        if message.role != MessageRole::Assistant || message.thinking.is_none() {
            return false;
        }
        message.thinking_open = !message.thinking_open;
        true
    }

    pub(super) fn toggle_model_menu(&mut self) {
        if !self.is_busy() {
            self.model_menu_open = !self.model_menu_open;
        }
    }

    pub(super) fn close_menus(&mut self) {
        self.model_menu_open = false;
    }

    pub(super) fn can_send(&self) -> bool {
        !self.is_busy()
            && !self.status_loading
            && self.status_error.is_none()
            && self.selected_model().is_none_or(|model| {
                model.catalog_selection.is_none() || self.is_model_ready(model.id())
            })
            && !self.draft.trim().is_empty()
            && self.active_conversation_id.is_some()
    }

    /// 统一处理发送：未选模型保留草稿，未就绪则等待初始化，已就绪才回复。
    pub(super) fn send_message(&mut self) -> Option<ChatAction> {
        if !self.can_send() {
            return None;
        }
        let Some(model) = self.selected_model() else {
            self.model_menu_open = true;
            return None;
        };
        if self.is_model_ready(model.id()) {
            return self.begin_generation().map(ChatAction::Generate);
        }
        let run_id = self.preparation()?.run_id;
        let run_id = self.start_preparation(run_id)?;
        self.pending_draft = Some(std::mem::take(&mut self.draft));
        Some(ChatAction::Prepare(run_id))
    }

    /// 当前选中模型最后一张流程卡片；历史卡片保持可读但不能驱动新任务。
    pub(super) fn preparation(&self) -> Option<&Preparation> {
        self.active_conversation()?
            .messages
            .iter()
            .rev()
            .filter_map(|message| message.preparation.as_ref())
            .find(|p| Some(p.model.id()) == self.selected_model_id())
    }

    /// 启动或重试必须从硬件检测开始；新的编号使旧任务回调失效。
    pub(super) fn start_preparation(&mut self, run_id: u64) -> Option<u64> {
        if self.is_busy() || self.status_loading || self.status_error.is_some() {
            return None;
        }
        let current = self.preparation()?;
        if current.run_id != run_id
            || !matches!(
                current.stage,
                PreparationStage::AwaitingStart
                    | PreparationStage::Failed
                    | PreparationStage::Cancelled
            )
            || self.is_model_ready(current.model.id())
        {
            return None;
        }
        if current.model.catalog_selection.is_some()
            && current.stage != PreparationStage::AwaitingStart
        {
            return None;
        }
        self.preparation_id = self.preparation_id.wrapping_add(1);
        let new_id = self.preparation_id;
        let preparation = self.preparation_mut()?;
        preparation.run_id = new_id;
        preparation.stage = PreparationStage::Detecting;
        preparation.completed_bytes = 0;
        preparation.total_bytes = None;
        preparation.hardware = Hardware::default();
        preparation.hardware_events.clear();
        preparation.events.clear();
        preparation.variant_id = None;
        preparation.actual_digest = None;
        preparation.local_path = None;
        preparation.error = None;
        preparation.collapsed = false;
        self.close_menus();
        Some(new_id)
    }

    /// 仅接受当前匹配列表中的版本，不允许跳过解析或重复点击下载。
    pub(super) fn choose_variant(&mut self, run_id: u64, variant_id: &str) -> bool {
        let Some(preparation) = self.preparation_mut() else {
            return false;
        };
        if preparation.run_id != run_id || preparation.stage != PreparationStage::SelectingVariant {
            return false;
        }
        let Some(variant) = preparation
            .model
            .variants
            .iter()
            .find(|variant| variant.id == variant_id && variant.fits(&preparation.hardware))
        else {
            return false;
        };
        preparation.variant_id = Some(variant.id.clone());
        preparation.stage = PreparationStage::Downloading;
        preparation.completed_bytes = 0;
        true
    }

    /// 接收已完成硬件项；重复项无副作用，矛盾回调结束当前任务。
    pub(super) fn hardware_event(&mut self, run_id: u64, event: HardwareEvent) {
        let Some(p) = self.preparation_mut() else {
            return;
        };
        if p.run_id != run_id || p.stage != PreparationStage::Detecting {
            return;
        }
        if p.hardware_events.contains(&event) {
            return;
        }
        if p.hardware_events
            .iter()
            .any(|old| std::mem::discriminant(old) == std::mem::discriminant(&event))
        {
            self.invalid_preparation("硬件检测项返回矛盾结果");
            return;
        }
        match &event {
            HardwareEvent::Cpu(cpu) => p.hardware.cpu = Some(cpu.clone()),
            HardwareEvent::Gpus(gpus) => p.hardware.gpus = Some(gpus.clone()),
            HardwareEvent::Memory(bytes) => p.hardware.ram_bytes = *bytes,
            HardwareEvent::Disk(bytes) => p.hardware.disk_bytes = *bytes,
        }
        p.hardware_events.push(event);
    }
    /// 检测返回后才能匹配版本；未知资源不推断为充足。
    pub(super) fn finish_hardware(&mut self, run_id: u64, result: Result<Hardware, ChatError>) {
        let Some(p) = self.preparation_mut() else {
            return;
        };
        if p.run_id != run_id || p.stage != PreparationStage::Detecting {
            return;
        }
        let hardware = match result {
            Ok(value) => value,
            Err(error) => {
                self.fail_preparation(error);
                return;
            }
        };
        let consistent = p.hardware_events.iter().all(|event| match event {
            HardwareEvent::Cpu(cpu) => hardware.cpu.as_ref() == Some(cpu),
            HardwareEvent::Gpus(gpus) => hardware.gpus.as_ref() == Some(gpus),
            HardwareEvent::Memory(bytes) => hardware.ram_bytes == *bytes,
            HardwareEvent::Disk(bytes) => hardware.disk_bytes == *bytes,
        });
        if !consistent {
            self.invalid_preparation("硬件返回值与已完成检测项矛盾");
            return;
        }
        p.hardware = hardware;
        if p.model.catalog_selection.is_some() && p.model.variants.is_empty() {
            self.fail_preparation(ChatError {
                kind: ErrorKind::NotConnected,
                operation: Operation::Hardware,
                reason: "尚未接入该仓库版本的工件解析与部署".into(),
            });
            return;
        }
        if p.model
            .variants
            .iter()
            .any(|variant| variant.fits(&p.hardware))
        {
            p.stage = PreparationStage::SelectingVariant;
        } else {
            let reason = p
                .model
                .variants
                .iter()
                .map(|v| format!("{}：{}", v.quantization, v.shortfall(&p.hardware)))
                .collect::<Vec<_>>()
                .join("\n");
            self.fail_preparation(ChatError {
                kind: ErrorKind::InsufficientResources,
                operation: Operation::Hardware,
                reason: format!("硬件不足\n{reason}"),
            });
        }
    }
    /// 用户明确选中版本后才提供初始化输入快照。
    pub(super) fn initialization_request(&self, run_id: u64) -> Option<InitializationRequest> {
        let p = self.preparation()?;
        if p.run_id != run_id || p.stage != PreparationStage::Downloading {
            return None;
        }
        Some(InitializationRequest {
            run_id,
            model: p.model.clone(),
            variant: p.variant()?.clone(),
        })
    }
    /// 严格消费同一任务的类型化事件；下载百分比不意味着后续成功。
    pub(super) fn preparation_event(&mut self, run_id: u64, event: PreparationEvent) {
        let Some(p) = self.preparation_mut() else {
            return;
        };
        if p.run_id != run_id || !p.stage.is_running() {
            return;
        }
        if p.events.contains(&event) {
            return;
        }
        let valid = match &event {
            PreparationEvent::DownloadProgress { completed, total }
                if p.stage == PreparationStage::Downloading =>
            {
                if *completed < p.completed_bytes
                    || total.is_some_and(|total| total == 0 || *completed > total)
                    || (p.total_bytes.is_some() && p.total_bytes != *total)
                {
                    false
                } else {
                    p.completed_bytes = *completed;
                    p.total_bytes = *total;
                    true
                }
            }
            PreparationEvent::Downloaded if p.stage == PreparationStage::Downloading => {
                if p.total_bytes
                    .is_some_and(|total| total != p.completed_bytes)
                {
                    false
                } else {
                    p.stage = PreparationStage::Verifying;
                    true
                }
            }
            PreparationEvent::Verified {
                expected,
                actual,
                path,
            } if p.stage == PreparationStage::Verifying => {
                if !valid_digest(expected)
                    || expected != actual
                    || p.variant().is_none_or(|v| v.expected_sha256 != *expected)
                    || path.trim().is_empty()
                    || path.contains('\0')
                {
                    self.fail_preparation(ChatError {
                        kind: ErrorKind::VerificationFailed,
                        operation: Operation::Initialize(PreparationStage::Verifying),
                        reason: "校验失败：摘要不一致、缺失或本地路径为空".into(),
                    });
                    return;
                }
                p.actual_digest = Some(actual.clone());
                p.local_path = Some(path.clone());
                p.stage = PreparationStage::LoadingPackage;
                true
            }
            PreparationEvent::PackageLoaded if p.stage == PreparationStage::LoadingPackage => {
                p.stage = PreparationStage::Deploying;
                true
            }
            PreparationEvent::Deployed if p.stage == PreparationStage::Deploying => {
                p.stage = PreparationStage::HealthChecking;
                true
            }
            PreparationEvent::Healthy if p.stage == PreparationStage::HealthChecking => {
                p.stage = PreparationStage::Reporting;
                true
            }
            PreparationEvent::Reported if p.stage == PreparationStage::Reporting => true,
            _ => false,
        };
        if valid {
            // 下载回调频率可能很高，只保留最新进度，避免状态无限增长。
            if matches!(event, PreparationEvent::DownloadProgress { .. }) {
                p.events
                    .retain(|old| !matches!(old, PreparationEvent::DownloadProgress { .. }));
            }
            p.events.push(event);
        } else {
            self.invalid_preparation("接口事件跳过必要阶段或下载进度异常");
        }
    }
    /// 最终返回值与全部阶段凭据一致才能记录就绪并续聊一次。
    pub(super) fn finish_preparation(
        &mut self,
        run_id: u64,
        result: Result<PreparedModel, ChatError>,
    ) -> Option<GenerationPlan> {
        let p = self.preparation_mut()?;
        if p.run_id != run_id || !p.stage.is_running() {
            return None;
        }
        let ready = match result {
            Ok(value) => value,
            Err(error) => {
                self.fail_preparation(error);
                return None;
            }
        };
        if p.stage != PreparationStage::Reporting
            || !p.events.contains(&PreparationEvent::Reported)
            || !ready.is_valid()
            || ready.model != p.model
            || Some(&ready.variant) != p.variant()
            || p.actual_digest.as_ref() != Some(&ready.actual_sha256)
            || p.local_path.as_ref() != Some(&ready.local_path)
        {
            self.invalid_preparation("接口提前成功或返回了不匹配的版本、摘要、路径");
            return None;
        }
        p.stage = PreparationStage::Ready;
        self.record_ready(ready);
        if let Some(draft) = self.pending_draft.take() {
            self.draft = draft;
            return self.begin_generation();
        }
        None
    }

    fn record_ready(&mut self, ready: PreparedModel) {
        self.ready_models
            .retain(|old| !(old.model.id == ready.model.id && old.variant.id == ready.variant.id));
        self.ready_models.push(ready);
    }
    fn invalid_preparation(&mut self, reason: &str) {
        let stage = self
            .preparation()
            .map(|p| p.stage)
            .unwrap_or(PreparationStage::Failed);
        self.fail_preparation(ChatError {
            kind: ErrorKind::InvalidResult,
            operation: Operation::Initialize(stage),
            reason: reason.into(),
        });
    }

    /// 等待和终态可以关闭；运行中不关闭，避免假装取消了后台动作。
    pub(super) fn close_preparation(&mut self, run_id: u64) -> bool {
        let Some(preparation) = self.preparation_mut() else {
            return false;
        };
        if preparation.run_id != run_id || preparation.stage.is_running() {
            return false;
        }
        if matches!(
            preparation.stage,
            PreparationStage::AwaitingStart | PreparationStage::SelectingVariant
        ) {
            preparation.stage = PreparationStage::Cancelled;
        }
        preparation.collapsed = true;
        self.restore_pending_draft();
        true
    }

    fn preparation_mut(&mut self) -> Option<&mut Preparation> {
        let selected_id = self.selected_model_id.as_deref();
        let conversation_id = self.active_conversation_id.as_deref()?;
        self.conversations
            .iter_mut()
            .find(|conversation| conversation.id == conversation_id)?
            .messages
            .iter_mut()
            .rev()
            .filter_map(|message| message.preparation.as_mut())
            .find(|p| Some(p.model.id()) == selected_id)
    }

    fn fail_preparation(&mut self, error: ChatError) {
        if let Some(preparation) = self.preparation_mut() {
            preparation.stage = PreparationStage::Failed;
            preparation.local_path = None;
            preparation.error = Some(error);
        }
        self.restore_pending_draft();
    }

    fn restore_pending_draft(&mut self) {
        if let Some(draft) = self.pending_draft.take() {
            self.draft = draft;
        }
    }

    pub(super) fn begin_generation(&mut self) -> Option<GenerationPlan> {
        if !self.can_send()
            || !self
                .selected_model()
                .is_some_and(|model| self.is_model_ready(model.id()))
        {
            return None;
        }
        let selected = self.selected_model()?;
        let prepared = self
            .ready_models
            .iter()
            .find(|ready| {
                ready.model.id == selected.id && selected.variants.contains(&ready.variant)
            })
            .cloned()?;
        let reasoning_effort = self.reasoning_effort;
        let conversation_id = self.active_conversation_id.clone()?;
        let draft = std::mem::take(&mut self.draft).trim().to_string();
        let conversation = self.active_conversation_mut()?;
        if conversation.title == "新建会话" {
            conversation.title = draft
                .lines()
                .next()
                .unwrap_or("新建会话")
                .chars()
                .take(32)
                .collect();
        }
        conversation.messages.push(ChatMessage {
            role: MessageRole::User,
            content: draft,
            thinking: None,
            thinking_open: false,
            streaming: false,
            copied: false,
            metrics: None,
            error: None,
            preparation: None,
        });
        let messages = conversation
            .messages
            .iter()
            .filter(|message| {
                message.preparation.is_none()
                    && matches!(message.role, MessageRole::User | MessageRole::Assistant)
            })
            .map(|message| HistoryMessage {
                role: message.role,
                content: message.content.clone(),
            })
            .collect();
        conversation.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            thinking: None,
            thinking_open: true,
            streaming: true,
            copied: false,
            metrics: None,
            error: None,
            preparation: None,
        });
        self.is_generating = true;
        self.generation_id = self.generation_id.wrapping_add(1);
        self.close_menus();
        Some(GenerationPlan {
            conversation_id,
            generation_id: self.generation_id,
            request: Box::new(GenerationRequest {
                prepared,
                messages,
                reasoning_effort,
            }),
        })
    }

    pub(super) fn is_generation_active(&self, plan: &GenerationPlan) -> bool {
        self.is_generating
            && self.generation_id == plan.generation_id
            && self.active_conversation_id.as_deref() == Some(plan.conversation_id.as_str())
    }

    /// 正文与思考增量独立积累，旧请求或停止后的事件不再写入。
    pub(super) fn generation_event(
        &mut self,
        plan: &GenerationPlan,
        event: GenerationEvent,
    ) -> bool {
        if !self.is_generation_active(plan) {
            return false;
        }
        let Some(message) = self
            .active_conversation_mut()
            .and_then(|conversation| conversation.messages.last_mut())
        else {
            return false;
        };
        if message.role != MessageRole::Assistant || !message.streaming {
            return false;
        }
        match event {
            GenerationEvent::Content(chunk) => message.content.push_str(&chunk),
            GenerationEvent::Thinking(chunk) => message
                .thinking
                .get_or_insert_with(String::new)
                .push_str(&chunk),
        }
        true
    }
    /// 缺省指标保持缺省；错误留在原助手消息，不编造补充回复。
    pub(super) fn complete_generation(
        &mut self,
        plan: &GenerationPlan,
        result: Result<Option<InferenceMetrics>, ChatError>,
    ) -> bool {
        if !self.is_generation_active(plan) {
            return false;
        }
        let Some(message) = self
            .active_conversation_mut()
            .and_then(|conversation| conversation.messages.last_mut())
        else {
            return false;
        };
        message.streaming = false;
        match result {
            Ok(metrics) => message.metrics = metrics,
            Err(error) => message.error = Some(error),
        }
        self.is_generating = false;
        true
    }

    pub(super) fn interrupt_generation(&mut self) {
        if !self.is_generating {
            return;
        }
        self.is_generating = false;
        self.generation_id = self.generation_id.wrapping_add(1);
        if let Some(conversation) = self.active_conversation_mut() {
            if let Some(message) = conversation.messages.last_mut() {
                if message.role == MessageRole::Assistant {
                    message.streaming = false;
                }
            }
        }
    }

    pub(super) fn mark_message_copied(
        &mut self,
        conversation_id: &str,
        message_index: usize,
        copied: bool,
    ) -> bool {
        let Some(conversation) = self
            .conversations
            .iter_mut()
            .find(|conversation| conversation.id == conversation_id)
        else {
            return false;
        };
        let Some(message) = conversation.messages.get_mut(message_index) else {
            return false;
        };
        if message.role != MessageRole::Assistant {
            return false;
        }
        message.copied = copied;
        true
    }

    fn active_conversation_mut(&mut self) -> Option<&mut ChatConversation> {
        let id = self.active_conversation_id.as_deref()?;
        self.conversations
            .iter_mut()
            .find(|conversation| conversation.id == id)
    }
}

#[cfg(test)]
#[path = "state_test_support.rs"]
mod test_support;
