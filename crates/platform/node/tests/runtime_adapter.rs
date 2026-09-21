use burncloud_node_runtime::{
    LlamaCppNativeAdapter, LlamaCppNativeConfig, PreparedArtifact, PreparedRuntime, RuntimeAdapter,
    SglangDockerAdapter, SglangDockerConfig,
};

fn runtime(executable: &str) -> PreparedRuntime {
    PreparedRuntime {
        executable: executable.to_string(),
    }
}

fn artifact(path: &str) -> PreparedArtifact {
    PreparedArtifact {
        local_path: path.to_string(),
        verified: true,
    }
}

fn has_pair(args: &[String], key: &str, value: &str) -> bool {
    args.windows(2)
        .any(|pair| pair[0] == key && pair[1] == value)
}

fn has_triplet(args: &[String], first: &str, second: &str, third: &str) -> bool {
    args.windows(3)
        .any(|part| part[0] == first && part[1] == second && part[2] == third)
}

#[tokio::test]
async fn llama_cpp_native_plan_uses_exact_runtime_artifact_and_port() {
    let adapter = LlamaCppNativeAdapter::new(
        LlamaCppNativeConfig::new("127.0.0.1", 39122)
            .with_extra_args(vec!["--ctx-size".into(), "8192".into()]),
    );

    let plan = adapter
        .plan(
            &runtime(r"C:\BurnCloud\llama-server.exe"),
            &artifact(r"D:\models\qwen.gguf"),
        )
        .await
        .unwrap();

    assert_eq!(plan.process.program, r"C:\BurnCloud\llama-server.exe");
    assert!(has_pair(
        &plan.process.args,
        "--model",
        r"D:\models\qwen.gguf"
    ));
    assert!(has_pair(&plan.process.args, "--host", "127.0.0.1"));
    assert!(has_pair(&plan.process.args, "--port", "39122"));
    assert_eq!(plan.local_endpoint, "http://127.0.0.1:39122");
    assert_eq!(plan.readiness.endpoint, "http://127.0.0.1:39122/health");
}

#[tokio::test]
async fn llama_cpp_native_plan_rejects_invalid_inputs_and_reserved_overrides() {
    let invalid = [
        ("", "model.gguf", "127.0.0.1", 39122),
        ("llama-server", "", "127.0.0.1", 39122),
        ("llama-server", "model.gguf", "bad host", 39122),
        ("llama-server", "model.gguf", "127.0.0.1", 0),
    ];

    for (executable, model, host, port) in invalid {
        let adapter = LlamaCppNativeAdapter::new(LlamaCppNativeConfig::new(host, port));
        assert!(adapter
            .plan(&runtime(executable), &artifact(model))
            .await
            .is_err());
    }

    let adapter = LlamaCppNativeAdapter::new(
        LlamaCppNativeConfig::new("127.0.0.1", 39122)
            .with_extra_args(vec!["--port".into(), "9999".into()]),
    );
    assert!(adapter
        .plan(&runtime("llama-server"), &artifact("model.gguf"))
        .await
        .is_err());
}

#[tokio::test]
async fn runtime_adapters_reject_unverified_artifacts() {
    let adapter = LlamaCppNativeAdapter::new(LlamaCppNativeConfig::new("127.0.0.1", 39122));
    let unverified = PreparedArtifact {
        local_path: "model.gguf".into(),
        verified: false,
    };

    assert!(adapter
        .plan(&runtime("llama-server"), &unverified)
        .await
        .is_err());
}

#[tokio::test]
async fn sglang_docker_plan_contains_image_mount_port_and_server_command() {
    let adapter = SglangDockerAdapter::new(SglangDockerConfig::new(
        "lmsysorg/sglang:latest",
        "127.0.0.1",
        39123,
    ));

    let plan = adapter
        .plan(&runtime("/usr/bin/docker"), &artifact("/models/qwen"))
        .await
        .unwrap();

    assert_eq!(plan.process.program, "/usr/bin/docker");
    assert_eq!(plan.process.args[0], "run");
    assert_eq!(plan.process.args[1], "--rm");
    assert_eq!(plan.process.args[2], "--gpus");
    assert_eq!(plan.process.args[3], "all");
    assert!(has_pair(
        &plan.process.args,
        "--publish",
        "127.0.0.1:39123:39123"
    ));
    assert!(has_pair(
        &plan.process.args,
        "--volume",
        "/models/qwen:/models/burncloud-artifact:ro"
    ));
    assert!(plan
        .process
        .args
        .iter()
        .any(|arg| arg == "lmsysorg/sglang:latest"));
    assert!(has_triplet(
        &plan.process.args,
        "python3",
        "-m",
        "sglang.launch_server"
    ));
    assert!(has_pair(
        &plan.process.args,
        "--model-path",
        "/models/burncloud-artifact"
    ));
    assert_eq!(plan.local_endpoint, "http://127.0.0.1:39123");
    assert_eq!(plan.readiness.endpoint, "http://127.0.0.1:39123/health");
}

#[tokio::test]
async fn sglang_docker_plan_rejects_invalid_executable_image_artifact_host_and_port() {
    let invalid = [
        ("", "sglang:latest", "/models/qwen", "127.0.0.1", 39123),
        ("docker", "", "/models/qwen", "127.0.0.1", 39123),
        ("docker", "bad image", "/models/qwen", "127.0.0.1", 39123),
        ("docker", "sglang:latest", "", "127.0.0.1", 39123),
        ("docker", "sglang:latest", "/models/qwen", "bad host", 39123),
        ("docker", "sglang:latest", "/models/qwen", "127.0.0.1", 0),
    ];

    for (executable, image, model, host, port) in invalid {
        let adapter = SglangDockerAdapter::new(SglangDockerConfig::new(image, host, port));
        assert!(adapter
            .plan(&runtime(executable), &artifact(model))
            .await
            .is_err());
    }
}

#[tokio::test]
async fn runtime_adapter_plans_are_deterministic() {
    let llama = LlamaCppNativeAdapter::new(LlamaCppNativeConfig::new("127.0.0.1", 39122));
    let llama_runtime = runtime("llama-server");
    let llama_artifact = artifact("model.gguf");
    assert_eq!(
        llama.plan(&llama_runtime, &llama_artifact).await.unwrap(),
        llama.plan(&llama_runtime, &llama_artifact).await.unwrap()
    );

    let sglang =
        SglangDockerAdapter::new(SglangDockerConfig::new("sglang:latest", "127.0.0.1", 39123));
    let docker_runtime = runtime("docker");
    let docker_artifact = artifact("/models/qwen");
    assert_eq!(
        sglang
            .plan(&docker_runtime, &docker_artifact)
            .await
            .unwrap(),
        sglang
            .plan(&docker_runtime, &docker_artifact)
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn ipv6_host_is_bracketed_in_runtime_endpoints() {
    let adapter = LlamaCppNativeAdapter::new(LlamaCppNativeConfig::new("::1", 39122));
    let plan = adapter
        .plan(&runtime("llama-server"), &artifact("model.gguf"))
        .await
        .unwrap();

    assert_eq!(plan.local_endpoint, "http://[::1]:39122");
    assert_eq!(plan.readiness.endpoint, "http://[::1]:39122/health");
}
