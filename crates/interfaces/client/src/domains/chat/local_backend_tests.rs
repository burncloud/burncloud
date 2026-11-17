//! 默认后端本身的目录、三平台硬件与占位路径回归，不启动计时或外部操作。
use super::*;

#[test]
fn local_platform_fixtures_have_consistent_resources_and_placeholder_paths() {
    for (platform, prefix, cpu_fragment) in [
        (Platform::Windows, r"C:\BurnCloud\models\", "x64"),
        (Platform::MacOs, "/Users/user/BurnCloud/models/", "Apple"),
        (Platform::Linux, "/home/user/BurnCloud/models/", "x64"),
    ] {
        let detected = hardware(platform);
        assert_eq!(detected.platform, Some(platform));
        assert!(detected
            .cpu
            .as_deref()
            .expect("固定 CPU")
            .contains(cpu_fragment));
        assert_eq!(detected.ram_bytes, Some(24 * GIB));
        assert_eq!(detected.max_vram(), Some(8 * GIB));
        assert_eq!(detected.disk_bytes, Some(128 * GIB));
        let path = example_path(platform, "qwen3:8b", "q4");
        assert!(path.starts_with(prefix));
        assert!(path.ends_with("qwen3-8b-q4.gguf"));
    }
}

#[test]
fn local_catalog_contract_has_artifact_identity_and_one_resource_failure_sample() {
    let models = catalog();
    assert_eq!(models.len(), 14);
    let hardware = hardware(Platform::Windows);
    for model in models {
        assert!(!model.id.is_empty());
        assert!(model.updated_at.is_some());
        assert!(!model.variants.is_empty());
        assert_eq!(
            model.variants.iter().any(|variant| variant.fits(&hardware)),
            model.id != "qwen3.6-max"
        );
        for variant in model.variants {
            assert!(valid_digest(&variant.expected_sha256));
            assert!(!variant.runtime.is_empty());
            assert!(!variant.artifact_source.is_empty());
        }
    }
}

#[tokio::test]
async fn local_search_matches_names_ids_descriptions_and_trims_ascii_case() {
    for (query, expected) in [
        (" QWEN3:8B ", Some("qwen3:8b")),
        ("代码编写", Some("qwen2.5-coder:7b")),
        ("日常问答", Some("llama3.2:1b")),
        ("不存在的模型", None),
    ] {
        let models = LocalBackend.search(query.into()).await.expect("本地搜索");
        assert_eq!(models.len(), usize::from(expected.is_some()));
        assert_eq!(models.first().map(ChatModel::id), expected);
    }
    let models = LocalBackend.search("   ".into()).await.expect("空查询");
    assert_eq!(models, catalog());
}
