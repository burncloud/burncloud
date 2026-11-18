use burncloud_supply_model::{load_default_manifests, parse_manifests, ModelManifest, Variant};

#[test]
fn manifest_round_trips_through_json() {
    let manifest = ModelManifest {
        model_name: "deepseek-r1".into(),
        description: "推理模型".into(),
        supported_backends: vec!["llama.cpp".into()],
        version: "1".into(),
        variants: vec![Variant {
            id: "deepseek-r1-7b-q4_k_m".into(),
            model_size: Some("7b".into()),
            quantization: "q4_k_m".into(),
            backend: "llama.cpp".into(),
            min_ram_bytes: 8 * 1024 * 1024 * 1024,
            min_vram_bytes: 6 * 1024 * 1024 * 1024,
            disk_size_bytes: 5 * 1024 * 1024 * 1024,
            checksum: "sha256:example".into(),
            artifact_uri: "https://example.invalid/deepseek-r1-7b-q4_k_m.gguf".into(),
        }],
    };

    let json = serde_json::to_string(&manifest).unwrap();
    let decoded: ModelManifest = serde_json::from_str(&json).unwrap();

    assert_eq!(decoded, manifest);
}

#[test]
fn default_manifest_loads_shared_models_with_variant_identifiers() {
    let manifests = load_default_manifests().unwrap();

    assert_eq!(
        manifests
            .iter()
            .map(|manifest| manifest.model_name.as_str())
            .collect::<Vec<_>>(),
        [
            "qwen2.5:1.5b",
            "qwen2.5:3b",
            "llama3.2:1b",
            "DeepSeek V4.1 Flash",
            "GLM-5.3",
            "GLM-5.3-Flash",
            "Kimi K3",
            "MiniMax M3.1 Flash",
            "Qwen3.8-27B",
            "Qwen3-8B",
            "Qwen3-Coder-Next",
            "Qwen3.8-2.4T-A95B",
            "Qwen3.8-Flash-Next",
            "Qwen-Image-2512",
            "Wan2.2-TI2V-5B",
            "MiMo-V2.6-Flash-RL",
            "gpt-oss-20b",
            "gpt-oss-120b",
            "Gemma 4 E4B IT",
            "Gemma 4 31B"
        ]
    );
    assert_eq!(manifests.len(), 20);
    assert!(manifests
        .iter()
        .all(|manifest| !manifest.variants.is_empty()));
    assert_eq!(
        manifests
            .iter()
            .flat_map(|manifest| manifest.variants.iter())
            .map(|variant| (variant.id.as_str(), variant.backend.as_str()))
            .collect::<Vec<_>>(),
        [
            ("qwen2.5-1.5b-instruct-q4_k_m", "llama.cpp"),
            ("qwen2.5-3b-instruct-q4_k_m", "llama.cpp"),
            ("llama3.2-1b-instruct-q8_0", "llama.cpp"),
            ("deepseek-v4-1-flash-iq1-s", "llama.cpp"),
            ("glm-5-3-iq1-s", "llama.cpp"),
            ("glm-5-3-flash-iq2-xxs", "llama.cpp"),
            ("kimi-k3-ud-iq1-s", "llama.cpp"),
            ("minimax-m3-1-flash-iq1-s", "llama.cpp"),
            ("qwen3-8-27b-iq1-s", "llama.cpp"),
            ("qwen3-8b-q4", "llama.cpp"),
            ("qwen3-coder-next-2-bit-xl", "llama.cpp"),
            ("qwen3-8-2-4t-a95b-1-bit", "llama.cpp"),
            ("qwen3-8-flash-next-ud-iq3-xxs", "llama.cpp"),
            ("qwen-image-2512-q4-k-m", "llama.cpp"),
            ("wan2-2-ti2v-5b-q4-q5", "llama.cpp"),
            ("mimo-v2-6-flash-rl-iq2-xs", "llama.cpp"),
            ("gpt-oss-20b-q4-k-m", "llama.cpp"),
            ("gpt-oss-120b-mxfp4", "llama.cpp"),
            ("gemma-4-e4b-it-4-bit", "llama.cpp"),
            ("gemma-4-31b-4-bit", "llama.cpp")
        ]
    );
    assert!(manifests
        .iter()
        .flat_map(|manifest| manifest.variants.iter())
        .all(|variant| !variant.artifact_uri.starts_with("file:///")));
    assert!(manifests[3..].iter().all(|manifest| {
        manifest.version == "preset-2026-11-14"
            && manifest.variants[0].checksum
                == "sha256:0000000000000000000000000000000000000000000000000000000000000000"
    }));
}

#[test]
fn external_manifest_json_can_override_default_source() {
    let external_json = r#"[{
        "model_name": "external-model",
        "description": "外部清单",
        "supported_backends": ["llama.cpp"],
        "version": "external",
        "variants": [{
            "id": "external-q4",
            "model_size": "7b",
            "quantization": "Q4_K_M",
            "backend": "llama.cpp",
            "min_ram_bytes": 8,
            "min_vram_bytes": 4,
            "disk_size_bytes": 3,
            "checksum": "sha256:external",
            "artifact_uri": "https://models.example/external-q4.gguf"
        }]
    }]"#;

    let manifests = parse_manifests(external_json).unwrap();

    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].model_name, "external-model");
    assert_eq!(manifests[0].variants[0].id, "external-q4");
}
