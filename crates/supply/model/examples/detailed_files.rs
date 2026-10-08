use burncloud_service_models::get_model_files;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = "deepseek-ai/DeepSeek-OCR";

    println!("=== 模型文件列表获取测试 ===\n");
    println!("模型 ID: {}\n", model_id);

    let files = get_model_files(model_id).await?;

    println!("共找到 {} 个文件\n", files.len());

    println!("二维数组格式示例 (前3个文件):\n");
    for (i, file) in files.iter().take(3).enumerate() {
        println!("文件 {}: {:?}", i + 1, file);
        println!("  [0] 类型: {}", file[0]);
        println!("  [1] OID:  {}", file[1]);
        println!("  [2] 大小: {}", file[2]);
        println!("  [3] 路径: {}\n", file[3]);
    }

    // 统计
    let total_size: i64 = files.iter().filter_map(|f| f[2].parse::<i64>().ok()).sum();

    println!("统计信息:");
    println!("  总文件数: {}", files.len());
    println!(
        "  总大小:   {:.2} GB",
        total_size as f64 / 1024.0 / 1024.0 / 1024.0
    );

    // 按目录分组
    println!("\n目录结构:");
    let mut dirs = std::collections::HashSet::new();
    for file in &files {
        let path = &file[3];
        if let Some(idx) = path.rfind('/') {
            let dir = &path[..idx];
            dirs.insert(dir);
        }
    }
    for dir in &dirs {
        let count = files.iter().filter(|f| f[3].starts_with(dir)).count();
        println!("  {} ({} 个文件)", dir, count);
    }

    Ok(())
}
