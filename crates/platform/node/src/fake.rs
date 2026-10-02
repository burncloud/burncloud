use async_trait::async_trait;

use crate::{
    AcceleratorKind, AcceleratorProfile, ArtifactPrepareError, ArtifactPreparer, ArtifactRequest,
    ArtifactSource, ArtifactVerificationStatus, HardwareProbe, HardwareProbeError, HardwareProfile,
    HealthError, HealthProbe, PreparedArtifact, PreparedRuntime, ProcessError, ProcessHandle,
    ProcessManager, ProcessPlan, ProcessSpec, ReadinessError, ReadinessProbe, ReadinessTarget,
    RuntimeAdapter, RuntimeAdapterError, RuntimePrepareError, RuntimePreparer, RuntimeRequest,
};

/// Deterministic fake used to assemble and test the Node skeleton before real
/// hardware detection exists.
#[derive(Debug, Clone)]
pub struct FakeHardwareProbe {
    profile: HardwareProfile,
}

impl Default for FakeHardwareProbe {
    /// 创建包含固定硬件信息的伪硬件探测器。
    fn default() -> Self {
        Self {
            profile: HardwareProfile {
                cpu_threads: 16,
                memory_bytes: 64 * 1024 * 1024 * 1024,
                disk_available_bytes: 512 * 1024 * 1024 * 1024,
                accelerators: vec![AcceleratorProfile {
                    kind: AcceleratorKind::Nvidia,
                    name: "Fake GPU".to_string(),
                    memory_bytes: Some(24 * 1024 * 1024 * 1024),
                }],
            },
        }
    }
}

impl FakeHardwareProbe {
    /// 使用指定的硬件描述创建伪硬件探测器。
    pub fn new(profile: HardwareProfile) -> Self {
        Self { profile }
    }
}

#[async_trait]
impl HardwareProbe for FakeHardwareProbe {
    /// 返回预先配置的硬件描述，不访问真实系统资源。
    async fn inspect(&self) -> Result<HardwareProfile, HardwareProbeError> {
        Ok(self.profile.clone())
    }
}

/// Deterministic fake artifact preparer. It performs no download or file I/O.
#[derive(Debug, Clone, Default)]
pub struct FakeArtifactPreparer;

#[async_trait]
impl ArtifactPreparer for FakeArtifactPreparer {
    /// 根据工件来源构造确定性的本地伪工件回执。
    async fn prepare(
        &self,
        request: ArtifactRequest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        let source = match request.source() {
            ArtifactSource::File(path) => path.to_string_lossy().into_owned(),
            ArtifactSource::Http(url) => url.to_string(),
        };
        // 将来源转换为文件名片段，避免伪回执路径包含平台路径分隔符。
        // Build the fake receipt under the platform temporary directory so the
        // resulting path is absolute on Windows, Linux, and macOS alike.
        let local_path = std::env::temp_dir()
            .join("burncloud")
            .join("fake")
            .join("artifacts")
            .join(source.replace(['/', '\\', ':'], "_"));
        PreparedArtifact::new(local_path, ArtifactVerificationStatus::Verified)
            .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))
    }
}

/// Deterministic fake runtime preparer. It never downloads llama.cpp or any
/// other runtime; it only proves the runtime preparation contract can be wired.
#[derive(Debug, Clone, Default)]
pub struct FakeRuntimePreparer;

#[async_trait]
impl RuntimePreparer for FakeRuntimePreparer {
    /// 返回固定格式的伪运行时可执行文件路径。
    async fn prepare(
        &self,
        request: RuntimeRequest,
    ) -> Result<PreparedRuntime, RuntimePrepareError> {
        Ok(PreparedRuntime {
            executable: format!("/fake/runtime/{}/server", request.runtime),
        })
    }
}

/// Deterministic fake runtime adapter. It owns fake launch details so the
/// application orchestrator never invents process arguments or endpoints.
#[derive(Debug, Clone, Default)]
pub struct FakeRuntimeAdapter;

#[async_trait]
impl RuntimeAdapter for FakeRuntimeAdapter {
    /// 根据伪运行时和工件生成固定的进程及就绪检查计划。
    async fn plan(
        &self,
        runtime: &PreparedRuntime,
        artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError> {
        Ok(ProcessPlan {
            process: ProcessSpec {
                program: runtime.executable.clone(),
                args: vec![artifact.local_path().to_string_lossy().into_owned()],
            },
            readiness: ReadinessTarget {
                endpoint: "http://127.0.0.1:39122/health".into(),
            },
            local_endpoint: "http://127.0.0.1:39122".into(),
        })
    }
}

/// Deterministic fake process manager. It never spawns an OS process.
#[derive(Debug, Clone)]
pub struct FakeProcessManager {
    pid: u32,
}

impl Default for FakeProcessManager {
    /// 创建使用固定进程标识的伪进程管理器。
    fn default() -> Self {
        Self { pid: 42 }
    }
}

impl FakeProcessManager {
    /// 使用指定进程标识创建伪进程管理器。
    pub fn new(pid: u32) -> Self {
        Self { pid }
    }
}

#[async_trait]
impl ProcessManager for FakeProcessManager {
    /// 返回固定进程句柄，不启动操作系统进程。
    async fn start(&self, _spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        Ok(ProcessHandle { pid: self.pid })
    }

    /// 忽略停止请求，不操作操作系统进程。
    async fn stop(&self, _handle: ProcessHandle) -> Result<(), ProcessError> {
        Ok(())
    }
}

/// Deterministic fake readiness probe. It performs no network request.
#[derive(Debug, Clone, Default)]
pub struct FakeReadinessProbe;

#[async_trait]
impl ReadinessProbe for FakeReadinessProbe {
    /// 立即返回成功，不发送网络请求。
    async fn wait_ready(&self, _target: ReadinessTarget) -> Result<(), ReadinessError> {
        Ok(())
    }
}

/// Deterministic fake health probe. It performs no network request.
#[derive(Debug, Clone, Default)]
pub struct FakeHealthProbe;

#[async_trait]
impl HealthProbe for FakeHealthProbe {
    /// 返回固定的健康结果，不发送网络请求。
    async fn is_healthy(&self, _target: ReadinessTarget) -> Result<bool, HealthError> {
        Ok(true)
    }
}
