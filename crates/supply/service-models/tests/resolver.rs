use burncloud_service_models::{
    FakeModelResolver, ModelResolutionRequest, ModelResolver,
};

#[tokio::test]
async fn fake_resolver_returns_runtime_and_artifact_choice() {
    let resolved = FakeModelResolver
        .resolve(ModelResolutionRequest {
            model: "qwen-4b".into(),
            accelerator_memory_bytes: Some(24 * 1024 * 1024 * 1024),
        })
        .await
        .unwrap();

    assert_eq!(resolved.model, "qwen-4b");
    assert_eq!(resolved.runtime, "llama.cpp");
    assert!(resolved.artifact_source.contains("qwen-4b"));
    assert!(resolved.artifact_source.ends_with(".gguf"));
}
