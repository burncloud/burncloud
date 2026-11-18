// ============================================================================
// 依赖导入区
// ============================================================================

// 从本 crate 的 node_orchestrator 模块导入：
// - LocalPreparationOutcome：本地准备阶段的三种结果（Unsupported / Routable / Ready）
// - ModelDemand：模型需求（携带模型名等信息）
// - NodeOrchestrator：本地节点生命周期编排器（泛型化，依赖注入各类探测/管理器）
use crate::node_orchestrator::{LocalPreparationOutcome, ModelDemand, NodeOrchestrator};

// 从 burncloud_node_runtime 导入运行时抽象 trait 与探测接口：
// - ArtifactPreparation：模型制品（权重等）准备
// - HardwareProbe：硬件能力探测
// - HealthProbe：运行时健康探测
// - ProcessManager：进程管理（启动/停止）
// - ReadinessProbe：运行时就绪探测
// - RuntimeAdapter：运行时适配（把具体运行时抽象出来）
// - RuntimePreparer：运行时准备（环境、依赖等）
use burncloud_node_runtime::{
    ArtifactPreparation, HardwareProbe, HealthProbe, ProcessManager, ReadinessProbe,
    RuntimeAdapter, RuntimePreparer,
};

// 从 burncloud_router 导入本地路由附件 ID（newtype，内部用 .0 访问整型）
use burncloud_router::local_attachment::LocalRouteAttachmentId;

// 从 burncloud_supply_model 导入：
// - LocalModelUnsupported：本地模型不支持的原因
// - ModelResolver：模型解析器（解析模型名 -> 运行时适配）
use burncloud_supply_model::{LocalModelUnsupported, ModelResolver};

// 导入 Future trait，用于泛型异步闭包的返回类型约束
use std::future::Future;

// ============================================================================
// DetachedRoute：路由已分离的凭证
// ============================================================================

// 派生 Debug（调试打印）、Clone（可复制）、PartialEq/Eq（可比较，便于测试断言）
#[derive(Debug, Clone, PartialEq, Eq)]
// 定义一个结构体，表示“某模型的那条路由附件已被移除”的证明
pub struct DetachedRoute {
    // 该凭证所属的模型名（私有字段，外部无法直接构造）
    model: String,
    // 被分离的路由附件 ID（私有字段，保证只能由内部成功路径产生）
    attachment_id: LocalRouteAttachmentId,
}

// 为 DetachedRoute 实现访问方法
impl DetachedRoute {
    // 返回该凭证所属模型名的字符串切片引用
    pub fn model(&self) -> &str {
        // 借用内部 model 字段
        &self.model
    }

    // 返回被分离的路由附件 ID
    // const fn：可在编译期求值；LocalRouteAttachmentId 是 Copy，所以直接返回值
    pub const fn attachment_id(&self) -> LocalRouteAttachmentId {
        // 直接复制返回
        self.attachment_id
    }
}

// ============================================================================
// LocalRouteOutcome：准备 + 挂载组合后的最终结果
// ============================================================================

// 派生 Debug/Clone/PartialEq/Eq，便于打印与测试比较
#[derive(Debug, Clone, PartialEq, Eq)]
// 对外只暴露两种终态，不暴露内部 Ready 中间态
pub enum LocalRouteOutcome {
    // 可路由：携带成功挂载的路由附件 ID
    Routable(LocalRouteAttachmentId),
    // 本地不支持：携带不支持的原因
    Unsupported(LocalModelUnsupported),
}

// ============================================================================
// attach_ready_node_route：挂载路由附件（带生命周期门禁）
// ============================================================================

// 异步函数：执行 Traffic 侧附件副作用，并只在成功后记录路由证据
// 泛型 M/T/H/A/R/P/Q/E：对应 NodeOrchestrator 的八个依赖 trait
// 泛型 F/Fut：调用方注入的异步闭包与其返回 Future
pub async fn attach_ready_node_route<M, T, H, A, R, P, Q, E, F, Fut>(
    // 可变借用的编排器：需要修改其内部生命周期状态
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    // 目标模型名
    model: &str,
    // 调用方注入的附件副作用闭包（FnOnce：只会被调用一次）
    attach: F,
) -> anyhow::Result<LocalRouteAttachmentId>
// where 子句：约束各泛型参数必须实现对应 trait
where
    // 模型解析器
    M: ModelResolver,
    // 运行时适配器
    T: RuntimeAdapter,
    // 硬件探测
    H: HardwareProbe,
    // 制品准备
    A: ArtifactPreparation,
    // 运行时准备器
    R: RuntimePreparer,
    // 进程管理器
    P: ProcessManager,
    // 就绪探测
    Q: ReadinessProbe,
    // 健康探测
    E: HealthProbe,
    // 闭包：无参，返回一个 Future
    F: FnOnce() -> Fut,
    // 该 Future 输出 anyhow::Result<LocalRouteAttachmentId>
    Fut: Future<Output = anyhow::Result<LocalRouteAttachmentId>>,
{
    // 步骤 1：生命周期门禁校验——模型必须处于“可挂载”状态
    // 若不在，? 直接返回错误，此时尚未调用 attach()，不会产生任何副作用
    orchestrator.ensure_ready_for_attachment(model)?;

    // 步骤 2：执行调用方注入的 Traffic 侧副作用（真正创建路由附件）
    // match 处理成功/失败两种分支
    let attachment_id = match attach().await {
        // 成功：拿到附件 ID
        Ok(attachment_id) => attachment_id,
        // 失败：进入错误处理分支
        Err(error) => {
            // 步骤 3：把模型工作负载收敛到 Failed，避免 READY 成为永久伪进度态
            // 这会让请求侧停止等待（同步更新请求侧状态）
            orchestrator.mark_failed(model)?;
            // 把原始错误向上传播
            return Err(error);
        }
    };
    // 步骤 4：副作用成功后才记录附件证据，模型真正进入 Routable
    orchestrator.record_attachment(model, attachment_id)?;
    // 返回成功挂载的附件 ID
    Ok(attachment_id)
}

// ============================================================================
// prepare_and_attach_node_route：准备 + 挂载的组合流程
// ============================================================================

// 异步函数：跑完本地准备轨道，仅在 READY 后把 endpoint 挂到既有路由真值上
// 泛型与上类似，多了一个 attach 闭包接收 (模型名, endpoint) 两个参数
pub async fn prepare_and_attach_node_route<M, T, H, A, R, P, Q, E, F, Fut>(
    // 编排器可变借用
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    // 模型需求（按值传入，因为 prepare_until_ready 需要消费它）
    demand: ModelDemand,
    // 调用方注入的附件闭包：接收 (模型名, base_url) 返回 Future
    attach: F,
) -> anyhow::Result<LocalRouteOutcome>
// 泛型约束
where
    // 模型解析器
    M: ModelResolver,
    // 运行时适配器
    T: RuntimeAdapter,
    // 硬件探测
    H: HardwareProbe,
    // 制品准备
    A: ArtifactPreparation,
    // 运行时准备器
    R: RuntimePreparer,
    // 进程管理器
    P: ProcessManager,
    // 就绪探测
    Q: ReadinessProbe,
    // 健康探测
    E: HealthProbe,
    // 闭包：接收两个 String 参数，返回 Future
    F: FnOnce(String, String) -> Fut,
    // Future 输出附件 ID
    Fut: Future<Output = anyhow::Result<LocalRouteAttachmentId>>,
{
    // 先克隆出模型名（demand 后面会被消费，需要提前保留 model）
    let model = demand.model.clone();
    // 调用编排器的准备流程，直到 READY 或返回 Unsupported/Routable
    // 按三种结果分支处理
    match orchestrator.prepare_until_ready(demand).await? {
        // 分支 A：本地不支持——直接透传原因，不产生任何路由副作用
        LocalPreparationOutcome::Unsupported(reason) => Ok(LocalRouteOutcome::Unsupported(reason)),
        // 分支 B：已经可路由（例如此前已挂载）——直接透传附件 ID
        LocalPreparationOutcome::Routable(attachment_id) => {
            Ok(LocalRouteOutcome::Routable(attachment_id))
        }
        // 分支 C：刚就绪——需要把运行时 endpoint 挂到既有路由真值上
        LocalPreparationOutcome::Ready => {
            // 从工作负载计划中取出本地 endpoint（如 http://127.0.0.1:39122）
            // 若没有计划，说明状态不一致，直接返回错误
            let base_url = orchestrator
                .workload_plan(&model)
                .ok_or_else(|| anyhow::anyhow!("ready local runtime has no process plan"))?
                // 克隆 endpoint 字符串（后面要 move 进异步闭包）
                .local_endpoint
                .clone();
            // 克隆模型名，准备 move 进闭包
            let attach_model = model.clone();
            // 复用 attach_ready_node_route：门禁 -> attach(model, base_url) -> 记录
            let attachment_id =
                attach_ready_node_route(orchestrator, &model, || attach(attach_model, base_url))
                    .await?;
            // 返回可路由结果
            Ok(LocalRouteOutcome::Routable(attachment_id))
        }
    }
}

// ============================================================================
// detach_unhealthy_node_route：健康失败时的 fail-closed 分离
// ============================================================================

// 异步函数：把不健康的本地路由安全分离，产出唯一凭证
// 多了两个闭包泛型 FQ/QFut（quarantine）与 FD/DFut（detach）
pub async fn detach_unhealthy_node_route<M, T, H, A, R, P, Q, E, FQ, QFut, FD, DFut>(
    // 编排器可变借用
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    // 目标模型名
    model: &str,
    // 待分离的附件 ID
    attachment_id: LocalRouteAttachmentId,
    // Traffic 侧隔离闭包（让路由不可选）
    quarantine: FQ,
    // Traffic 侧删除闭包（清理附件）
    detach: FD,
) -> anyhow::Result<DetachedRoute>
// 泛型约束
where
    // 模型解析器
    M: ModelResolver,
    // 运行时适配器
    T: RuntimeAdapter,
    // 硬件探测
    H: HardwareProbe,
    // 制品准备
    A: ArtifactPreparation,
    // 运行时准备器
    R: RuntimePreparer,
    // 进程管理器
    P: ProcessManager,
    // 就绪探测
    Q: ReadinessProbe,
    // 健康探测
    E: HealthProbe,
    // quarantine 闭包：接收附件 ID，返回 Future
    FQ: FnOnce(LocalRouteAttachmentId) -> QFut,
    // quarantine Future 输出 ()
    QFut: Future<Output = anyhow::Result<()>>,
    // detach 闭包：接收附件 ID，返回 Future
    FD: FnOnce(LocalRouteAttachmentId) -> DFut,
    // detach Future 输出 ()
    DFut: Future<Output = anyhow::Result<()>>,
{
    // 步骤 1：所有权校验——附件必须属于该模型
    // 防止拿别的模型的 attachment 来 detach
    if orchestrator.workload_attachment_id(model) != Some(attachment_id) {
        // 不匹配则直接报错退出
        anyhow::bail!(
            "attachment {} does not belong to model '{model}'",
            attachment_id.0
        );
    }

    // 步骤 2：按当前生命周期状态分派处理
    match orchestrator.workload_state(model) {
        // 情况 A：当前是 Routable——需要先隔离再标记不健康
        Some(burncloud_node_runtime::NodeState::Routable) => {
            // 关键顺序：先 Traffic 真值（quarantine），让路由不可选
            quarantine(attachment_id).await?;
            // 成功后再发布生命周期状态 Unhealthy
            orchestrator.mark_unhealthy(model)?;
        }
        // 情况 B：已经是 Unhealthy——说明上次 quarantine 成功但 delete 失败
        Some(burncloud_node_runtime::NodeState::Unhealthy) => {
            // Traffic 已 fail-closed，直接重试 cleanup，无需再次 quarantine 或改状态
        }
        // 情况 C：其他状态——拒绝分离
        state => anyhow::bail!("model '{model}' cannot detach local route from state {state:?}"),
    }

    // 步骤 3：删除附件（cleanup）。失败则 ? 返回，不产生凭证、状态保持 Unhealthy
    detach(attachment_id).await?;
    // 步骤 4：cleanup 成功，清空附件记录
    orchestrator.clear_attachment(model, attachment_id)?;
    // 产出唯一凭证：把模型名与附件 ID 绑定
    Ok(DetachedRoute {
        // 模型名转 String
        model: model.to_string(),
        // 附件 ID 复制
        attachment_id,
    })
}

// ============================================================================
// begin_detached_route_recovery：只有分离成功才开启恢复
// ============================================================================

// 同步函数：用分离凭证开启恢复轨道
pub fn begin_detached_route_recovery<M, T, H, A, R, P, Q, E>(
    // 编排器可变借用
    orchestrator: &mut NodeOrchestrator<M, T, H, A, R, P, Q, E>,
    // 目标模型名
    model: &str,
    // 分离凭证（按值传入，消费后不可复用）
    detached: DetachedRoute,
) -> anyhow::Result<()>
// 泛型约束
where
    // 模型解析器
    M: ModelResolver,
    // 运行时适配器
    T: RuntimeAdapter,
    // 硬件探测
    H: HardwareProbe,
    // 制品准备
    A: ArtifactPreparation,
    // 运行时准备器
    R: RuntimePreparer,
    // 进程管理器
    P: ProcessManager,
    // 就绪探测
    Q: ReadinessProbe,
    // 健康探测
    E: HealthProbe,
{
    // 校验 1：凭证的模型必须等于传入的模型（防跨模型恢复）
    if detached.model() != model {
        // 不匹配则报错退出
        anyhow::bail!(
            "detached route receipt belongs to model '{}', not '{model}'",
            detached.model()
        );
    }
    // 校验 2：附件必须已清空（仍持有附件说明分离不完整）
    if orchestrator.workload_attachment_id(model).is_some() {
        // 仍有附件则报错退出
        anyhow::bail!("model '{model}' still owns a route attachment");
    }
    // 校验 3：交给编排器开启恢复，内部会校验凭证是否匹配“当前那次 detach”
    // 状态会从 Unhealthy 推到 Starting
    orchestrator.begin_recovery(model, detached.attachment_id())
}

// ============================================================================
// 测试模块
// ============================================================================

// 仅在测试编译时编译
#[cfg(test)]
mod tests {
    // 导入父模块所有内容（被测试的函数与类型）
    use super::*;
    // 导入请求侧状态类型，用于校验请求侧同步
    use crate::node_request::NodeRequestState;
    // 导入各类 Fake 实现与组合/状态类型，用于构造测试编排器
    use burncloud_node_runtime::{
        FakeArtifactPreparer, FakeHardwareProbe, FakeHealthProbe, FakeProcessManager,
        FakeReadinessProbe, FakeRuntimeAdapter, FakeRuntimePreparer, NodeComposition, NodeState,
    };
    // 导入 Fake 模型解析器
    use burncloud_supply_model::FakeModelResolver;

    // 构造一个已关闭的部署监控错误通道（接收端立即关闭，Fake 组合可正常启动）
    fn closed_monitor_errors() -> tokio::sync::mpsc::UnboundedReceiver<
        burncloud_node_runtime::deployment::DeploymentMonitorError,
    > {
        // 创建 unbounded channel，丢弃发送端 => 接收端关闭
        let (_, receiver) = tokio::sync::mpsc::unbounded_channel();
        // 返回接收端
        receiver
    }

    // 构造一个全 Fake 实现的 NodeOrchestrator（未准备任何模型）
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
        // 用 Fake 实现构造编排器
        NodeOrchestrator::new(
            // Fake 模型解析器
            FakeModelResolver,
            // Fake 运行时适配器
            FakeRuntimeAdapter,
            // 用 Fake 组件启动 NodeComposition
            NodeComposition::start(
                // Fake 硬件探测
                FakeHardwareProbe::default(),
                // Fake 制品准备
                FakeArtifactPreparer,
                // Fake 运行时准备器
                FakeRuntimePreparer,
                // Fake 进程管理器（Arc 包裹以共享）
                std::sync::Arc::new(FakeProcessManager::default()),
                // Fake 就绪探测
                std::sync::Arc::new(FakeReadinessProbe),
                // Fake 健康探测
                std::sync::Arc::new(FakeHealthProbe),
                // 已关闭的监控错误通道
                closed_monitor_errors(),
            ),
        )
    }

    // 构造已把 "qwen-4b" 准备到 READY 的编排器，并附带请求侧状态句柄
    async fn ready_orchestrator() -> (
        NodeOrchestrator<
            FakeModelResolver,
            FakeRuntimeAdapter,
            FakeHardwareProbe,
            FakeArtifactPreparer,
            FakeRuntimePreparer,
            FakeProcessManager,
            FakeReadinessProbe,
            FakeHealthProbe,
        >,
        NodeRequestState,
    ) {
        // 创建默认请求侧状态
        let request_state = NodeRequestState::default();
        // 用请求侧状态句柄包装 Fake 编排器
        let mut orchestrator = fake_orchestrator().with_request_state(request_state.clone());
        // 准备 "qwen-4b" 直到 READY
        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        // 返回编排器与请求侧状态句柄
        (orchestrator, request_state)
    }

    // 测试：挂载成功是进入 Routable 的必要条件
    #[tokio::test]
    async fn successful_existing_route_attachment_is_required_for_routable() {
        // 拿到已就绪的编排器与请求侧状态
        let (mut orchestrator, request_state) = ready_orchestrator().await;
        // 调用挂载函数，attach 闭包返回固定 ID 42
        let id = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();

        // 断言返回的 ID 是 42
        assert_eq!(id, LocalRouteAttachmentId(42));
        // 断言编排器状态是 Routable
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
        // 断言编排器记录了附件 ID 42
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(42))
        );
        // 断言请求侧状态同步为 Routable
        assert_eq!(
            request_state.state_for("qwen-4b"),
            Some(NodeState::Routable)
        );
    }

    // 测试：就绪运行时被自动组合进既有路由附件
    #[tokio::test]
    async fn ready_runtime_is_automatically_composed_with_existing_route_attachment() {
        // 拿到未准备模型的 Fake 编排器
        let mut orchestrator = fake_orchestrator();
        // 调用组合流程（从准备到挂载一条龙）
        let outcome = prepare_and_attach_node_route(
            &mut orchestrator,
            ModelDemand::new("qwen-4b").unwrap(),
            |model, base_url| async move {
                // 断言传入的模型名正确
                assert_eq!(model, "qwen-4b");
                // 断言传入的 base_url 来自运行时计划
                assert_eq!(base_url, "http://127.0.0.1:39122");
                // 返回附件 ID 77
                Ok(LocalRouteAttachmentId(77))
            },
        )
        .await
        .unwrap();

        // 断言最终结果是 Routable(77)
        assert_eq!(
            outcome,
            LocalRouteOutcome::Routable(LocalRouteAttachmentId(77))
        );
        // 断言编排器状态是 Routable
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
    }

    // 测试：非就绪模型不能创建附件副作用
    #[tokio::test]
    async fn non_ready_model_cannot_create_attachment_side_effect() {
        // 拿到未准备模型的 Fake 编排器
        let mut orchestrator = fake_orchestrator();
        // 原子计数器，记录 attach 闭包被调用次数
        let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        // 克隆句柄 move 进闭包
        let calls_for_attach = calls.clone();

        // 直接对未就绪模型调用挂载函数
        let result = attach_ready_node_route(&mut orchestrator, "qwen-4b", move || async move {
            // 若被调用则计数 +1
            calls_for_attach.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            // 返回附件 ID 999
            Ok(LocalRouteAttachmentId(999))
        })
        .await;

        // 断言调用返回错误（门禁失败）
        assert!(result.is_err());
        // 断言 attach 闭包零调用（门禁先行，未产生副作用）
        assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
        // 断言编排器没有该模型状态
        assert_eq!(orchestrator.workload_state("qwen-4b"), None);
        // 断言编排器没有附件记录
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);
    }

    // 测试：附件失败会收敛到 Failed
    #[tokio::test]
    async fn failed_existing_route_attachment_converges_to_failed() {
        // 拿到已就绪的编排器与请求侧状态
        let (mut orchestrator, request_state) = ready_orchestrator().await;
        // 调用挂载函数，attach 闭包直接失败
        let result = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            // 模拟数据库写入失败
            anyhow::bail!("database write failed")
        })
        .await;

        // 断言返回错误
        assert!(result.is_err());
        // 断言编排器状态收敛到 Failed
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Failed)
        );
        // 断言请求侧状态同步为 Failed
        assert_eq!(request_state.state_for("qwen-4b"), Some(NodeState::Failed));
        // 断言没有路由遗漏响应（不会给请求方一个“找不到路由”的假响应）
        assert!(request_state.route_miss_response("qwen-4b").is_none());
        // 断言没有残留附件
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);
    }

    // 测试：不健康路由先 quarantine 后 detach，之后可恢复
    #[tokio::test]
    async fn unhealthy_route_is_quarantined_before_detach_and_can_then_recover() {
        // 拿到已就绪的编排器与请求侧状态
        let (mut orchestrator, request_state) = ready_orchestrator().await;
        // 先挂载一个附件（ID 42）
        let route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();

        // 原子布尔，记录 quarantine 是否已执行
        let quarantined = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        // 克隆给 quarantine 闭包
        let quarantine_flag = quarantined.clone();
        // 克隆给 detach 闭包（用于断言顺序）
        let detach_flag = quarantined.clone();
        // 执行分离流程
        let detached = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            route,
            // quarantine 闭包
            move |id| async move {
                // 断言附件 ID 正确
                assert_eq!(id, LocalRouteAttachmentId(42));
                // 标记 quarantine 已执行
                quarantine_flag.store(true, std::sync::atomic::Ordering::SeqCst);
                // 返回成功
                Ok(())
            },
            // detach 闭包
            move |id| async move {
                // 断言附件 ID 正确
                assert_eq!(id, LocalRouteAttachmentId(42));
                // 断言 detach 时 quarantine 已先执行（顺序契约）
                assert!(detach_flag.load(std::sync::atomic::Ordering::SeqCst));
                // 返回成功
                Ok(())
            },
        )
        .await
        .unwrap();

        // 断言编排器状态是 Unhealthy
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Unhealthy)
        );
        // 断言请求侧状态同步为 Unhealthy
        assert_eq!(
            request_state.state_for("qwen-4b"),
            Some(NodeState::Unhealthy)
        );
        // 断言附件已被清空
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);

        // 用凭证开启恢复
        begin_detached_route_recovery(&mut orchestrator, "qwen-4b", detached).unwrap();
        // 断言状态推进到 Starting
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Starting)
        );
    }

    // 测试：detach 失败时保持 Unhealthy，且不产生恢复凭证
    #[tokio::test]
    async fn failed_detach_stays_unhealthy_and_cannot_produce_recovery_receipt() {
        // 拿到已就绪的编排器与请求侧状态
        let (mut orchestrator, request_state) = ready_orchestrator().await;
        // 先挂载一个附件（ID 42）
        let route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();

        // 执行分离流程，detach 闭包失败
        let result = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            route,
            // quarantine 成功
            |_| async { Ok(()) },
            // detach 失败
            |_| async { anyhow::bail!("existing router detach failed") },
        )
        .await;

        // 断言返回错误（未产出凭证）
        assert!(result.is_err());
        // 断言状态保持 Unhealthy
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Unhealthy)
        );
        // 断言请求侧状态同步为 Unhealthy
        assert_eq!(
            request_state.state_for("qwen-4b"),
            Some(NodeState::Unhealthy)
        );
        // 断言附件仍保留（cleanup 未完成）
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(42))
        );
    }

    // 测试：detach 失败后可重试 cleanup，且不重复 quarantine
    #[tokio::test]
    async fn failed_detach_can_retry_cleanup_without_requarantine() {
        // 拿到已就绪的编排器（不需要请求侧状态）
        let (mut orchestrator, _) = ready_orchestrator().await;
        // 先挂载一个附件（ID 40）
        let route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(40))
        })
        .await
        .unwrap();

        // 第一次分离：quarantine 成功，detach 失败
        let first = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            route,
            // quarantine 成功
            |_| async { Ok(()) },
            // detach 失败
            |_| async { anyhow::bail!("first cleanup failed") },
        )
        .await;
        // 断言第一次失败
        assert!(first.is_err());
        // 断言状态是 Unhealthy
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Unhealthy)
        );

        // 第二次分离：重试 cleanup
        let detached = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            route,
            // quarantine 若被执行会 panic（证明重试路径跳过 quarantine）
            |_| async { panic!("quarantine must not repeat on cleanup retry") },
            // detach 成功
            |_| async { Ok(()) },
        )
        .await
        .unwrap();

        // 断言凭证模型名正确
        assert_eq!(detached.model(), "qwen-4b");
        // 断言凭证附件 ID 是 40
        assert_eq!(detached.attachment_id(), LocalRouteAttachmentId(40));
        // 断言附件已清空
        assert_eq!(orchestrator.workload_attachment_id("qwen-4b"), None);

        // 用凭证开启恢复
        begin_detached_route_recovery(&mut orchestrator, "qwen-4b", detached).unwrap();
        // 断言状态推进到 Starting
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Starting)
        );
    }

    // 测试：跨模型凭证被拒
    #[tokio::test]
    async fn detached_route_receipt_cannot_recover_another_model() {
        // 拿到未准备模型的 Fake 编排器
        let mut orchestrator = fake_orchestrator();

        // 准备 qwen-4b 到 READY
        orchestrator
            .prepare_until_ready(ModelDemand::new("qwen-4b").unwrap())
            .await
            .unwrap();
        // 挂载 qwen-4b 附件（ID 41）
        let qwen_route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(41))
        })
        .await
        .unwrap();
        // 分离 qwen-4b，拿到凭证
        let qwen_receipt = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            qwen_route,
            |_| async { Ok(()) },
            |_| async { Ok(()) },
        )
        .await
        .unwrap();

        // 准备 deepseek-8b 到 READY
        orchestrator
            .prepare_until_ready(ModelDemand::new("deepseek-8b").unwrap())
            .await
            .unwrap();
        // 挂载 deepseek-8b 附件（ID 42）
        let deepseek_route = attach_ready_node_route(&mut orchestrator, "deepseek-8b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();
        // 分离 deepseek-8b，拿到凭证
        let deepseek_receipt = detach_unhealthy_node_route(
            &mut orchestrator,
            "deepseek-8b",
            deepseek_route,
            |_| async { Ok(()) },
            |_| async { Ok(()) },
        )
        .await
        .unwrap();

        // 断言 deepseek-8b 当前是 Unhealthy
        assert_eq!(
            orchestrator.workload_state("deepseek-8b"),
            Some(NodeState::Unhealthy)
        );

        // 尝试用 qwen 的凭证恢复 deepseek
        let result = begin_detached_route_recovery(&mut orchestrator, "deepseek-8b", qwen_receipt);
        // 断言被拒绝
        assert!(result.is_err());
        // 断言 deepseek-8b 状态仍是 Unhealthy
        assert_eq!(
            orchestrator.workload_state("deepseek-8b"),
            Some(NodeState::Unhealthy)
        );

        // 用 deepseek 自己的凭证恢复
        begin_detached_route_recovery(&mut orchestrator, "deepseek-8b", deepseek_receipt).unwrap();
        // 断言状态推进到 Starting
        assert_eq!(
            orchestrator.workload_state("deepseek-8b"),
            Some(NodeState::Starting)
        );
    }

    // 测试：同模型的过期凭证不能恢复最新的 detach
    #[tokio::test]
    async fn stale_receipt_from_same_model_cannot_recover_latest_detach() {
        // 拿到已就绪的编排器
        let (mut orchestrator, _) = ready_orchestrator().await;

        // 第一次挂载（ID 61）
        let first_route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(61))
        })
        .await
        .unwrap();
        // 第一次分离，拿到旧凭证
        let old_receipt = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            first_route,
            |_| async { Ok(()) },
            |_| async { Ok(()) },
        )
        .await
        .unwrap();
        // 克隆旧凭证（后面要用）
        let old_receipt_copy = old_receipt.clone();

        // 用旧凭证正常恢复一次
        begin_detached_route_recovery(&mut orchestrator, "qwen-4b", old_receipt).unwrap();
        // 恢复直到 READY
        orchestrator.recover_until_ready("qwen-4b").await.unwrap();
        // 重新挂载新附件（ID 62）
        attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(62))
        })
        .await
        .unwrap();

        // 第二次分离（ID 62），拿到当前凭证
        let current_receipt = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            LocalRouteAttachmentId(62),
            |_| async { Ok(()) },
            |_| async { Ok(()) },
        )
        .await
        .unwrap();

        // 用旧凭证（61）尝试恢复当前 detach（62），应被拒
        assert!(
            begin_detached_route_recovery(&mut orchestrator, "qwen-4b", old_receipt_copy).is_err()
        );
        // 断言状态仍是 Unhealthy
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Unhealthy)
        );

        // 用当前凭证（62）恢复，成功
        begin_detached_route_recovery(&mut orchestrator, "qwen-4b", current_receipt).unwrap();
        // 断言状态推进到 Starting
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Starting)
        );
    }

    // 测试：quarantine 失败即停，不进入 Unhealthy，detach 不被调用
    #[tokio::test]
    async fn quarantine_failure_stops_before_unhealthy_or_recovery() {
        // 拿到已就绪的编排器
        let (mut orchestrator, _) = ready_orchestrator().await;
        // 先挂载一个附件（ID 42）
        let route = attach_ready_node_route(&mut orchestrator, "qwen-4b", || async {
            Ok(LocalRouteAttachmentId(42))
        })
        .await
        .unwrap();

        // 执行分离流程，quarantine 失败
        let result = detach_unhealthy_node_route(
            &mut orchestrator,
            "qwen-4b",
            route,
            // quarantine 失败
            |_| async { anyhow::bail!("traffic quarantine failed") },
            // detach 若被执行会 panic（证明 quarantine 失败后不会继续）
            |_| async { panic!("detach must not run after quarantine failure") },
        )
        .await;

        // 断言返回错误
        assert!(result.is_err());
        // 断言状态保持 Routable（未进入 Unhealthy）
        assert_eq!(
            orchestrator.workload_state("qwen-4b"),
            Some(NodeState::Routable)
        );
        // 断言附件仍保留
        assert_eq!(
            orchestrator.workload_attachment_id("qwen-4b"),
            Some(LocalRouteAttachmentId(42))
        );
    }
}
