use burncloud_download::DownloadManager;
use burncloud_service_models::build_download_url;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 使用一个小文件进行测试
    println!("=== 测试下载小文件 ===");
    let model_id = "Qwen/Qwen2.5-7B-Instruct-GGUF";
    let path = "README.md"; // 通常 README 文件很小

    println!("模型: {}", model_id);
    println!("文件: {}", path);

    let url = build_download_url(model_id, path).await?;
    println!("URL: {}\n", url);

    // 创建 DownloadManager 实例（只创建一次）
    println!("初始化下载管理器...");
    let manager = DownloadManager::new().await?;

    // 使用 manager 直接添加下载
    println!("开始下载...");
    let final_path = manager.download(&url, model_id).await?;
    println!("下载完成，文件保存位置: {}", final_path.display());

    Ok(())
}
