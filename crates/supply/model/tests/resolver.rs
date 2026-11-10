use burncloud_node_runtime::{HardwareProbe, SystemHardwareProbe};
use burncloud_supply_model::{
    FakeModelResolver, ModelResolutionOutcome, ModelResolutionRequest, ModelResolver,
};

#[tokio::test]
async fn fake_resolver_returns_runtime_and_artifact_choice() {
    let outcome = FakeModelResolver
        .resolve(ModelResolutionRequest {
            model: "qwen-4b".into(),
            hardware: SystemHardwareProbe::new().inspect().await.unwrap(),
        })
        .await
        .unwrap();

    let resolved = match outcome {
        ModelResolutionOutcome::Local(resolved) => resolved,
        ModelResolutionOutcome::Unsupported(reason) => {
            panic!("fake unexpectedly unsupported: {reason:?}")
        }
    };
    assert_eq!(resolved.model, "qwen-4b");
    assert_eq!(resolved.runtime, "llama.cpp");
    assert!(resolved.artifact_source.contains("qwen-4b"));
    assert!(resolved.artifact_source.ends_with(".gguf"));
}
