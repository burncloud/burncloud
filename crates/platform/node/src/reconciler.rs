use crate::{InvalidNodeTransition, NodeState, NodeStateMachine};

/// One orchestration action requested by the Node runtime skeleton.
///
/// The reconciler chooses *which stage comes next*. It does not implement the
/// stage and does not make BurnCloud Model / Provider / Route decisions.
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

/// Evidence reported back after an action has been executed by its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileEvidence {
    Resolved,
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
    fn default() -> Self {
        Self::new()
    }
}

impl DemandReconciler {
    pub const fn new() -> Self {
        Self {
            machine: NodeStateMachine::new(),
        }
    }

    pub const fn state(&self) -> NodeState {
        self.machine.state()
    }

    /// Start or continue convergence and return the next required action.
    pub fn next_action(&mut self) -> Result<ReconcileAction, InvalidNodeTransition> {
        use NodeState::*;

        match self.machine.state() {
            Absent => {
                self.machine.transition(Resolving)?;
                Ok(ReconcileAction::Resolve)
            }
            Resolving => Ok(ReconcileAction::Resolve),
            PreparingArtifact => Ok(ReconcileAction::PrepareArtifact),
            ArtifactReady => {
                self.machine.transition(PreparingRuntime)?;
                Ok(ReconcileAction::PrepareRuntime)
            }
            PreparingRuntime => Ok(ReconcileAction::PrepareRuntime),
            Starting => Ok(ReconcileAction::StartProcess),
            Ready => Ok(ReconcileAction::AttachToExistingRouter),
            Routable => Ok(ReconcileAction::Noop),
            Failed | Unhealthy => Ok(ReconcileAction::Recover),
        }
    }

    /// Accept evidence from a stage owner and advance the state machine.
    ///
    /// Evidence never means "sleep finished". Real implementations must only
    /// emit evidence after the corresponding stage has actually succeeded.
    pub fn observe(&mut self, evidence: ReconcileEvidence) -> Result<(), InvalidNodeTransition> {
        use NodeState::*;
        use ReconcileEvidence::*;

        let next = match (self.machine.state(), evidence) {
            (Resolving, Resolved) => PreparingArtifact,
            (PreparingArtifact, ArtifactPrepared) => ArtifactReady,
            (PreparingRuntime, RuntimePrepared) => Starting,
            (Starting, ProcessStarted) => Ready,
            (Ready, ReadinessVerified) => Ready,
            (Ready, RouterAttached) => Routable,
            (Ready | Routable, BecameUnhealthy) => Unhealthy,
            (Resolving | PreparingArtifact | PreparingRuntime | Starting | Unhealthy, Failed) => Failed,
            (Failed, Resolved) => Resolving,
            (from, _) => {
                return Err(InvalidNodeTransition { from, to: from });
            }
        };

        if next != self.machine.state() {
            self.machine.transition(next)?;
        }
        Ok(())
    }

    pub fn retry(&mut self) -> Result<(), InvalidNodeTransition> {
        match self.machine.state() {
            NodeState::Failed => self.machine.transition(NodeState::Resolving),
            NodeState::Unhealthy => self.machine.transition(NodeState::Starting),
            state => Err(InvalidNodeTransition { from: state, to: state }),
        }
    }
}
