use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, FakeArtifactPreparer, FakeHealthProbe, FakeReadinessProbe,
    FakeRuntimePreparer, HealthProbe, ReadinessProbe, ReadinessTarget, RuntimePreparer,
    RuntimeRequest,
};

#[tokio::test]
async fn fake_artifact_preparer_returns_verified_local_artifact() {
    let artifact = FakeArtifactPreparer
        .prepare(ArtifactRequest {
            source: "models/qwen-4b.gguf".into(),
            expected_digest: Some("sha256:fake".into()),
        })
        .await
        .unwrap();

    assert!(artifact.verified);
    assert!(artifact.local_path.starts_with("/fake/artifacts/"));
}

#[tokio::test]
async fn fake_runtime_preparer_returns_fake_llama_cpp_executable() {
    let runtime = FakeRuntimePreparer
        .prepare(RuntimeRequest {
            runtime: "llama.cpp".into(),
            version: Some("fake-v0".into()),
        })
        .await
        .unwrap();

    assert_eq!(runtime.executable, "/fake/runtime/llama.cpp/server");
}

#[tokio::test]
async fn fake_readiness_and_health_are_explicit_ports() {
    let target = ReadinessTarget {
        endpoint: "http://127.0.0.1:39122/health".into(),
    };

    FakeReadinessProbe.wait_ready(target.clone()).await.unwrap();
    assert!(FakeHealthProbe.is_healthy(target).await.unwrap());
}
