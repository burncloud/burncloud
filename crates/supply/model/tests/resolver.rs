use burncloud_supply_model::{
    FakeModelResolver, LocalModelUnsupported, LocalModelUnsupportedReason, ModelResolutionOutcome,
    ModelResolutionRequest, ModelResolver,
};

#[test]
fn unsupported_is_a_normal_resolution_outcome_not_an_infrastructure_error() {
    let outcome = ModelResolutionOutcome::Unsupported(LocalModelUnsupported {
        model: "qwen-4b".into(),
        reason: LocalModelUnsupportedReason::InsufficientAcceleratorMemory {
            required_bytes: 12 * 1024 * 1024 * 1024,
            available_bytes: Some(8 * 1024 * 1024 * 1024),
        },
    });
    assert!(matches!(outcome, ModelResolutionOutcome::Unsupported(_)));
}

#[tokio::test]
async fn fake_resolver_returns_runtime_and_artifact_choice() {
    let outcome = FakeModelResolver
        .resolve(ModelResolutionRequest {
            model: "qwen-4b".into(),
            accelerator_memory_bytes: Some(24 * 1024 * 1024 * 1024),
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
