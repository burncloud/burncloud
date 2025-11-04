use burncloud_client_models::app_state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 测试数据库数据加载...");

    // 创建 AppState
    println!("1. 创建 AppState...");
    let mut app_state = AppState::new().await?;

    // 加载数据
    println!("2. 加载数据...");
    app_state.load_data().await?;

    // 显示结果
    println!("\n📊 数据加载结果:");
    println!("已安装模型数量: {}", app_state.installed_models.len());
    println!("可用模型数量: {}", app_state.available_models.len());

    println!("\n🔍 已安装模型详细信息:");
    for (i, model) in app_state.installed_models.iter().enumerate() {
        println!("  {}. {} - {} (状态: {:?})",
            i + 1,
            model.model.display_name,
            model.model.name,
            model.status
        );
    }

    println!("\n🔍 可用模型详细信息:");
    for (i, model) in app_state.available_models.iter().enumerate() {
        println!("  {}. {} - {} ({})",
            i + 1,
            model.model.display_name,
            model.model.name,
            model.model.provider
        );
    }

    println!("\n✅ 测试完成!");
    Ok(())
}