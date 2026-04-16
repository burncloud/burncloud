//! 使用示例
//!
//! 运行方式:
//! ```bash
//! cargo run --example usage
//! ```

use burncloud_service_setting::{SettingDatabase, SettingService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== BurnCloud 设置服务使用示例 ===\n");

    // 创建数据库实例
    let db = SettingDatabase::new().await?;
    println!("✓ 服务初始化成功\n");

    // 1. 设置配置项
    println!("1. 设置配置项:");
    SettingService::set(&db, "app_name", "BurnCloud").await?;
    println!("   设置 app_name = BurnCloud");

    SettingService::set(&db, "version", "0.1.17").await?;
    println!("   设置 version = 0.1.17");

    SettingService::set(&db, "theme", "dark").await?;
    println!("   设置 theme = dark\n");

    // 2. 获取配置值
    println!("2. 获取配置值:");
    if let Some(app_name) = SettingService::get(&db, "app_name").await? {
        println!("   app_name = {}", app_name);
    }

    if let Some(version) = SettingService::get(&db, "version").await? {
        println!("   version = {}", version);
    }

    match SettingService::get(&db, "not_exist").await? {
        Some(value) => println!("   not_exist = {}", value),
        None => println!("   not_exist = (不存在)"),
    }
    println!();

    // 3. 更新配置项
    println!("3. 更新配置项:");
    SettingService::set(&db, "theme", "light").await?;
    if let Some(theme) = SettingService::get(&db, "theme").await? {
        println!("   更新后 theme = {}\n", theme);
    }

    // 4. 列出所有配置
    println!("4. 列出所有配置:");
    let settings = SettingService::list_all(&db).await?;
    for setting in &settings {
        println!("   {} = {}", setting.name, setting.value);
    }
    println!("   总计: {} 项配置\n", settings.len());

    // 5. 删除配置项
    println!("5. 删除配置项:");
    SettingService::delete(&db, "version").await?;
    println!("   已删除 version");

    let settings = SettingService::list_all(&db).await?;
    println!("   剩余配置数量: {}\n", settings.len());

    db.close().await?;
    println!("✓ 服务已关闭");

    Ok(())
}
