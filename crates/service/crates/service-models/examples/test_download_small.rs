use burncloud_service_models::{get_data_dir, build_download_url};
use burncloud_download::DownloadManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 测试获取数据目录
    println!("=== 测试数据目录 ===");
    let dir = get_data_dir().await?;
    println!("数据目录: {}\n", dir);

    // 使用一个小文件进行测试
    println!("=== 测试下载小文件 ===");
    let model_id = "Qwen/Qwen2.5-7B-Instruct-GGUF";
    let path = "README.md";  // 通常 README 文件很小

    println!("模型: {}", model_id);
    println!("文件: {}", path);

    let url = build_download_url(model_id, path).await?;
    println!("URL: {}\n", url);

    // 创建 DownloadManager 实例（只创建一次）
    println!("初始化下载管理器...");
    let manager = DownloadManager::new().await?;

    // 使用 manager 直接添加下载
    println!("开始下载...");
    let download_dir = format!("{}/{}", dir, model_id);
    let gid = manager.add_download(&url, Some(&download_dir)).await?;

    println!("下载已启动!");
    println!("GID: {}", gid);
    println!("\n正在下载，等待完成...\n");

    // 轮询下载状态，直到完成或出错
    loop {
        match manager.get_status(&gid).await {
            Ok(status) => {
                let total: i64 = status.total_length.parse().unwrap_or(0);
                let completed: i64 = status.completed_length.parse().unwrap_or(0);
                let speed: i64 = status.download_speed.parse().unwrap_or(0);

                let progress = if total > 0 {
                    completed as f64 / total as f64 * 100.0
                } else {
                    0.0
                };

                // 格式化显示
                let total_kb = total as f64 / 1024.0;
                let completed_kb = completed as f64 / 1024.0;
                let speed_kb = speed as f64 / 1024.0;

                print!("\r状态: {} | 进度: {:.2}% | 已下载: {:.2} KB / {:.2} KB | 速度: {:.2} KB/s",
                    status.status, progress, completed_kb, total_kb, speed_kb);

                // 刷新输出
                use std::io::Write;
                std::io::stdout().flush()?;

                // 检查是否完成
                if status.status == "complete" {
                    println!("\n\n✅ 下载完成!");
                    println!("文件保存位置: {}/{}/{}", dir, model_id, path);
                    break;
                } else if status.status == "error" {
                    println!("\n\n❌ 下载失败");
                    return Err("下载失败".into());
                }
            }
            Err(e) => {
                println!("\n\n❌ 获取下载状态失败: {}", e);
                return Err(e.into());
            }
        }

        // 每 1 秒查询一次（小文件可以更频繁）
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }

    Ok(())
}
