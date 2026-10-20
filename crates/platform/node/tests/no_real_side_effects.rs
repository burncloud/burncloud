use burncloud_node_runtime::{
    ArtifactPreparation, Digest, FakeArtifactPreparer, FakeRuntimePreparer, RuntimePreparer,
    RuntimeRequest,
};
use url::Url;

#[tokio::test]
async fn preparation_fakes_return_virtual_paths_only() {
    let artifact = FakeArtifactPreparer
        .prepare(
            Url::parse("https://modelscope.cn/models/qwen-4b/resolve/main/fake.gguf").unwrap(),
            Digest::parse(
                "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        )
        .await
        .unwrap();
    let runtime = FakeRuntimePreparer
        .prepare(RuntimeRequest {
            runtime: "llama.cpp".into(),
            version: None,
        })
        .await
        .unwrap();

    assert!(artifact.local_path().is_absolute());
    assert!(artifact
        .local_path()
        .to_string_lossy()
        .contains("burncloud"));
    assert!(runtime.executable.starts_with("/fake/"));
}
