use async_trait::async_trait;
use burncloud_node_runtime::{
    ArtifactPrepareError, ArtifactPreparer, ArtifactRequest, ArtifactSource,
    ArtifactVerificationStatus, HealthError, HealthProbe, PreparedArtifact, PreparedRuntime,
    ReadinessError, ReadinessProbe, ReadinessTarget, RuntimePrepareError, RuntimePreparer,
    RuntimeRequest,
};

struct TestArtifact;
#[async_trait]
impl ArtifactPreparer for TestArtifact {
    async fn prepare(
        &self,
        _request: ArtifactRequest,
    ) -> Result<PreparedArtifact, ArtifactPrepareError> {
        PreparedArtifact::new(
            std::env::temp_dir().join("model.gguf"),
            ArtifactVerificationStatus::Verified,
        )
        .map_err(|error| ArtifactPrepareError::PrepareFailed(error.to_string()))
    }
}

struct TestRuntime;
#[async_trait]
impl RuntimePreparer for TestRuntime {
    async fn prepare(
        &self,
        _request: RuntimeRequest,
    ) -> Result<PreparedRuntime, RuntimePrepareError> {
        Ok(PreparedRuntime {
            executable: "/tmp/llama-server".into(),
        })
    }
}

struct TestReadiness;
#[async_trait]
impl ReadinessProbe for TestReadiness {
    async fn wait_ready(&self, _target: ReadinessTarget) -> Result<(), ReadinessError> {
        Ok(())
    }
}

struct TestHealth;
#[async_trait]
impl HealthProbe for TestHealth {
    async fn is_healthy(&self, _target: ReadinessTarget) -> Result<bool, HealthError> {
        Ok(true)
    }
}

#[tokio::test]
async fn preparation_ports_accept_independent_implementations() {
    let artifact = TestArtifact
        .prepare(ArtifactRequest::new(
            ArtifactSource::parse("ignored").unwrap(),
            None,
        ))
        .await
        .unwrap();
    let runtime = TestRuntime
        .prepare(RuntimeRequest {
            runtime: "ignored".into(),
            version: None,
        })
        .await
        .unwrap();
    let target = ReadinessTarget {
        endpoint: "local".into(),
    };
    TestReadiness.wait_ready(target.clone()).await.unwrap();

    assert!(artifact.is_verified());
    assert_eq!(runtime.executable, "/tmp/llama-server");
    assert!(TestHealth.is_healthy(target).await.unwrap());
}
