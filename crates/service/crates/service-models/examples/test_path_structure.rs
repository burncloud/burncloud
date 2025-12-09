use burncloud_service_models::{build_download_url, get_data_dir};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = "Qwen/Qwen2.5-7B-Instruct-GGUF";
    let filename = "qwen2.5-7b-instruct-q4_0-00001-of-00002.gguf";

    let base_dir = get_data_dir().await?;
    let download_dir = format!("{}/{}", base_dir, model_id);
    let url = build_download_url(model_id, filename).await?;

    println!("=== 下载路径结构测试 ===\n");
    println!("模型 ID: {}", model_id);
    println!("文件名: {}", filename);
    println!("\n基础目录: {}", base_dir);
    println!("下载目录: {}", download_dir);
    println!("完整路径: {}/{}\n", download_dir, filename);
    println!("下载 URL: {}", url);

    Ok(())
}
