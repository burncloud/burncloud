//! 模型服务使用示例

use burncloud_service_models::{ModelInfo, ModelService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建服务实例
    let service = ModelService::new().await?;

    // 创建模型
    let model = ModelInfo {
        model_id: "test123/model".to_string(),
        private: false,
        pipeline_tag: Some("text-generation".to_string()),
        library_name: Some("transformers".to_string()),
        model_type: Some("gpt2".to_string()),
        downloads: 1000,
        likes: 50,
        sha: Some("abc123".to_string()),
        last_modified: Some("2024-01-01T00:00:00Z".to_string()),
        gated: false,
        disabled: false,
        tags: "[]".to_string(),
        config: "{}".to_string(),
        widget_data: "[]".to_string(),
        card_data: "{}".to_string(),
        transformers_info: "{}".to_string(),
        siblings: "[]".to_string(),
        spaces: "[]".to_string(),
        safetensors: "{}".to_string(),
        used_storage: 0,
        filename: None,
        size: 0,
        created_at: "2024-01-01T00:00:00Z".to_string(),
        updated_at: "2024-01-01T00:00:00Z".to_string(),
    };

    // 增：添加模型
    service.create(&model).await?;
    println!("✓ 模型已创建");

    // 查：获取模型
    if let Some(m) = service.get("test/model").await? {
        println!("✓ 查询到模型: {}", m.model_id);
    }

    // 改：更新模型
    let mut updated = model.clone();
    updated.downloads = 2000;
    service.update(&updated).await?;
    println!("✓ 模型已更新");

    // 查：列出所有模型
    let models = service.list().await?;
    println!("✓ 共有 {} 个模型", models.len());

    // 查：搜索
    let results = service.search_by_pipeline("text-generation").await?;
    println!("✓ 找到 {} 个文本生成模型", results.len());

    // 查：热门模型
    let popular = service.get_popular(10).await?;
    println!("✓ 获取到 {} 个热门模型", popular.len());

    // 删：删除模型
    // service.delete("test/model").await?;
    // println!("✓ 模型已删除");

    // 关闭服务
    service.close().await?;
    println!("✓ 服务已关闭");

    Ok(())
}
