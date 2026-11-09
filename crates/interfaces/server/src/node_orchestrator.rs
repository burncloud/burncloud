// 从 burncloud_node_runtime crate 引入节点运行时相关的所有核心抽象
use burncloud_node_runtime::{
    // 工件准备 trait（负责下载/校验模型文件）
    ArtifactPreparation,
    // 需求协调器：状态机，驱动单个模型的生命周期收敛
    DemandReconciler,
    // 摘要类型（如 sha256:...），用于校验工件完整性
    Digest,
    // 硬件探测 trait（探测 GPU/CPU/内存）
    HardwareProbe,
    // 健康探测 trait
    HealthProbe,
    // 节点组合：把硬件、工件、运行时、进程、就绪、健康等能力聚合在一起
    NodeComposition,
    // 节点状态枚举（Resolved / PreparingArtifact / Ready / Routable / Failed 等）
    NodeState,
    // 已准备好的工件（路径 + 验证状态）
    PreparedArtifact,
    // 已准备好的运行时（可执行文件路径）
    PreparedRuntime,
    // 进程句柄（持有已启动进程的控制权）
    ProcessHandle,
    // 进程管理 trait（启动/停止进程）
    ProcessManager,
    // 进程计划（如何启动进程的完整描述，含 readiness target）
    ProcessPlan,
    // 就绪探测 trait
    ReadinessProbe,
    // 协调动作枚举（Resolve / PrepareArtifact / StartProcess / AttachToExistingRouter 等）
    ReconcileAction,
    // 协调证据枚举（Resolved / ArtifactPrepared / ProcessStarted / Failed 等）
    ReconcileEvidence,
    // 运行时适配器 trait（为特定运行时生成 ProcessPlan）
    RuntimeAdapter,
    // 运行时准备器 trait（下载/安装运行时二进制）
    RuntimePreparer,
    // 运行时请求（运行时名称 + 版本）
    RuntimeRequest,
};
// 引入本地路由附件 ID（标识模型在 Traffic 路由器中的挂载点）
use burncloud_router::local_attachment::LocalRouteAttachmentId;
// 引入模型供给解析相关类型
use burncloud_supply_model::{
    // 本地模型不支持的描述（模型 + 原因）
    LocalModelUnsupported,
    // 模型解析结果（Local 或 Unsupported）
    ModelResolutionOutcome,
    // 模型解析请求（模型名 + 可用加速器内存）
    ModelResolutionRequest,
    // 模型解析器 trait
    ModelResolver,
    // 已解析的模型（含工件地址、摘要、运行时等）
    ResolvedModel,
};
// 引入 HashMap（按模型名索引 workload）
use std::collections::HashMap;
// 引入 fmt（用于实现 Display）
use std::fmt;

// 引入 NodeRequestState（请求层可见的状态投影，用于 route_miss 判断）
use crate::node_request::NodeRequestState;

// 一个模型部署需求：只包含模型名
// Debug/Clone/PartialEq/Eq：便于调试、克隆、比较
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelDemand {
    // 请求的模型名
    pub model: String,
}

// 为 ModelDemand 实现方法
impl ModelDemand {
    // 构造并校验模型需求
    pub fn new(model: impl Into<String>) -> Result<Self, ModelDemandError> {
        // 转成 String
        let model = model.into();
        // 去除首尾空白
        let model = model.trim();
        // 空字符串拒绝
        if model.is_empty() {
            return Err(ModelDemandError::EmptyModel);
        }
        // 构造需求（注意：这里 model.into() 又把 &str 转成 String，
        // 因为上面 trim() 返回 &str 借用自局部 String）
        Ok(Self {
            model: model.into(),
        })
    }
}

// 模型需求错误枚举
// Debug/Clone/Copy/PartialEq/Eq：便于调试和比较
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelDemandError {
    // 模型名为空
    EmptyModel,
}

// 为 ModelDemandError 实现 Display（用户可读错误信息）
impl fmt::Display for ModelDemandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("model demand must name a model")
    }
}

// 实现标准 Error trait，使其可被 ? 传播和 anyhow 转换
impl std::error::Error for ModelDemandError {}

/// Result of the optional local-supply preparation rail.
/// Unsupported means "do nothing locally"; it must never become a routing error.
// 本地供给准备的结果
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LocalPreparationOutcome {
    // 已就绪，等待挂载路由
    Ready,
    // 已挂载路由（携带附件 ID）
    Routable(LocalRouteAttachmentId),
    // 本机不支持（Supply 层判定），不报错，直接放弃本地供给
    Unsupported(LocalModelUnsupported),
}

/// Application-owned lifecycle for exactly one requested model.
///
/// The machine capabilities are shared, but convergence truth is not: every
/// model owns an independent reconciler, process plan, process receipt and
/// Traffic attachment identity.
// 单个模型的工作负载：记录该模型收敛过程中的所有"回执"（receipt）
// Debug：便于调试；Default：可默认构造（全部为 None/空）
#[derive(Debug, Default)]
struct ModelWorkload {
    // 该模型独立的状态机协调器
    reconciler: DemandReconciler,
    // 解析结果回执
    resolved: Option<ResolvedModel>,
    // 工件准备回执
    artifact: Option<PreparedArtifact>,
    // 运行时准备回执
    runtime: Option<PreparedRuntime>,
    // 进程计划回执
    plan: Option<ProcessPlan>,
    // 进程句柄回执（持有进程控制权）
    process: Option<ProcessHandle>,
    // 当前挂载的路由附件 ID
    attachment_id: Option<LocalRouteAttachmentId>,
    // 已摘除（但尚未完成恢复）的路由附件 ID
    detached_attachment_id: Option<LocalRouteAttachmentId>,
    // 不支持原因回执（Supply 判定本机不可运行该模型时记录）
    unsupported: Option<LocalModelUnsupported>,
}

// 节点编排器：协调"解析 → 准备工件 → 准备运行时 → 启动进程 → 验证就绪 → 挂载路由"的全流程
// 泛型参数：
//   M: ModelResolver      模型解析器
//   T: RuntimeAdapter     运行时适配器
//   H: HardwareProbe      硬件探测
//   A: ArtifactPreparation 工件准备
//   R: RuntimePreparer    运行时准备
//   P: ProcessManager     进程管理
//   Q: ReadinessProbe     就绪探测
//   E: HealthProbe        健康探测
#[derive(Debug)]
pub struct NodeOrchestrator<M, T, H, A, R, P, Q, E> {
    // 模型解析器
    resolver: M,
    // 运行时适配器
    runtime_adapter: T,
    // 机器能力组合（共享的硬件/工件/运行时/进程/探测能力）
    machine: NodeComposition<H, A, R, P, Q, E>,
    // 所有模型的工作负载（按模型名索引）
    workloads: HashMap<String, ModelWorkload>,
    // 请求层可见的状态投影（用于 route_miss 判断）
    request_state: NodeRequestState,
}

// 为 NodeOrchestrator 实现方法，约束各泛型参数必须实现对应 trait
impl<M, T, H, A, R, P, Q, E> NodeOrchestrator<M, T, H, A, R, P, Q, E>
where
    // 模型解析器
    M: ModelResolver,
    // 运行时适配器
    T: RuntimeAdapter,
    // 硬件探测
    H: HardwareProbe,
    // 工件准备
    A: ArtifactPreparation,
    // 运行时准备
    R: RuntimePreparer,
    // 进程管理
    P: ProcessManager,
    // 就绪探测
    Q: ReadinessProbe,
    // 健康探测
    E: HealthProbe,
{
    // 构造函数
    pub fn new(
        resolver: M,
        runtime_adapter: T,
        machine: NodeComposition<H, A, R, P, Q, E>,
    ) -> Self {
        Self {
            resolver,
            runtime_adapter,
            machine,
            // 初始无任何 workload
            workloads: HashMap::new(),
            // 默认请求状态
            request_state: NodeRequestState::default(),
        }
    }

    // 构建器模式：注入自定义 NodeRequestState（便于测试）
    pub fn with_request_state(mut self, request_state: NodeRequestState) -> Self {
        self.request_state = request_state;
        self
    }

    // 获取请求状态的克隆（供外部使用）
    pub fn request_state(&self) -> NodeRequestState {
        self.request_state.clone()
    }

    // 获取解析器引用（const fn，编译期可求值场景优化）
    pub const fn resolver(&self) -> &M {
        &self.resolver
    }

    // 获取机器组合引用
    pub const fn machine(&self) -> &NodeComposition<H, A, R, P, Q, E> {
        &self.machine
    }

    // 获取机器组合可变引用
    pub fn machine_mut(&mut self) -> &mut NodeComposition<H, A, R, P, Q, E> {
        &mut self.machine
    }

    // 当前 workload 数量
    pub fn workload_count(&self) -> usize {
        self.workloads.len()
    }

    // 查询某模型的状态（不存在返回 None）
    pub fn workload_state(&self, model: &str) -> Option<NodeState> {
        self.workloads
            .get(model)
            // 取协调器的当前状态
            .map(|workload| workload.reconciler.state())
    }

    // 查询某模型的进程计划
    pub fn workload_plan(&self, model: &str) -> Option<&ProcessPlan> {
        // 若模型不存在返回 None；否则取 plan 的引用
        self.workloads.get(model)?.plan.as_ref()
    }

    // 查询某模型的挂载附件 ID
    pub fn workload_attachment_id(&self, model: &str) -> Option<LocalRouteAttachmentId> {
        // 若模型不存在返回 None；否则取 attachment_id（Option<Copy> 直接复制）
        self.workloads.get(model)?.attachment_id
    }

    // 确保某模型有 workload 条目（不存在则用 Default 创建）
    fn ensure_workload(&mut self, model: &str) {
        self.workloads.entry(model.to_string()).or_default();
    }

    // 把某模型的当前状态发布到请求状态投影
    fn publish_state(&self, model: &str) {
        // 若有状态则发布
        if let Some(state) = self.workload_state(model) {
            self.request_state.publish(model, state);
        }
    }

    // 推进状态机：请求下一个动作
    fn next_action(&mut self, model: &str) -> anyhow::Result<ReconcileAction> {
        // 确保 workload 存在
        self.ensure_workload(model);
        // 取可变引用，调用协调器的 next_action
        let action = self
            .workloads
            .get_mut(model)
            // 注：这里用 expect 是因为 ensure_workload 刚保证存在，
            // 属于逻辑不变量（见文件顶部 clippy 豁免的 reason）
            .expect("workload must exist")
            .reconciler
            .next_action()?;
        // 发布新状态
        self.publish_state(model);
        Ok(action)
    }

    // 向协调器提交证据（推进状态机）
    fn observe(&mut self, model: &str, evidence: ReconcileEvidence) -> anyhow::Result<()> {
        // 确保 workload 存在
        self.ensure_workload(model);
        // 提交证据
        self.workloads
            .get_mut(model)
            .expect("workload must exist")
            .reconciler
            .observe(evidence)?;
        // 发布新状态
        self.publish_state(model);
        Ok(())
    }

    // 标记某模型失败（crate 内可见）
    pub(crate) fn mark_failed(&mut self, model: &str) -> anyhow::Result<()> {
        // 提交 Failed 证据
        self.observe(model, ReconcileEvidence::Failed)
    }

    // 挂载前的就绪性校验
    pub(crate) fn ensure_ready_for_attachment(&self, model: &str) -> anyhow::Result<()> {
        // 取 workload，不存在报错
        let workload = self
            .workloads
            .get(model)
            .ok_or_else(|| anyhow::anyhow!("workload missing for model '{model}'"))?;

        // 状态必须是 Ready
        if workload.reconciler.state() != NodeState::Ready {
            anyhow::bail!(
                "model '{model}' is not ready for attachment: {:?}",
                workload.reconciler.state()
            );
        }
        // 不能已经挂载过
        if workload.attachment_id.is_some() {
            anyhow::bail!("model '{model}' already owns a route attachment");
        }
        // 必须有进程计划（Ready 状态下的不变量）
        if workload.plan.is_none() {
            anyhow::bail!("model '{model}' is Ready without a process plan");
        }
        Ok(())
    }

    // 记录路由附件挂载（crate 内可见）
    pub(crate) fn record_attachment(
        &mut self,
        model: &str,
        attachment_id: LocalRouteAttachmentId,
    ) -> anyhow::Result<()> {
        // 先校验前置条件
        self.ensure_ready_for_attachment(model)?;

        // Store the Traffic receipt before publishing Routable. This method is
        // synchronous, so no observer can see a Routable request projection
        // without the workload already owning the exact attachment identity.
        // 用块限制可变借用的作用域
        {
            let workload = self.workloads.get_mut(model).expect("workload must exist");
            // 记录附件 ID
            workload.attachment_id = Some(attachment_id);
            // 清空"已摘除"回执
            workload.detached_attachment_id = None;
            // 向协调器提交 RouterAttached 证据，推进到 Routable
            if let Err(error) = workload
                .reconciler
                .observe(ReconcileEvidence::RouterAttached)
            {
                // 若失败，回滚 attachment_id，保持状态一致
                workload.attachment_id = None;
                return Err(error.into());
            }
        }

        // 发布新状态（此时状态为 Routable）
        self.publish_state(model);
        Ok(())
    }

    // 标记某模型不健康（crate 内可见）
    pub(crate) fn mark_unhealthy(&mut self, model: &str) -> anyhow::Result<()> {
        // 提交 BecameUnhealthy 证据
        self.observe(model, ReconcileEvidence::BecameUnhealthy)
    }

    // 清除路由附件（摘除挂载，crate 内可见）
    pub(crate) fn clear_attachment(
        &mut self,
        model: &str,
        attachment_id: LocalRouteAttachmentId,
    ) -> anyhow::Result<()> {
        // 取可变 workload
        let workload = self
            .workloads
            .get_mut(model)
            .ok_or_else(|| anyhow::anyhow!("workload missing for model '{model}'"))?;
        // 校验附件 ID 匹配（防止误摘除）
        if workload.attachment_id != Some(attachment_id) {
            anyhow::bail!(
                "attachment mismatch for model '{model}': expected {:?}, got {:?}",
                workload.attachment_id,
                attachment_id
            );
        }
        // 清空当前附件
        workload.attachment_id = None;
        // 记录"已摘除"回执，供恢复流程校验
        workload.detached_attachment_id = Some(attachment_id);
        Ok(())
    }

    // 开始恢复流程（crate 内可见）
    pub(crate) fn begin_recovery(
        &mut self,
        model: &str,
        detached_attachment_id: LocalRouteAttachmentId,
    ) -> anyhow::Result<()> {
        // 取可变 workload
        let workload = self
            .workloads
            .get_mut(model)
            .ok_or_else(|| anyhow::anyhow!("workload missing for model '{model}'"))?;

        // 校验"已摘除"回执匹配
        if workload.detached_attachment_id != Some(detached_attachment_id) {
            anyhow::bail!(
                "detached route receipt mismatch for model '{model}': expected {:?}, got {:?}",
                workload.detached_attachment_id,
                detached_attachment_id
            );
        }

        // 协调器重试（从失败/不健康状态回到可重试状态）
        workload.reconciler.retry()?;
        // 清空"已摘除"回执
        workload.detached_attachment_id = None;
        // 发布新状态
        self.publish_state(model);
        Ok(())
    }

    /// Prepare an optional local supply. If Supply says the machine cannot run
    /// the model, stop immediately: no artifact, runtime, process or route work
    /// is performed. The existing request/router path is deliberately untouched.
    // 主流程：准备直到就绪（或返回不支持/已挂载）
    pub async fn prepare_until_ready(
        &mut self,
        demand: ModelDemand,
    ) -> anyhow::Result<LocalPreparationOutcome> {
        // 确保 workload 存在
        self.ensure_workload(&demand.model);

        // 循环推进状态机，直到返回终态
        loop {
            // 请求下一个动作
            let action = self.next_action(&demand.model)?;
            // 根据动作分支处理
            match action {
                // 动作：解析模型
                ReconcileAction::Resolve => {
                    // 探测硬件
                    let hardware = match self.machine.hardware().inspect().await {
                        Ok(hardware) => hardware,
                        Err(error) => {
                            // 失败则标记 Failed 并返回错误
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    // 取所有加速器中最大的显存（用于解析时选择变体）
                    let accelerator_memory_bytes = hardware
                        .accelerators
                        .iter()
                        .filter_map(|a| a.memory_bytes)
                        .max();
                    // 调用解析器
                    match self
                        .resolver
                        .resolve(ModelResolutionRequest {
                            model: demand.model.clone(),
                            accelerator_memory_bytes,
                        })
                        .await
                    {
                        // 解析成功：记录 resolved 回执并提交证据
                        Ok(ModelResolutionOutcome::Local(resolved)) => {
                            self.workloads
                                .get_mut(&demand.model)
                                .expect("workload must exist")
                                .resolved = Some(resolved);
                            self.observe(&demand.model, ReconcileEvidence::Resolved)?;
                        }
                        // Supply 判定本机不支持：记录原因，提交证据，直接返回 Unsupported
                        Ok(ModelResolutionOutcome::Unsupported(reason)) => {
                            self.workloads
                                .get_mut(&demand.model)
                                .expect("workload must exist")
                                .unsupported = Some(reason.clone());
                            self.observe(&demand.model, ReconcileEvidence::LocalUnsupported)?;
                            return Ok(LocalPreparationOutcome::Unsupported(reason));
                        }
                        // 解析出错：标记 Failed 并返回
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    }
                }
                // 动作：准备工件（下载/校验模型文件）
                ReconcileAction::PrepareArtifact => {
                    // 取 resolved 回执（缺失则报错）
                    let resolved = self
                        .workloads
                        .get(&demand.model)
                        .and_then(|workload| workload.resolved.clone())
                        .ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    // 解析工件 URL
                    let artifact_url = match reqwest::Url::parse(&resolved.artifact_source) {
                        Ok(url) => url,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(anyhow::anyhow!(
                                "invalid artifact source '{}': {error}",
                                resolved.artifact_source
                            ));
                        }
                    };
                    // 解析期望摘要（必须存在且合法）
                    let expected_digest = match resolved.artifact_digest {
                        Some(value) => match Digest::parse(&value) {
                            Ok(digest) => digest,
                            Err(error) => {
                                self.mark_failed(&demand.model)?;
                                return Err(anyhow::anyhow!(
                                    "invalid artifact digest '{}': {error}",
                                    value
                                ));
                            }
                        },
                        None => {
                            self.mark_failed(&demand.model)?;
                            anyhow::bail!("artifact digest is required");
                        }
                    };
                    // 调用工件准备器下载并校验
                    let artifact = match self
                        .machine
                        .artifacts()
                        .prepare(artifact_url, expected_digest)
                        .await
                    {
                        Ok(artifact) => artifact,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    // 工件必须已验证
                    if !artifact.is_verified() {
                        self.mark_failed(&demand.model)?;
                        anyhow::bail!("artifact preparer returned unverified artifact");
                    }
                    // 记录工件回执
                    self.workloads
                        .get_mut(&demand.model)
                        .expect("workload must exist")
                        .artifact = Some(artifact);
                    // 提交证据
                    self.observe(&demand.model, ReconcileEvidence::ArtifactPrepared)?;
                }
                // 动作：准备运行时（下载/安装 llama.cpp 等）
                ReconcileAction::PrepareRuntime => {
                    // 取 resolved 回执
                    let resolved = self
                        .workloads
                        .get(&demand.model)
                        .and_then(|workload| workload.resolved.clone())
                        .ok_or_else(|| anyhow::anyhow!("resolved model receipt missing"))?;
                    // 调用运行时准备器
                    let runtime = match self
                        .machine
                        .runtimes()
                        .prepare(RuntimeRequest {
                            runtime: resolved.runtime,
                            version: resolved.runtime_version,
                        })
                        .await
                    {
                        Ok(runtime) => runtime,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    // 记录运行时回执
                    self.workloads
                        .get_mut(&demand.model)
                        .expect("workload must exist")
                        .runtime = Some(runtime);
                    // 提交证据
                    self.observe(&demand.model, ReconcileEvidence::RuntimePrepared)?;
                }
                // 动作：启动进程
                ReconcileAction::StartProcess => {
                    // 同时取运行时和工件回执
                    let (runtime, artifact) = self
                        .workloads
                        .get(&demand.model)
                        .map(|workload| (workload.runtime.clone(), workload.artifact.clone()))
                        .ok_or_else(|| {
                            anyhow::anyhow!("workload missing for model '{}'", demand.model)
                        })?;
                    // 两者都必须存在
                    let runtime =
                        runtime.ok_or_else(|| anyhow::anyhow!("runtime receipt missing"))?;
                    let artifact =
                        artifact.ok_or_else(|| anyhow::anyhow!("artifact receipt missing"))?;

                    // 运行时适配器生成启动计划
                    let plan = match self.runtime_adapter.plan(&runtime, &artifact).await {
                        Ok(plan) => plan,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };

                    // Persist the launch receipt before awaiting process start.
                    // If this orchestration call is cancelled while start() is
                    // pending, a later reconcile call still has all framework
                    // evidence needed to continue from Starting.
                    // 先持久化计划回执（关键：即使 start() 被取消，后续也能从 Starting 继续）
                    self.workloads
                        .get_mut(&demand.model)
                        .expect("workload must exist")
                        .plan = Some(plan.clone());

                    // 启动进程
                    let handle = match self.machine.processes().start(plan.process.clone()).await {
                        Ok(handle) => handle,
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    };
                    // 记录进程句柄
                    self.workloads
                        .get_mut(&demand.model)
                        .expect("workload must exist")
                        .process = Some(handle);
                    // 提交证据
                    self.observe(&demand.model, ReconcileEvidence::ProcessStarted)?;
                }
                // 动作：验证就绪
                ReconcileAction::VerifyReadiness => {
                    // 从计划中取就绪探测目标
                    let target = self
                        .workload_plan(&demand.model)
                        .map(|plan| plan.readiness.clone())
                        .ok_or_else(|| anyhow::anyhow!("readiness target receipt missing"))?;
                    // 等待就绪（带重试，直到超时）
                    if let Err(error) = self.machine.readiness().wait_ready(target.clone()).await {
                        self.mark_failed(&demand.model)?;
                        return Err(error.into());
                    }
                    // 再做一次健康检查
                    match self.machine.health().is_healthy(target).await {
                        Ok(true) => {}
                        Ok(false) => {
                            self.mark_failed(&demand.model)?;
                            anyhow::bail!("runtime is ready but unhealthy");
                        }
                        Err(error) => {
                            self.mark_failed(&demand.model)?;
                            return Err(error.into());
                        }
                    }
                    // 提交就绪证据
                    self.observe(&demand.model, ReconcileEvidence::ReadinessVerified)?;
                }
                // 动作：挂载到现有路由器（说明已就绪，直接返回 Ready）
                ReconcileAction::AttachToExistingRouter => {
                    return Ok(LocalPreparationOutcome::Ready)
                }
                // 动作：无操作（终态）
                ReconcileAction::Noop => match self.workload_state(&demand.model) {
                    // 已可路由：返回附件 ID
                    Some(NodeState::Routable) => {
                        let attachment_id =
                            self.workload_attachment_id(&demand.model).ok_or_else(|| {
                                anyhow::anyhow!(
                                    "routable model '{}' has no route attachment receipt",
                                    demand.model
                                )
                            })?;
                        return Ok(LocalPreparationOutcome::Routable(attachment_id));
                    }
                    // 不支持：返回不支持原因
                    Some(NodeState::LocalUnsupported) => {
                        let reason = self
                            .workloads
                            .get(&demand.model)
                            .and_then(|workload| workload.unsupported.clone())
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "unsupported model '{}' has no unsupported receipt",
                                    demand.model
                                )
                            })?;
                        return Ok(LocalPreparationOutcome::Unsupported(reason));
                    }
                    // 其他状态：报错（不应该出现）
                    state => anyhow::bail!(
                        "unexpected no-op state for model '{}': {state:?}",
                        demand.model
                    ),
                },
                // 动作：需要显式恢复（prepare_until_ready 不处理）
                ReconcileAction::Recover => {
                    anyhow::bail!("explicit recovery is required for model '{}'", demand.model)
                }
            }
        }
    }

    // 恢复流程：从失败/不健康状态恢复到就绪
    pub async fn recover_until_ready(&mut self, model: &str) -> anyhow::Result<()> {
        // 取进程计划（恢复必须已有计划）
        let plan = self
            .workload_plan(model)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("process plan missing for model '{model}'"))?;

        // 循环推进状态机，只接受恢复相关动作
        loop {
            let action = self.next_action(model)?;
            match action {
                // 重新启动进程
                ReconcileAction::StartProcess => {
                    let handle = match self.machine.processes().start(plan.process.clone()).await {
                        Ok(handle) => handle,
                        Err(error) => {
                            self.mark_failed(model)?;
                            return Err(error.into());
                        }
                    };
                    self.workloads
                        .get_mut(model)
                        .expect("workload must exist")
                        .process = Some(handle);
                    self.observe(model, ReconcileEvidence::ProcessStarted)?;
                }
                // 重新验证就绪
                ReconcileAction::VerifyReadiness => {
                    let target = plan.readiness.clone();
                    if let Err(error) = self.machine.readiness().wait_ready(target.clone()).await {
                        self.mark_failed(model)?;
                        return Err(error.into());
                    }
                    match self.machine.health().is_healthy(target).await {
                        Ok(true) => {}
                        Ok(false) => {
                            self.mark_failed(model)?;
                            anyhow::bail!("recovered runtime is unhealthy");
                        }
                        Err(error) => {
                            self.mark_failed(model)?;
                            return Err(error.into());
                        }
                    }
                    self.observe(model, ReconcileEvidence::ReadinessVerified)?;
                }
                // 挂载到现有路由器：恢复完成
                ReconcileAction::AttachToExistingRouter => return Ok(()),
                // 其他动作：报错（恢复流程不应出现）
                other => anyhow::bail!("unexpected recovery action for '{model}': {other:?}"),
            }
        }
    }
}

// 测试模块
#[cfg(test)]
mod tests {
    // 引入父模块所有内容
    use super::*;
    // 引入 async_trait
    use async_trait::async_trait;
    // 引入各种 Fake 实现（用于测试）
    use burncloud_node_runtime::{
        ArtifactVerificationStatus, FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe,
        FakeProcessManager, FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer,
    };
    use burncloud_supply_model::{
        FakeModelResolver, LocalModelUnsupportedReason, ModelResolutionError,
    };

    // 构造一个已关闭的监控错误通道（测试用，不需要真正监控）
    fn closed_monitor_errors() -> tokio::sync::mpsc::UnboundedReceiver<
        burncloud_node_runtime::deployment::DeploymentMonitorError,
    > {
        // 创建通道但立即丢弃发送端，使接收端永远为空
        let (_, receiver) = tokio::sync::mpsc::unbounded_channel();
        receiver
    }

    // 构造一个全 Fake 的编排器（用于大部分测试）
    fn fake_orchestrator() -> NodeOrchestrator<
        FakeModelResolver,
        FakeRuntimeAdapter,
        FakeHardwareProbe,
        FakeArtifactPreparer,
        FakeRuntimePreparer,
        FakeProcessManager,
        FakeReadinessProbe,
        FakeHealthProbe,
    > {
        NodeOrchestrator::new(
            FakeModelResolver,
            FakeRuntimeAdapter,
            NodeComposition::start(
                FakeHardwareProbe::default(),
                FakeArtifactPreparer,
                FakeRuntimePreparer,
                std::sync::Arc::new(FakeProcessManager::default()),
                std::sync::Arc::new(FakeReadinessProbe),
                std::sync::Arc::new(FakeHealthProbe),
                closed_monitor_errors(),
            ),
        )
    }

    // 总是返回"不支持"的解析器
    #[derive(Debug)]
    struct UnsupportedResolver;

    // 总是返回错误的解析器
    #[derive(Debug)]
    struct FailingResolver;

    // 为 FailingResolver 实现 ModelResolver
    #[async_trait]
    impl ModelResolver for FailingResolver {
        async fn resolve(
            &self,
            _request: ModelResolutionRequest,
        ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
            Err(ModelResolutionError::ResolutionFailed(
                "resolver offline".into(),
            ))
        }
    }

    // 为 UnsupportedResolver 实现 ModelResolver
    #[async_trait]
    impl ModelResolver for UnsupportedResolver {
        async fn resolve(
            &self,
            request: ModelResolutionRequest,
        ) -> Result<ModelResolutionOutcome, ModelResolutionError> {
            Ok(ModelResolutionOutcome::Unsupported(LocalModelUnsupported {
                model: request.model,
                reason: LocalModelUnsupportedReason::NoCompatibleVariant,
            }))
        }
    }

    // 测试：ModelDemand 只包含请求的模型（去除空白）
    #[test]
    fn demand_contains_only_the_requested_model() {
        assert_eq!(ModelDemand::new(" qwen-4b ").unwrap().model, "qwen-4b");
        assert_eq!(ModelDemand::new("   "), Err(ModelDemandError::EmptyModel));
    }

    // 测试：解析失败时收敛到 Failed，且不再报告 Preparing
    #[tokio::test]
    async fn failed_resolution_converges_to_failed_and_stops_reporting_preparing() {
        let request_state = NodeRequestState::default();
        let machine = NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            std::sync::Arc::new(FakeProcessManager::default()),
            std::sync::Arc::new(FakeReadinessProbe),
            std::sync::Arc::new(FakeHealthProbe),
            closed_monitor_errors(),
        );
        let mut orchestrator = NodeOrchestrator::new(FailingResolver, FakeRuntimeAdapter, machine)
            .with_request_state(request_state.clone());

        // prepare 应返回 Err
        assert!(orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .is_err());
        // 状态应为 Failed
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Failed)
        );
        // route_miss 不应再返回响应（因为不是 Preparing 状态）
        assert!(request_state.route_miss_response("qwen-4b").is_none());
    }

    // 测试：支持的模型能走到 Ready
    #[tokio::test]
    async fn supported_demand_reaches_ready() {
        let mut orchestrator = fake_orchestrator();
        // prepare 返回 Ready
        assert_eq!(
            orchestrator
                .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
                .await
                .unwrap(),
            LocalPreparationOutcome::Ready
        );
        // 状态为 Ready
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Ready)
        );
        // 进程计划的本地端点为预期值
        assert_eq!(
            orchestrator
                .workload_plan("qwen-4b")
                .unwrap()
                .local_endpoint,
            "http://127.0.0.1:39122"
        );
    }

    // 测试：两个模型拥有独立 workload
    #[tokio::test]
    async fn two_models_own_independent_workloads() {
        let mut orchestrator = fake_orchestrator();

        // 第一个模型就绪并挂载路由
        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        orchestrator
            .record_attachment("qwen-4b", LocalRouteAttachmentId(101))
            .unwrap();

        // 记录 qwen 的计划
        let qwen_plan = orchestrator.workload_plan("qwen-4b").cloned().unwrap();
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );

        // 第二个模型就绪
        orchestrator
            .prepare_until_ready(ModelDemand::new("deepseek-8b").unwrap())
            .await
            .unwrap();

        // 两个 workload 互不影响
        assert_eq!(orchestrator.workload_count(), 2);
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(101))
        );
        assert_eq!(orchestrator.workload_plan("qwen-4b"), Some(&qwen_plan));
        assert_eq!(
            orchestrator.workload_state("deepseek-8b"),
            Some(NodeState::Ready)
        );
        assert_eq!(orchestrator.workload_attachment_id("deepseek-8b"), None);
    }

    // 测试：重复请求同一模型复用同一个 workload
    #[tokio::test]
    async fn repeated_same_model_demand_reuses_one_workload() {
        let mut orchestrator = fake_orchestrator();

        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        assert_eq!(orchestrator.workload_count(), 1);

        // 再次请求同一模型
        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();

        // 仍然只有一个 workload
        assert_eq!(orchestrator.workload_count(), 1);
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Ready)
        );
    }

    // 测试：重复请求已挂载的模型返回现有附件 ID
    #[tokio::test]
    async fn repeated_routable_demand_returns_existing_attachment() {
        let mut orchestrator = fake_orchestrator();

        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        orchestrator
            .record_attachment("qwen-4b", LocalRouteAttachmentId(101))
            .unwrap();

        // 再次 prepare，应返回 Routable(101)
        let outcome = orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();

        assert_eq!(
            outcome,
            LocalPreparationOutcome::Routable(LocalRouteAttachmentId(101))
        );
        assert_eq!(orchestrator.workload_count(), 1);
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(101))
        );
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
    }

    // 测试：工件准备中断后，复用已持久化的解析回执继续
    #[tokio::test]
    async fn interrupted_preparing_artifact_reuses_persisted_resolution_receipt() {
        let mut orchestrator = fake_orchestrator();
        let model = "qwen-4b";

        // 手动推进：第一个动作是 Resolve
        assert_eq!(
            orchestrator.next_action(model).unwrap(),
            ReconcileAction::Resolve
        );
        // 手动注入 resolved 回执
        orchestrator
            .workloads
            .get_mut(model)
            .expect("workload")
            .resolved = Some(ResolvedModel {
            model: model.into(),
            artifact_source: "https://modelscope.cn/models/qwen-4b/resolve/main/fake.gguf".into(),
            artifact_digest: Some(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000".into(),
            ),
            runtime: "llama.cpp".into(),
            runtime_version: Some("fake-v0".into()),
        });
        // 提交 Resolved 证据
        orchestrator
            .observe(model, ReconcileEvidence::Resolved)
            .unwrap();

        // 状态应为 PreparingArtifact（模拟中断点）
        assert_eq!(
            orchestrator.workload_state(model),
            Some(NodeState::PreparingArtifact)
        );

        // A new orchestration call must continue from the workload receipt,
        // not depend on a previous function-local variable.
        // 新的 prepare 调用应从回执继续，最终走到 Ready
        assert_eq!(
            orchestrator
                .prepare_until_ready(ModelDemand::new(model).unwrap())
                .await
                .unwrap(),
            LocalPreparationOutcome::Ready
        );
        assert_eq!(orchestrator.workload_state(model), Some(NodeState::Ready));
    }

    // 测试：运行时准备中断后，复用已持久化的工件回执继续
    #[tokio::test]
    async fn interrupted_preparing_runtime_reuses_persisted_artifact_receipt() {
        let mut orchestrator = fake_orchestrator();
        let model = "qwen-4b";

        // 推进到 Resolve
        orchestrator.next_action(model).unwrap();
        // 注入 resolved 回执
        {
            let workload = orchestrator.workloads.get_mut(model).expect("workload");
            workload.resolved = Some(ResolvedModel {
                model: model.into(),
                artifact_source: "https://modelscope.cn/models/qwen-4b/resolve/main/fake.gguf"
                    .into(),
                artifact_digest: Some(
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .into(),
                ),
                runtime: "llama.cpp".into(),
                runtime_version: Some("fake-v0".into()),
            });
        }
        // 提交 Resolved 证据
        orchestrator
            .observe(model, ReconcileEvidence::Resolved)
            .unwrap();
        // 注入工件回执
        orchestrator
            .workloads
            .get_mut(model)
            .expect("workload")
            .artifact = Some(
            PreparedArtifact::new(
                std::env::temp_dir().join("burncloud/fake/artifacts/qwen_fake.gguf"),
                // 测试回执必须明确表示已完成摘要校验并通过。
                ArtifactVerificationStatus::Verified,
            )
            .unwrap(),
        );
        // 提交 ArtifactPrepared 证据
        orchestrator
            .observe(model, ReconcileEvidence::ArtifactPrepared)
            .unwrap();

        // 下一个动作应为 PrepareRuntime
        assert_eq!(
            orchestrator.next_action(model).unwrap(),
            ReconcileAction::PrepareRuntime
        );
        // 状态为 PreparingRuntime（模拟中断点）
        assert_eq!(
            orchestrator.workload_state(model),
            Some(NodeState::PreparingRuntime)
        );

        // 新的 prepare 从回执继续，最终 Ready
        assert_eq!(
            orchestrator
                .prepare_until_ready(ModelDemand::new(model).unwrap())
                .await
                .unwrap(),
            LocalPreparationOutcome::Ready
        );
        assert_eq!(orchestrator.workload_state(model), Some(NodeState::Ready));
    }

    // 测试：启动中断后，复用已持久化的运行时和工件回执继续
    #[tokio::test]
    async fn interrupted_starting_reuses_persisted_runtime_and_artifact_receipts() {
        let mut orchestrator = fake_orchestrator();
        let model = "qwen-4b";

        // 推进到 Resolve
        orchestrator.next_action(model).unwrap();
        // 注入 resolved 回执
        {
            let workload = orchestrator.workloads.get_mut(model).expect("workload");
            workload.resolved = Some(ResolvedModel {
                model: model.into(),
                artifact_source: "https://modelscope.cn/models/qwen-4b/resolve/main/fake.gguf"
                    .into(),
                artifact_digest: Some(
                    "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                        .into(),
                ),
                runtime: "llama.cpp".into(),
                runtime_version: Some("fake-v0".into()),
            });
        }
        orchestrator
            .observe(model, ReconcileEvidence::Resolved)
            .unwrap();
        // 注入工件回执
        orchestrator
            .workloads
            .get_mut(model)
            .expect("workload")
            .artifact = Some(
            PreparedArtifact::new(
                std::env::temp_dir().join("burncloud/fake/artifacts/qwen_fake.gguf"),
                // 测试回执必须明确表示已完成摘要校验并通过。
                ArtifactVerificationStatus::Verified,
            )
            .unwrap(),
        );
        orchestrator
            .observe(model, ReconcileEvidence::ArtifactPrepared)
            .unwrap();
        // 推进到下一个动作（PrepareRuntime）
        orchestrator.next_action(model).unwrap();
        // 注入运行时回执
        orchestrator
            .workloads
            .get_mut(model)
            .expect("workload")
            .runtime = Some(PreparedRuntime {
            executable: "/fake/runtime/llama.cpp/server".into(),
        });
        orchestrator
            .observe(model, ReconcileEvidence::RuntimePrepared)
            .unwrap();

        // 状态为 Starting（模拟中断点）
        assert_eq!(
            orchestrator.workload_state(model),
            Some(NodeState::Starting)
        );

        // 新的 prepare 从回执继续，最终 Ready
        assert_eq!(
            orchestrator
                .prepare_until_ready(ModelDemand::new(model).unwrap())
                .await
                .unwrap(),
            LocalPreparationOutcome::Ready
        );
        assert_eq!(orchestrator.workload_state(model), Some(NodeState::Ready));
    }

    // 测试：不支持的本地模型在准备任何东西前就停止
    #[tokio::test]
    async fn unsupported_local_model_stops_before_any_preparation_or_route_state() {
        let machine = NodeComposition::start(
            FakeHardwareProbe::default(),
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            std::sync::Arc::new(FakeProcessManager::default()),
            std::sync::Arc::new(FakeReadinessProbe),
            std::sync::Arc::new(FakeHealthProbe),
            closed_monitor_errors(),
        );
        let mut orchestrator =
            NodeOrchestrator::new(UnsupportedResolver, FakeRuntimeAdapter, machine);
        // prepare 返回 Unsupported
        let outcome = orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        assert!(matches!(outcome, LocalPreparationOutcome::Unsupported(_)));
        // 状态为 LocalUnsupported
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::LocalUnsupported)
        );
        // 没有计划，没有挂载
        assert_eq!(orchestrator.workload_plan("qwen-4b"), None);
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);

        // 重复请求仍返回 Unsupported（终态幂等）
        let repeated = orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        assert!(matches!(repeated, LocalPreparationOutcome::Unsupported(_)));
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::LocalUnsupported)
        );
    }
}