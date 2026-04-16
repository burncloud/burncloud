use burncloud_download::DownloadManager;
use burncloud_service_models::{build_download_url, get_data_dir};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 测试获取数据目录
    println!("=== 测试数据目录 ===");
    let dir = get_data_dir().await?;
    println!("数据目录: {}\n", dir);

    // 测试构建 URL
    println!("=== 测试 URL 构建 ===");
    let model_id = "Qwen/Qwen2.5-7B-Instruct-GGUF";
    let path = "qwen2.5-7b-instruct-q4_0-00001-of-00002.gguf";
    let url = build_download_url(model_id, path).await?;
    println!("模型: {}", model_id);
    println!("文件: {}", path);
    println!("URL: {}\n", url);

    // 测试下载（需要确认是否真的下载）
    println!("=== 测试下载 ===");
    println!("准备下载文件...");
    println!("警告: 这将开始真实下载，文件大小约 3.7 GB");
    println!("如需取消，请按 Ctrl+C\n");

    // 暂停 5 秒给用户取消的机会
    println!("5 秒后开始下载...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // 创建 DownloadManager 实例（只创建一次）
    let manager = DownloadManager::new().await?;

    // 使用 manager 直接添加下载
    let download_dir = format!("{}/{}", dir, model_id);
    let gid = manager.add_download(&url, Some(&download_dir)).await?;

    println!("下载已启动!");
    println!("GID: {}", gid);
    println!("下载目录: {}", download_dir);
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
                let total_mb = total as f64 / 1024.0 / 1024.0;
                let completed_mb = completed as f64 / 1024.0 / 1024.0;
                let speed_mb = speed as f64 / 1024.0 / 1024.0;

                print!(
                    "\r状态: {} | 进度: {:.2}% | 已下载: {:.2} MB / {:.2} MB | 速度: {:.2} MB/s",
                    status.status, progress, completed_mb, total_mb, speed_mb
                );

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

        // 每 2 秒查询一次
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    Ok(())
}
