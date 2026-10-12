use serde::{Deserialize, Serialize};

/// A manifest describing one logical model and its runnable variants.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelManifest {
    /// Short model name accepted by the local resolver.
    pub model_name: String,
    /// Human-readable description of the model.
    pub description: String,
    /// Runtime backends that can run at least one variant of this model.
    pub supported_backends: Vec<String>,
    /// Manifest or catalog version.
    pub version: String,
    /// Available model variants, such as different sizes or quantizations.
    pub variants: Vec<Variant>,
}

/// One downloadable model artifact together with its minimum resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    /// Stable identifier within the model manifest.
    pub id: String,
    /// Optional model-size label, such as `7b` or `13b`.
    pub model_size: Option<String>,
    /// Quantization label, such as `q4_k_m` or `q5_k_m`.
    pub quantization: String,
    /// Runtime backend required by this variant.
    pub backend: String,
    /// Minimum system RAM required to run the variant.
    pub min_ram_bytes: u64,
    /// Minimum accelerator VRAM required to run the variant.
    pub min_vram_bytes: u64,
    /// Disk space occupied by the downloaded artifact.
    pub disk_size_bytes: u64,
    /// Integrity checksum of the artifact, for example a SHA-256 digest.
    pub checksum: String,
    /// Download location of the artifact.
    pub artifact_uri: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trips_through_json() {
        let manifest = ModelManifest {
            model_name: "deepseek-r1".into(),
            description: "A reasoning model".into(),
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
}
