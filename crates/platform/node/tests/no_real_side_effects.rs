use burncloud_node_runtime::{
    ArtifactPreparer, ArtifactRequest, ArtifactSource, FakeArtifactPreparer, FakeRuntimePreparer,
    RuntimePreparer, RuntimeRequest,
};

#[tokio::test]
async fn preparation_fakes_return_virtual_paths_only() {
    let artifact = FakeArtifactPreparer
        .prepare(ArtifactRequest::new(
            ArtifactSource::parse("qwen-4b.gguf").unwrap(),
            None,
        ))
        .await
        .unwrap();
    let runtime = FakeRuntimePreparer
        .prepare(RuntimeRequest {
            runtime: "llama.cpp".into(),
            version: None,
        })
        .await
        .unwrap();

    assert!(artifact.local_path.starts_with("/fake/"));
    assert!(runtime.executable.starts_with("/fake/"));
}
