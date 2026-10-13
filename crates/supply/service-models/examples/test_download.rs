use burncloud_download::DownloadManager;
use burncloud_service_models::build_download_url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let final_path = manager.download(&url, model_id).await?;
    println!("下载完成，文件保存位置: {}", final_path.display());

    Ok(())
}
