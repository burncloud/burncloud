/// Runtime convergence state for one local Node workload.
///
/// These states describe runtime progress only. They deliberately do not own
/// BurnCloud model/provider/router business decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NodeState {
    #[default]
    Absent,
    Resolving,
    /// Supply proved that this machine has no acceptable local variant.
    /// This is a settled non-serving outcome, not a runtime failure.
    LocalUnsupported,
    PreparingArtifact,
    ArtifactReady,
    PreparingRuntime,
    /// The local process is being spawned.
    Starting,
    /// The process exists, but readiness has not yet been proven.
    WaitingReady,
    /// Readiness evidence has been observed.
    Ready,
    /// Attached to the existing BurnCloud routing path.
    Routable,
    Failed,
    Unhealthy,
}

impl NodeState {
    /// 判断从当前状态到 `next` 是否属于已定义的合法迁移。
    pub const fn can_transition_to(self, next: Self) -> bool {
        use NodeState::*;

        matches!(
            (self, next),
            (Absent, Resolving)
                | (Resolving, LocalUnsupported)
                | (Resolving, PreparingArtifact)
                | (Resolving, Failed)
                | (PreparingArtifact, ArtifactReady)
                | (PreparingArtifact, Failed)
                | (ArtifactReady, PreparingRuntime)
                | (PreparingRuntime, Starting)
                | (PreparingRuntime, Failed)
                | (Starting, WaitingReady)
                | (Starting, Failed)
                | (WaitingReady, Ready)
                | (WaitingReady, Failed)
                | (Ready, Routable)
                | (Ready, Unhealthy)
                | (Ready, Failed)
                | (Routable, Unhealthy)
                | (Unhealthy, Starting)
                | (Unhealthy, Failed)
                | (Failed, Resolving)
                | (Failed, Absent)
                | (LocalUnsupported, Absent)
                | (Routable, Absent)
        )
    }

    /// 判断当前状态是否表示工作负载已接入路由。
    pub const fn is_serving(self) -> bool {
        matches!(self, Self::Routable)
    }

    /// 判断当前状态是否属于失败或不健康状态。
    pub const fn is_failure(self) -> bool {
        matches!(self, Self::Failed | Self::Unhealthy)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidNodeTransition {
    pub from: NodeState,
    pub to: NodeState,
}

impl std::fmt::Display for InvalidNodeTransition {
    /// 格式化非法迁移的起始状态和目标状态。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid node state transition: {:?} -> {:?}",
            self.from, self.to
        )
    }
}

impl std::error::Error for InvalidNodeTransition {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeStateMachine {
    state: NodeState,
}

impl NodeStateMachine {
    /// 创建处于 `Absent` 初始状态的状态机。
    pub const fn new() -> Self {
        Self {
            state: NodeState::Absent,
        }
    }

    /// 返回状态机当前记录的节点状态。
    pub const fn state(&self) -> NodeState {
        self.state
    }

    /// 校验并应用状态迁移；非法迁移时保留原状态并返回错误。
    pub fn transition(&mut self, next: NodeState) -> Result<(), InvalidNodeTransition> {
        // 先验证迁移边界，确保失败不会部分修改状态机。
        if !self.state.can_transition_to(next) {
            return Err(InvalidNodeTransition {
                from: self.state,
                to: next,
            });
        }
        self.state = next;
        Ok(())
    }
}
