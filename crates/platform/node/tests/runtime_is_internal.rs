use burncloud_node_runtime::{FakeRuntimePreparer, RuntimePreparer, RuntimeRequest};

#[tokio::test]
async fn llama_cpp_is_only_runtime_input_not_business_api() {
    let prepared = FakeRuntimePreparer
        .prepare(RuntimeRequest {
            runtime: "llama.cpp".into(),
            version: None,
        })
        .await
        .unwrap();

    assert_eq!(prepared.executable, "/fake/runtime/llama.cpp/server");
}
