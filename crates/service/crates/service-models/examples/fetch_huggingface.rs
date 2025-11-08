use burncloud_service_models::ModelService;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("正在从 HuggingFace API 获取模型列表...\n");

    // 获取模型列表
    let models = ModelService::fetch_from_huggingface().await?;

    println!("成功获取 {} 个模型\n", models.len());

    // 显示前 5 个模型
    for (i, model) in models.iter().take(5).enumerate() {
        println!("模型 {}:", i + 1);
        println!("  ID: {}", model.id);
        println!("  Likes: {:?}", model.likes);
        println!("  Downloads: {:?}", model.downloads);
        println!("  Pipeline: {:?}", model.pipeline_tag);
        println!("  Tags: {:?}", model.tags);
        println!();
    }

    Ok(())
}
