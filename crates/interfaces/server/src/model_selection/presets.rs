//! 保留导入表格的名称、顺序、链接和最低及推荐硬件配置。

use serde::{Deserialize, Serialize, Serializer};
use std::sync::OnceLock;

use super::sources::ModelResource;

#[derive(Clone, Debug, Deserialize, Serialize)]
struct HardwareRequirements {
    #[serde(serialize_with = "serialize_capacity")]
    vram_min: String,
    #[serde(serialize_with = "serialize_capacity")]
    ram_min: String,
    #[serde(serialize_with = "serialize_capacity")]
    disk_min: String,
    #[serde(serialize_with = "serialize_capacity")]
    vram_recommended: String,
    #[serde(serialize_with = "serialize_capacity")]
    ram_recommended: String,
    #[serde(serialize_with = "serialize_capacity")]
    disk_recommended: String,
}

fn serialize_capacity<S>(value: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let formatted = value
        .strip_suffix('G')
        .and_then(|number| number.trim().parse::<f64>().ok())
        .filter(|number| number.is_finite())
        .map(|number| format!("{number:.2}G"))
        .unwrap_or_else(|| value.to_owned());
    serializer.serialize_str(&formatted)
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub(super) struct Preset {
    id: String,
    name: String,
    preset: bool,
    resource_url: String,
    hardware: HardwareRequirements,
    #[serde(flatten)]
    pub(super) resource: ModelResource,
}

/// 返回按原顺序排列的预置资源。
///
/// # Examples
/// 第一项为 `llama3.1-8B`，包含原表链接和六项硬件要求。
///
/// # Notes
/// 内嵌数据只解析一次；格式错误属于构建数据错误，不回退到旧目录。
pub(super) fn all() -> &'static [Preset] {
    static PRESETS: OnceLock<Vec<Preset>> = OnceLock::new();
    PRESETS.get_or_init(|| {
        serde_json::from_str(include_str!("presets.json"))
            .unwrap_or_else(|error| panic!("内嵌模型配置表格式错误：{error}"))
    })
}

/// 按预置标识返回绑定资源。
///
/// # Examples
/// `qwen3-8b` 对应原表链接中的 `Qwen/Qwen3-8b`。
///
/// # Notes
/// 未知标识返回 `None`，不会触发搜索。
pub(super) fn find(id: &str) -> Option<Preset> {
    all().iter().find(|preset| preset.id == id).cloned()
}
