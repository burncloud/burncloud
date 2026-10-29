use burncloud_supply_model::{ModelManifest, Variant};

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
