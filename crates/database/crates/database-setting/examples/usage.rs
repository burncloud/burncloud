//! 使用示例
//!
//! 运行方式:
//! ```bash
//! cargo run --example usage
//! ```

use burncloud_database_setting::SettingDatabase;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== BurnCloud 设置数据库使用示例 ===\n");

    // 创建数据库实例
    let db = SettingDatabase::new().await?;
    println!("✓ 数据库初始化成功\n");

    // 1. 添加设置项
    println!("1. 添加设置项:");
    db.set("theme", "dark").await?;
    println!("   设置 theme = dark");

    db.set("language", "zh-CN").await?;
    println!("   设置 language = zh-CN");

    db.set("auto_update", "true").await?;
    println!("   设置 auto_update = true\n");

    // 2. 获取设置值
    println!("2. 获取设置值:");
    if let Some(theme) = db.get("theme").await? {
        println!("   theme = {}", theme);
    }

    if let Some(language) = db.get("language").await? {
        println!("   language = {}", language);
    }

    match db.get("not_exist").await? {
        Some(value) => println!("   not_exist = {}", value),
        None => println!("   not_exist = (不存在)"),
    }
    println!();

    // 3. 更新设置项
    println!("3. 更新设置项:");
    db.set("theme", "light").await?;
    if let Some(theme) = db.get("theme").await? {
        println!("   更新后 theme = {}\n", theme);
    }

    // 4. 列出所有设置
    println!("4. 列出所有设置:");
    let settings = db.list_all().await?;
    for setting in settings {
        println!("   {} = {}", setting.name, setting.value);
    }
    println!();

    // 5. 删除设置项
    println!("5. 删除设置项:");
    db.delete("auto_update").await?;
    println!("   已删除 auto_update");

    let settings = db.list_all().await?;
    println!("   剩余设置数量: {}\n", settings.len());

    // 关闭数据库
    db.close().await?;
    println!("✓ 数据库已关闭");

    Ok(())
}
