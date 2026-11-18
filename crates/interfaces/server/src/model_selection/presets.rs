//! 原样保留 v0.2 的十八个预置名称、顺序和绑定仓库。

use serde::Serialize;

use super::sources::{version_id, ModelResource, Source};

// 预置目录只表示绑定关系，仓库是否可访问仍由状态接口判断。
const ENTRIES: [(&str, &str); 18] = [
    ("DeepSeek V4.1 Flash", "deepseek-ai/DeepSeek-V4.1-Flash"),
    ("DeepSeek V4 Pro", "deepseek-ai/DeepSeek-V4-Pro"),
    ("GLM‑5.3", "ZhipuAI/GLM-5.3"),
    ("GLM‑5.3‑Flash", "ZhipuAI/GLM-5.3-Flash"),
    ("Kimi K3", "moonshotai/Kimi-K3"),
    ("MiniMax M3.1 Flash", "MiniMax/MiniMax-M3.1-Flash"),
    ("Qwen3.8-27B", "Qwen/Qwen3.8-27B"),
    ("Qwen3-8B", "Qwen/Qwen3-8B"),
    ("Qwen3-Coder-Next", "Qwen/Qwen3-Coder-Next"),
    ("Qwen3.8-2.4T-A95B", "Qwen/Qwen3.8-2.4T-A95B"),
    ("Qwen3.8-Flash-Next", "Qwen/Qwen3.8-Flash-Next"),
    ("Qwen-Image-2512", "Qwen/Qwen-Image-2512"),
    ("Wan2.2-TI2V-5B", "Wan-AI/Wan2.2-TI2V-5B"),
    ("MiMo-V2.6-Flash-RL", "XiaomiMiMo/MiMo-V2.6-Flash-RL"),
    ("gpt-oss-20b", "openai-mirror/gpt-oss-20b"),
    ("gpt-oss-120b", "openai-mirror/gpt-oss-120b"),
    ("Gemma 4 E4B IT", "google/gemma-4-E4B-it"),
    ("Gemma 4 31B", "google/gemma-4-31B"),
];

#[derive(Clone, Debug, Serialize)]
pub(super) struct Preset {
    id: String,
    name: &'static str,
    preset: bool,
    #[serde(flatten)]
    pub(super) resource: ModelResource,
}

/// 返回按原顺序排列的预置资源。
///
/// # Examples
/// 第一项为 `DeepSeek V4.1 Flash`。
///
/// # Notes
/// 不因上游故障替换为模拟目录。
pub(super) fn all() -> Vec<Preset> {
    ENTRIES
        .iter()
        .map(|(name, model)| Preset {
            id: version_id(model),
            name,
            preset: true,
            resource: ModelResource {
                model: (*model).to_string(),
                source: Source::ModelScope,
                revision: "master".into(),
            },
        })
        .collect()
}

/// 按预置标识返回绑定资源。
///
/// # Examples
/// `qwen3-8b` 对应 `Qwen/Qwen3-8B`。
///
/// # Notes
/// 未知标识返回 `None`，不会触发搜索。
pub(super) fn find(id: &str) -> Option<Preset> {
    all().into_iter().find(|preset| preset.id == id)
}
