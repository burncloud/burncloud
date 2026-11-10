//! 聊天页使用的模型目录适配。

use crate::domains::buyer::marketplace::model::{MarketplaceModel, MODEL_CATALOG};

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct ChatModel {
    pub(super) model: MarketplaceModel,
    pub(super) updated_at: &'static str,
}

pub(super) fn catalog() -> [ChatModel; 6] {
    MODEL_CATALOG.map(|model| ChatModel {
        updated_at: updated_at_for(model.id),
        model,
    })
}

fn updated_at_for(model_id: &str) -> &'static str {
    match model_id {
        "qwen-2.5-72b-instruct" => "2026-10-15",
        "deepseek-r1" => "2026-10-12",
        "deepseek-v3" => "2026-10-10",
        "claude-3-5-sonnet" => "2026-10-08",
        "llama-3.3-70b-instruct" => "2026-10-05",
        "glm-4-plus" => "2026-09-28",
        _ => "1970-01-01",
    }
}
