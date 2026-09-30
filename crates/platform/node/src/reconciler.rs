use crate::{InvalidNodeTransition, NodeState, NodeStateMachine};

/// One orchestration action requested by the Node runtime skeleton.
/// The reconciler chooses the next stage; it does not implement that stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileAction {
    Resolve,
    PrepareArtifact,
    PrepareRuntime,
    StartProcess,
    VerifyReadiness,
    AttachToExistingRouter,
    Recover,
    Noop,
}

/// Evidence reported by the owner that actually executed a stage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileEvidence {
    Resolved,
    LocalUnsupported,
    ArtifactPrepared,
    RuntimePrepared,
    ProcessStarted,
    ReadinessVerified,
    RouterAttached,
    BecameUnhealthy,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DemandReconciler {
    machine: NodeStateMachine,
}

impl Default for DemandReconciler {
    /// 创建处于初始状态的需求协调器。
    fn default() -> Self {
        Self::new()
    }
}

impl DemandReconciler {
    /// 创建使用默认节点状态机的协调器。
    pub const fn new() -> Self {
        Self {
            machine: NodeStateMachine::new(),
        }
    }

    /// 返回协调器当前的节点状态。
    pub const fn state(&self) -> NodeState {
        self.machine.state()
    }

    /// 根据当前状态推进状态机，并返回应执行的下一项动作。
    pub fn next_action(&mut self) -> Result<ReconcileAction, InvalidNodeTransition> {
        use NodeState::*;
        match self.machine.state() {
            Absent => {
                self.machine.transition(Resolving)?;
                Ok(ReconcileAction::Resolve)
            }
            Resolving => Ok(ReconcileAction::Resolve),
            LocalUnsupported => Ok(ReconcileAction::Noop),
            PreparingArtifact => Ok(ReconcileAction::PrepareArtifact),
            ArtifactReady => {
                self.machine.transition(PreparingRuntime)?;
                Ok(ReconcileAction::PrepareRuntime)
            }
            PreparingRuntime => Ok(ReconcileAction::PrepareRuntime),
            Starting => Ok(ReconcileAction::StartProcess),
            WaitingReady => Ok(ReconcileAction::VerifyReadiness),
            Ready => Ok(ReconcileAction::AttachToExistingRouter),
            Routable => Ok(ReconcileAction::Noop),
            Failed | Unhealthy => Ok(ReconcileAction::Recover),
        }
    }

    /// 根据阶段完成证据推进状态机；经过时间或进程已启动本身不代表就绪。
    pub fn observe(&mut self, evidence: ReconcileEvidence) -> Result<(), InvalidNodeTransition> {
        use NodeState::*;
        use ReconcileEvidence::*;

        // 只接受与当前阶段匹配的证据，其他组合均保持原状态并报告错误。
        let next = match (self.machine.state(), evidence) {
            (Resolving, Resolved) => PreparingArtifact,
            (Resolving, ReconcileEvidence::LocalUnsupported) => NodeState::LocalUnsupported,
            (PreparingArtifact, ArtifactPrepared) => ArtifactReady,
            (PreparingRuntime, RuntimePrepared) => Starting,
            (Starting, ProcessStarted) => WaitingReady,
            (WaitingReady, ReadinessVerified) => Ready,
            (Ready, RouterAttached) => Routable,
            (Ready | Routable, BecameUnhealthy) => Unhealthy,
            (
                Resolving | PreparingArtifact | PreparingRuntime | Starting | WaitingReady | Ready
                | Unhealthy,
                ReconcileEvidence::Failed,
            ) => NodeState::Failed,
            (from, _) => return Err(InvalidNodeTransition { from, to: from }),
        };

        self.machine.transition(next)
    }

    /// 按失败或不健康状态对应的恢复路径重新尝试。
    pub fn retry(&mut self) -> Result<(), InvalidNodeTransition> {
        match self.machine.state() {
            NodeState::Failed => self.machine.transition(NodeState::Resolving),
            NodeState::Unhealthy => self.machine.transition(NodeState::Starting),
            state => Err(InvalidNodeTransition {
                from: state,
                to: state,
            }),
        }
    }
}
