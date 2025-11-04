use burncloud_database_models::{ModelDatabase, ModelInfo};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建数据库实例
    let db = ModelDatabase::new().await?;

    // 创建一个示例模型
    let model = ModelInfo {
        model_id: "openai-community/gpt4".to_string(),
        private: false,
        pipeline_tag: Some("text-generation".to_string()),
        library_name: Some("transformers".to_string()),
        model_type: Some("gpt2".to_string()),
        downloads: 1000000,
        likes: 5000,
        sha: Some("abc123".to_string()),
        last_modified: Some("2024-01-01 12:00:00".to_string()),
        gated: false,
        disabled: false,
        tags: r#"["transformers", "pytorch", "text-generation", "gpt2"]"#.to_string(),
        config: r#"{"architectures": ["GPT2LMHeadModel"], "model_type": "gpt2"}"#.to_string(),
        widget_data: "[]".to_string(),
        card_data: "{}".to_string(),
        transformers_info: "{}".to_string(),
        siblings: r#"[{"rfilename": "config.json"}, {"rfilename": "model.safetensors"}]"#.to_string(),
        spaces: "[]".to_string(),
        safetensors: "{}".to_string(),
        used_storage: 548000000,
        filename: Some("model.safetensors".to_string()),
        size: 548000000,
        created_at: "2024-01-01 10:00:00".to_string(),
        updated_at: "2024-01-01 12:00:00".to_string(),
    };

    // 添加模型
    db.add_model(&model).await?;
    println!("✅ 模型添加成功");

    // 获取模型
    if let Some(retrieved_model) = db.get_model("openai-community/gpt3").await? {
        println!("📋 获取到模型: {}", retrieved_model.model_id);
        println!("   管道类型: {:?}", retrieved_model.pipeline_tag);
        println!("   下载量: {}", retrieved_model.downloads);
        println!("   文件名: {:?}", retrieved_model.filename);
        println!("   文件大小: {} 字节", retrieved_model.size);
    }

    // 获取所有模型
    let all_models = db.list_models().await?;
    println!("📊 总共有 {} 个模型", all_models.len());

    // 按管道类型搜索
    let text_gen_models = db.search_by_pipeline("text-generation").await?;
    println!("🔍 找到 {} 个文本生成模型", text_gen_models.len());

    // 获取热门模型
    let popular_models = db.get_popular_models(5).await?;
    println!("🔥 热门模型前5名:");
    for (i, model) in popular_models.iter().enumerate() {
        println!("   {}. {} (下载量: {})", i + 1, model.model_id, model.downloads);
    }

    // 关闭数据库
    db.close().await?;
    println!("🔒 数据库连接已关闭");

    Ok(())
}