use serde::{Deserialize, Serialize};

/// 描述一个逻辑模型及其可运行 Variant 的 Manifest。
///
/// 用途：由上游加载并传给 Resolver，作为模型能力过滤的只读输入。
///
/// 注意事项：Resolver 不扫描模型仓库、不发现文件，也不构建下载地址；每个 Variant 的
/// `artifact_uri` 和 `checksum` 必须由上游随 Manifest 提供。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelManifest {
    /// Resolver 接受的模型名称。
    pub model_name: String,
    /// 模型的可读描述。
    pub description: String,
    /// 至少支持一个 Variant 的 Runtime 后端列表；每个 Variant 的 `backend` 都必须出现在
    /// 此列表中。
    pub supported_backends: Vec<String>,
    /// Manifest 或模型目录版本。
    pub version: String,
    /// 可用 Variant，例如不同模型规模或量化版本。
    pub variants: Vec<Variant>,
}

/// 一个模型工件及其最低资源要求。
///
/// `artifact_uri` 是 Manifest 数据的一部分，由上游提供；Resolver 只传递它，不负责
/// 生成或修改 URI。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Variant {
    /// Manifest 内稳定的 Variant 标识。
    pub id: String,
    /// 可选的模型规模标签，例如 `7b` 或 `13b`。
    pub model_size: Option<String>,
    /// 量化标签，例如 `q4_k_m` 或 `q5_k_m`。
    pub quantization: String,
    /// 该 Variant 要求的 Runtime 后端。
    pub backend: String,
    /// 运行该 Variant 所需的最低系统 RAM，单位为字节。
    pub min_ram_bytes: u64,
    /// 运行该 Variant 所需的最低加速器显存，单位为字节。
    pub min_vram_bytes: u64,
    /// 模型工件占用的磁盘空间，单位为字节。
    pub disk_size_bytes: u64,
    /// 模型工件完整性校验和，例如 SHA-256 摘要。
    pub checksum: String,
    /// 上游提供的模型工件地址。
    pub artifact_uri: String,
}
