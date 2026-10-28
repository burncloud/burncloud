#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "测试支持代码使用断言构造确定性场景。"
)]

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU32, AtomicUsize, Ordering},
        Arc,
    },
};

use async_trait::async_trait;
use burncloud_node_runtime::{
    deployment::{
        DeploymentError, DeploymentEvent, DeploymentEventKind, DeploymentEventPublishError,
        DeploymentEventPublisher, DeploymentMonitorError, DeploymentService,
    },
    ArtifactVerificationStatus, HealthError, HealthProbe, PreparedArtifact, PreparedRuntime,
    ProcessError, ProcessHandle, ProcessManager, ProcessPlan, ProcessSpec, ReadinessError,
    ReadinessProbe, ReadinessTarget, RuntimeAdapter, RuntimeAdapterError,
};
use tokio::sync::{mpsc::UnboundedReceiver, Mutex, Notify};

#[derive(Default)]
pub(super) struct Gate {
    entered: Notify,
    release: Notify,
}

impl Gate {
    pub(super) fn enter(&self) {
        self.entered.notify_one();
    }

    pub(super) async fn wait_until_entered(&self) {
        self.entered.notified().await;
    }

    pub(super) async fn pause(&self) {
        self.enter();
        self.release.notified().await;
    }

    pub(super) fn open(&self) {
        self.release.notify_one();
    }
}

pub(super) struct FixedAdapter {
    pub(super) result: Result<ProcessPlan, String>,
    pub(super) gate: Option<Arc<Gate>>,
    pub(super) calls: AtomicUsize,
    pub(super) inputs: Mutex<Vec<(PreparedRuntime, PreparedArtifact)>>,
}

impl FixedAdapter {
    pub(super) fn new(plan: ProcessPlan) -> Self {
        Self {
            result: Ok(plan),
            gate: None,
            calls: AtomicUsize::new(0),
            inputs: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn failing(message: &str) -> Self {
        Self {
            result: Err(message.to_string()),
            gate: None,
            calls: AtomicUsize::new(0),
            inputs: Mutex::new(Vec::new()),
        }
    }

    pub(super) fn gated(plan: ProcessPlan, gate: Arc<Gate>) -> Self {
        Self {
            result: Ok(plan),
            gate: Some(gate),
            calls: AtomicUsize::new(0),
            inputs: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl RuntimeAdapter for FixedAdapter {
    async fn plan(
        &self,
        runtime: &PreparedRuntime,
        artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst) + 1;
        self.inputs
            .lock()
            .await
            .push((runtime.clone(), artifact.clone()));
        if call == 1 {
            if let Some(gate) = &self.gate {
                gate.pause().await;
            }
        }
        self.result.clone().map_err(RuntimeAdapterError::PlanFailed)
    }
}

#[derive(Default)]
pub(super) struct ScriptedProcessManager {
    next_pid: AtomicU32,
    pub(super) start_error: Mutex<Option<String>>,
    pub(super) stop_error: Mutex<Option<String>>,
    pub(super) liveness: Mutex<VecDeque<Result<bool, String>>>,
    pub(super) starts: Mutex<Vec<ProcessSpec>>,
    pub(super) stops: Mutex<Vec<ProcessHandle>>,
    pub(super) liveness_calls: AtomicUsize,
}

impl ScriptedProcessManager {
    pub(super) fn new() -> Self {
        Self {
            next_pid: AtomicU32::new(41),
            ..Self::default()
        }
    }

    pub(super) async fn start_count(&self) -> usize {
        self.starts.lock().await.len()
    }

    pub(super) async fn stop_count(&self) -> usize {
        self.stops.lock().await.len()
    }
}

#[async_trait]
impl ProcessManager for ScriptedProcessManager {
    async fn start(&self, spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        self.starts.lock().await.push(spec);
        if let Some(message) = self.start_error.lock().await.take() {
            return Err(ProcessError::StartFailed(message));
        }
        Ok(ProcessHandle {
            pid: self.next_pid.fetch_add(1, Ordering::SeqCst),
        })
    }

    async fn stop(&self, handle: ProcessHandle) -> Result<(), ProcessError> {
        self.stops.lock().await.push(handle);
        if let Some(message) = self.stop_error.lock().await.take() {
            return Err(ProcessError::StopFailed(message));
        }
        Ok(())
    }

    async fn is_running(&self, _handle: ProcessHandle) -> Result<bool, ProcessError> {
        self.liveness_calls.fetch_add(1, Ordering::SeqCst);
        match self.liveness.lock().await.pop_front().unwrap_or(Ok(true)) {
            Ok(is_running) => Ok(is_running),
            Err(message) => Err(ProcessError::IsRunningFailed(message)),
        }
    }
}

#[derive(Default)]
pub(super) struct ScriptedReadinessProbe {
    pub(super) results: Mutex<VecDeque<Result<(), String>>>,
    pub(super) gate: Mutex<Option<Arc<Gate>>>,
    pub(super) targets: Mutex<Vec<ReadinessTarget>>,
}

#[async_trait]
impl ReadinessProbe for ScriptedReadinessProbe {
    async fn wait_ready(&self, target: ReadinessTarget) -> Result<(), ReadinessError> {
        self.targets.lock().await.push(target);
        if let Some(gate) = self.gate.lock().await.take() {
            gate.pause().await;
        }
        match self.results.lock().await.pop_front().unwrap_or(Ok(())) {
            Ok(()) => Ok(()),
            Err(message) => Err(ReadinessError::CheckFailed(message)),
        }
    }
}

#[derive(Default)]
pub(super) struct ScriptedHealthProbe {
    pub(super) results: Mutex<VecDeque<Result<bool, String>>>,
    pub(super) gate: Mutex<Option<Arc<Gate>>>,
    pub(super) targets: Mutex<Vec<ReadinessTarget>>,
}

#[async_trait]
impl HealthProbe for ScriptedHealthProbe {
    async fn is_healthy(&self, target: ReadinessTarget) -> Result<bool, HealthError> {
        self.targets.lock().await.push(target);
        if let Some(gate) = self.gate.lock().await.take() {
            gate.pause().await;
        }
        match self.results.lock().await.pop_front().unwrap_or(Ok(true)) {
            Ok(healthy) => Ok(healthy),
            Err(message) => Err(HealthError::CheckFailed(message)),
        }
    }
}

#[derive(Default)]
pub(super) struct RecordingPublisher {
    pub(super) attempts: Mutex<Vec<DeploymentEvent>>,
    pub(super) received: Mutex<Vec<DeploymentEvent>>,
    failures: Mutex<Vec<(DeploymentEventKind, usize)>>,
    blocked: Mutex<Option<(DeploymentEventKind, Arc<Gate>)>>,
}

impl RecordingPublisher {
    pub(super) async fn fail_kind(&self, kind: DeploymentEventKind, count: usize) {
        self.failures.lock().await.push((kind, count));
    }

    pub(super) async fn block_kind(&self, kind: DeploymentEventKind, gate: Arc<Gate>) {
        *self.blocked.lock().await = Some((kind, gate));
    }
}

#[async_trait]
impl DeploymentEventPublisher for RecordingPublisher {
    async fn publish(&self, event: DeploymentEvent) -> Result<(), DeploymentEventPublishError> {
        self.attempts.lock().await.push(event.clone());

        let gate = {
            let mut blocked = self.blocked.lock().await;
            if blocked
                .as_ref()
                .is_some_and(|(kind, _)| *kind == event.kind)
            {
                blocked.take().map(|(_, gate)| gate)
            } else {
                None
            }
        };
        if let Some(gate) = gate {
            gate.pause().await;
        }

        let should_fail = {
            let mut failures = self.failures.lock().await;
            match failures
                .iter_mut()
                .find(|(kind, count)| *kind == event.kind && *count > 0)
            {
                Some((_, count)) => {
                    *count -= 1;
                    true
                }
                None => false,
            }
        };
        if should_fail {
            return Err(DeploymentEventPublishError::PublishFailed(
                "scripted publisher failure".to_string(),
            ));
        }

        self.received.lock().await.push(event);
        Ok(())
    }
}

pub(super) fn artifact() -> PreparedArtifact {
    PreparedArtifact::new(
        "deployment-test-model.gguf",
        ArtifactVerificationStatus::Verified,
    )
    .expect("test artifact path is valid")
}

pub(super) fn plan() -> ProcessPlan {
    ProcessPlan {
        process: ProcessSpec {
            program: "runtime-test".to_string(),
            args: vec!["--model".to_string(), "model.gguf".to_string()],
        },
        readiness: ReadinessTarget {
            endpoint: "http://127.0.0.1:39122/ready".to_string(),
        },
        local_endpoint: "http://127.0.0.1:39122".to_string(),
    }
}

pub(super) fn new_service(
    processes: Arc<ScriptedProcessManager>,
    readiness: Arc<ScriptedReadinessProbe>,
    health: Arc<ScriptedHealthProbe>,
    publisher: Arc<RecordingPublisher>,
) -> (DeploymentService, UnboundedReceiver<DeploymentMonitorError>) {
    DeploymentService::new(processes, readiness, health, publisher)
}

pub(super) async fn deploy(
    service: &DeploymentService,
    model_id: &str,
    adapter: Arc<dyn RuntimeAdapter>,
) -> Result<burncloud_node_runtime::deployment::DeploymentOutcome, DeploymentError> {
    service
        .deploy(
            model_id.to_string(),
            artifact(),
            PreparedRuntime {
                executable: "runtime-prepared".to_string(),
            },
            adapter,
        )
        .await
}
