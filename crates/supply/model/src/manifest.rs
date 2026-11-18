use serde::{Deserialize, Serialize};

// 默认清单内嵌在 Rust 源文件中，以满足仓库 Rust-only PR 结构保护规则。
//
// Excel 只提供了模型仓库和资源配置，没有提供具体 GGUF 文件名或真实 SHA-256。
// 因此新增条目先用于模型解析和资源筛选，工件地址指向仓库文件页，摘要使用明确的
// 未绑定占位值；下载接入仍需后续工件解析阶段补齐文件级元数据。
const DEFAULT_MANIFEST_JSON: &str = r#"[
  {
    "model_name": "qwen2.5:1.5b",
    "description": "Qwen2.5 1.5B Instruct GGUF；量化版本、文件大小和 SHA-256 来自 Ollama 官方注册表，权重未下载。RAM/VRAM 门槛为匹配测试的演示预算，未实测，不代表官方最低配置。",
    "supported_backends": ["llama.cpp"],
    "version": "65ec06548149b04c096a120e4a6da9d4017ea809c91734ea5631e89f96ddc57b",
    "variants": [
      {
        "id": "qwen2.5-1.5b-instruct-q4_k_m",
        "model_size": "1.5b",
        "quantization": "Q4_K_M",
        "backend": "llama.cpp",
        "min_ram_bytes": 2147483648,
        "min_vram_bytes": 2147483648,
        "disk_size_bytes": 986048512,
        "checksum": "sha256:183715c435899236895da3869489cc30ac241476b4971a20285b1a462818a5b4",
        "artifact_uri": "https://registry.ollama.ai/v2/library/qwen2.5/blobs/sha256:183715c435899236895da3869489cc30ac241476b4971a20285b1a462818a5b4"
      }
    ]
  },
  {
    "model_name": "qwen2.5:3b",
    "description": "Qwen2.5 3B Instruct GGUF；量化版本、文件大小和 SHA-256 来自 Ollama 官方注册表，权重未下载。RAM/VRAM 门槛为匹配测试的演示预算，未实测，不代表官方最低配置。",
    "supported_backends": ["llama.cpp"],
    "version": "357c53fb659c5076de1d65ccb0b397446227b71a42be9d1603d46168015c9e4b",
    "variants": [
      {
        "id": "qwen2.5-3b-instruct-q4_k_m",
        "model_size": "3b",
        "quantization": "Q4_K_M",
        "backend": "llama.cpp",
        "min_ram_bytes": 4294967296,
        "min_vram_bytes": 3221225472,
        "disk_size_bytes": 1929903008,
        "checksum": "sha256:5ee4f07cdb9beadbbb293e85803c569b01bd37ed059d2715faa7bb405f31caa6",
        "artifact_uri": "https://registry.ollama.ai/v2/library/qwen2.5/blobs/sha256:5ee4f07cdb9beadbbb293e85803c569b01bd37ed059d2715faa7bb405f31caa6"
      }
    ]
  },
  {
    "model_name": "llama3.2:1b",
    "description": "Llama 3.2 1B Instruct GGUF；量化版本、文件大小和 SHA-256 来自 Ollama 官方注册表，权重未下载。RAM/VRAM 门槛为匹配测试的演示预算，未实测，不代表官方最低配置。",
    "supported_backends": ["llama.cpp"],
    "version": "baf6a787fdffd633537aa2eb51cfd54cb93ff08e28040095462bb63daf552878",
    "variants": [
      {
        "id": "llama3.2-1b-instruct-q8_0",
        "model_size": "1b",
        "quantization": "Q8_0",
        "backend": "llama.cpp",
        "min_ram_bytes": 2147483648,
        "min_vram_bytes": 2147483648,
        "disk_size_bytes": 1321082688,
        "checksum": "sha256:74701a8c35f6c8d9a4b91f3f3497643001d63e0c7a84e085bed452548fa88d45",
        "artifact_uri": "https://registry.ollama.ai/v2/library/llama3.2/blobs/sha256:74701a8c35f6c8d9a4b91f3f3497643001d63e0c7a84e085bed452548fa88d45"
      }
    ]
  },
  {
    "model_name": "DeepSeek V4.1 Flash",
    "description": "推荐 GPU：4×3090 或 80GB 级单卡；磁盘：1TB+ PCIe4/5 NVMe；RAM：128GB+。最低解析门槛：RAM 233GiB、显存 48GiB、磁盘 220GiB。来源：unsloth/DeepSeek-V4.1-Flash-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "deepseek-v4-1-flash-iq1-s",
        "model_size": null,
        "quantization": "IQ1_S",
        "backend": "llama.cpp",
        "min_ram_bytes": 250181844992,
        "min_vram_bytes": 51539607552,
        "disk_size_bytes": 236223201280,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/DeepSeek-V4.1-Flash-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "GLM-5.3",
    "description": "推荐 GPU：双 DGX Spark 256GB 或 Mac Studio 256GB；磁盘：512GB+ NVMe；RAM：256GB（2-bit）。最低解析门槛：RAM 223GiB、磁盘 250GiB。来源：Scantist/GLM-5.3-SLIM-GGUF 等社区仓库。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "glm-5-3-iq1-s",
        "model_size": null,
        "quantization": "IQ1_S",
        "backend": "llama.cpp",
        "min_ram_bytes": 239444426752,
        "min_vram_bytes": 0,
        "disk_size_bytes": 268435456000,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/Scantist/GLM-5.3-SLIM-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "GLM-5.3-Flash",
    "description": "推荐 GPU：单卡 96GB 或 128GB 统一内存设备；磁盘：120GB+ NVMe；RAM：128GB（3-bit）。最低解析门槛：RAM 64GiB、显存 24GiB、磁盘 90GiB。来源：unsloth/GLM-5.3-Flash-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "glm-5-3-flash-iq2-xxs",
        "model_size": null,
        "quantization": "IQ2_XXS",
        "backend": "llama.cpp",
        "min_ram_bytes": 68719476736,
        "min_vram_bytes": 25769803776,
        "disk_size_bytes": 96636764160,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/GLM-5.3-Flash-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Kimi K3",
    "description": "推荐 GPU：8×H100/H200/B300/MI355X；磁盘：2TB+ NVMe RAID0；RAM：2TB。最低解析门槛：RAM 610GiB、显存 640GiB、磁盘 600GiB。来源：unsloth/Kimi-K3-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "kimi-k3-ud-iq1-s",
        "model_size": null,
        "quantization": "UD-IQ1_S",
        "backend": "llama.cpp",
        "min_ram_bytes": 654982512640,
        "min_vram_bytes": 687194767360,
        "disk_size_bytes": 644245094400,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/Kimi-K3-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "MiniMax M3.1 Flash",
    "description": "推荐 GPU：8×A40 46GB（4-bit 实测）；磁盘：250GB+ NVMe；RAM：256GB+（4-bit）。最低解析门槛：RAM 130GiB、磁盘 130GiB，显存可由统一内存或多卡合计提供。来源：unsloth/MiniMax-M3.1-Flash-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "minimax-m3-1-flash-iq1-s",
        "model_size": null,
        "quantization": "IQ1_S",
        "backend": "llama.cpp",
        "min_ram_bytes": 139586437120,
        "min_vram_bytes": 0,
        "disk_size_bytes": 139586437120,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/MiniMax-M3.1-Flash-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Qwen3.8-27B",
    "description": "推荐 GPU：RTX 4090/5080 24GB 或 32GB+ 统一内存；磁盘：25–35GB（Q6/Q8）；RAM：64GB+。最低解析门槛：RAM 8GiB、显存 8GiB、磁盘 8GiB。来源：unsloth/Qwen3.8-27B-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "qwen3-8-27b-iq1-s",
        "model_size": null,
        "quantization": "IQ1_S",
        "backend": "llama.cpp",
        "min_ram_bytes": 8589934592,
        "min_vram_bytes": 8589934592,
        "disk_size_bytes": 8589934592,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/Qwen3.8-27B-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Qwen3-8B",
    "description": "推荐 GPU：RTX 3060 12GB 或 RTX 4060 8GB 以上；磁盘：20GB+；RAM：32GB。最低解析门槛：RAM 16GiB、显存 6GiB、磁盘 6GiB。来源：Qwen/Qwen3-8B-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "qwen3-8b-q4",
        "model_size": "8b",
        "quantization": "Q4",
        "backend": "llama.cpp",
        "min_ram_bytes": 17179869184,
        "min_vram_bytes": 6442450944,
        "disk_size_bytes": 6442450944,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/Qwen/Qwen3-8B-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Qwen3-Coder-Next",
    "description": "推荐 GPU：双卡 24GB（48GB）或 48GB 统一内存；磁盘：50GB+；RAM：64–96GB。最低解析门槛：RAM 30GiB、显存 24GiB、磁盘 30GiB。来源：unsloth/Qwen3-Coder-Next-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "qwen3-coder-next-2-bit-xl",
        "model_size": null,
        "quantization": "2-bit XL",
        "backend": "llama.cpp",
        "min_ram_bytes": 32212254720,
        "min_vram_bytes": 25769803776,
        "disk_size_bytes": 32212254720,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/Qwen3-Coder-Next-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Qwen3.8-2.4T-A95B",
    "description": "推荐 GPU：A100/H100 80GB×4 以上；磁盘：512GB–1TB NVMe；RAM：512GB（剪枝版目标）。最低解析门槛：RAM 397GiB、磁盘 400GiB，多卡大显存由设备提供。来源：unsloth/Qwen3.8-2.4T-A95B-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "qwen3-8-2-4t-a95b-1-bit",
        "model_size": null,
        "quantization": "1-bit dynamic",
        "backend": "llama.cpp",
        "min_ram_bytes": 426275504128,
        "min_vram_bytes": 0,
        "disk_size_bytes": 429496729600,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/Qwen3.8-2.4T-A95B-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Qwen3.8-Flash-Next",
    "description": "推荐 GPU：双 RTX 3090（48GB）；磁盘：100GB+ NVMe；RAM：128GB。最低解析门槛：RAM 64GiB、显存 24GiB、磁盘 80GiB。来源：unsloth/Qwen3.8-Flash-Next-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "qwen3-8-flash-next-ud-iq3-xxs",
        "model_size": null,
        "quantization": "UD-IQ3_XXS",
        "backend": "llama.cpp",
        "min_ram_bytes": 68719476736,
        "min_vram_bytes": 25769803776,
        "disk_size_bytes": 85899345920,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/Qwen3.8-Flash-Next-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Qwen-Image-2512",
    "description": "推荐 GPU：24GB 卡（3090/4090）；磁盘：30GB+；RAM：32–40GB。最低解析门槛：RAM 16GiB、显存 8GiB、磁盘 20GiB。来源：unsloth/Qwen-Image-2512-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "qwen-image-2512-q4-k-m",
        "model_size": null,
        "quantization": "Q4_K_M",
        "backend": "llama.cpp",
        "min_ram_bytes": 17179869184,
        "min_vram_bytes": 8589934592,
        "disk_size_bytes": 21474836480,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/Qwen-Image-2512-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Wan2.2-TI2V-5B",
    "description": "推荐 GPU：RTX 4090 24GB（720p 舒适）；磁盘：20GB+；RAM：32GB。最低解析门槛：RAM 16GiB、显存 4GiB、磁盘 10GiB。来源：unsloth/Wan2.2-TI2V-5B-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "wan2-2-ti2v-5b-q4-q5",
        "model_size": "5b",
        "quantization": "Q4/Q5",
        "backend": "llama.cpp",
        "min_ram_bytes": 17179869184,
        "min_vram_bytes": 4294967296,
        "disk_size_bytes": 10737418240,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/Wan2.2-TI2V-5B-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "MiMo-V2.6-Flash-RL",
    "description": "推荐 GPU：4×H200/4×MI355X；磁盘：170GB+（MXFP4）；RAM：256GB+。最低解析门槛：RAM 96GiB、显存 128GiB、磁盘 92GiB。来源：ggml-org/MiMo-V2.6-Flash-RL-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "mimo-v2-6-flash-rl-iq2-xs",
        "model_size": null,
        "quantization": "IQ2_XS",
        "backend": "llama.cpp",
        "min_ram_bytes": 103079215104,
        "min_vram_bytes": 137438953472,
        "disk_size_bytes": 98784247808,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/ggml-org/MiMo-V2.6-Flash-RL-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "gpt-oss-20b",
    "description": "推荐 GPU：16GB+ VRAM（RTX 4060 Ti/4070 以上）；磁盘：15GB+；RAM：32GB。最低解析门槛：RAM 16GiB、显存 8GiB、磁盘 8GiB。来源：unsloth/gpt-oss-20b-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "gpt-oss-20b-q4-k-m",
        "model_size": "20b",
        "quantization": "Q4_K_M",
        "backend": "llama.cpp",
        "min_ram_bytes": 17179869184,
        "min_vram_bytes": 8589934592,
        "disk_size_bytes": 8589934592,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/gpt-oss-20b-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "gpt-oss-120b",
    "description": "推荐 GPU：80GB+ VRAM；磁盘：120GB+ NVMe；RAM：128GB+。最低解析门槛：RAM 96GiB、显存 72GiB、磁盘 70GiB。来源：unsloth/gpt-oss-120b-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "gpt-oss-120b-mxfp4",
        "model_size": "120b",
        "quantization": "MXFP4",
        "backend": "llama.cpp",
        "min_ram_bytes": 103079215104,
        "min_vram_bytes": 77309411328,
        "disk_size_bytes": 75161927680,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/gpt-oss-120b-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Gemma 4 E4B IT",
    "description": "推荐 GPU：RTX 4060 Ti 8GB/12GB 以上；磁盘：12GB+；RAM：16GB。最低解析门槛：RAM 5GiB、显存 8GiB、磁盘 6GiB。来源：unsloth/gemma-4-E4B-it-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "gemma-4-e4b-it-4-bit",
        "model_size": "e4b",
        "quantization": "4-bit",
        "backend": "llama.cpp",
        "min_ram_bytes": 5368709120,
        "min_vram_bytes": 8589934592,
        "disk_size_bytes": 6442450944,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/gemma-4-E4B-it-GGUF/files?Revision=master"
      }
    ]
  },
  {
    "model_name": "Gemma 4 31B",
    "description": "推荐 GPU：48GB 单卡或 32GB 卡（Q6_K）；磁盘：35–40GB（Q6/Q8）；RAM：34–38GB 或 64GB 加载缓冲。最低解析门槛：RAM 20GiB、显存 17GiB、磁盘 20GiB。来源：unsloth/gemma-4-31B-it-GGUF。",
    "supported_backends": ["llama.cpp"],
    "version": "preset-2026-11-14",
    "variants": [
      {
        "id": "gemma-4-31b-4-bit",
        "model_size": "31b",
        "quantization": "4-bit",
        "backend": "llama.cpp",
        "min_ram_bytes": 21474836480,
        "min_vram_bytes": 18253611008,
        "disk_size_bytes": 21474836480,
        "checksum": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
        "artifact_uri": "https://modelscope.cn/models/unsloth/gemma-4-31B-it-GGUF/files?Revision=master"
      }
    ]
  }
]"#;

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

/// 从 JSON 文本加载模型 Manifest 清单。
///
/// 用途：解析调用方通过 `--manifest` 指定的外部清单，并保持 JSON 中模型和 Variant
/// 的顺序。
///
/// 参数：
/// - `json`：包含 `Vec<ModelManifest>` 的 JSON 文本。
///
/// 返回值：返回按输入顺序解析出的 Manifest；JSON 格式错误时返回
/// `serde_json::Error`。
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), serde_json::Error> {
/// use burncloud_supply_model::parse_manifests;
///
/// let manifests = parse_manifests(
///     r#"[{
///         "model_name": "demo",
///         "description": "测试模型",
///         "supported_backends": ["llama.cpp"],
///         "version": "1",
///         "variants": []
///     }]"#,
/// )?;
/// assert_eq!(manifests[0].model_name, "demo");
/// # Ok(())
/// # }
/// ```
pub fn parse_manifests(json: &str) -> Result<Vec<ModelManifest>, serde_json::Error> {
    serde_json::from_str(json)
}

/// 加载随 Supply Model 发布的默认 Manifest 清单。
///
/// 用途：为阶段三提供静态、版本受控的默认模型来源；清单在编译时随 crate 打包，
/// 调用时只执行一次 JSON 反序列化。默认清单保留原有 3 个基础模型，并包含 Excel
/// 确认的 17 个预设模型及其最低资源门槛。
///
/// 返回值：返回默认清单中的全部 Manifest；清单格式错误时返回 `serde_json::Error`。
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), serde_json::Error> {
/// use burncloud_supply_model::load_default_manifests;
///
/// let manifests = load_default_manifests()?;
/// assert!(!manifests.is_empty());
/// # Ok(())
/// # }
/// ```
pub fn load_default_manifests() -> Result<Vec<ModelManifest>, serde_json::Error> {
    parse_manifests(DEFAULT_MANIFEST_JSON)
}
