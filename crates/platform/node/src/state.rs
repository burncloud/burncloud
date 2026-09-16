/// Runtime convergence state for one local Node workload.
///
/// These states describe runtime progress only. They deliberately do not own
/// BurnCloud model/provider/router business decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum NodeState {
    #[default]
    Absent,
    Resolving,
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
    pub const fn can_transition_to(self, next: Self) -> bool {
        use NodeState::*;

        matches!(
            (self, next),
            (Absent, Resolving)
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
                | (Routable, Unhealthy)
                | (Unhealthy, Starting)
                | (Unhealthy, Failed)
                | (Failed, Resolving)
                | (Failed, Absent)
                | (Routable, Absent)
        )
    }

    pub const fn is_serving(self) -> bool {
        matches!(self, Self::Routable)
    }

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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid node state transition: {:?} -> {:?}", self.from, self.to)
    }
}

impl std::error::Error for InvalidNodeTransition {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NodeStateMachine {
    state: NodeState,
}

impl NodeStateMachine {
    pub const fn new() -> Self {
        Self { state: NodeState::Absent }
    }

    pub const fn state(&self) -> NodeState {
        self.state
    }

    pub fn transition(&mut self, next: NodeState) -> Result<(), InvalidNodeTransition> {
        if !self.state.can_transition_to(next) {
            return Err(InvalidNodeTransition { from: self.state, to: next });
        }
        self.state = next;
        Ok(())
    }
}
