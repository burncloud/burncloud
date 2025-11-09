use burncloud_service_models::{get_model_files, filter_gguf_files, get_data_dir, build_download_url};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = "Qwen/Qwen2.5-7B-Instruct-GGUF";

    println!("=== 模型下载工作流演示 ===\n");

    // 1. 获取数据目录
    let data_dir = get_data_dir().await?;
    println!("1. 数据目录: {}", data_dir);
    println!("   模型存储: {}/{}\n", data_dir, model_id);

    // 2. 获取所有文件
    println!("2. 正在获取文件列表...");
    let all_files = get_model_files(model_id).await?;
    println!("   总文件数: {}\n", all_files.len());

    // 3. 筛选 GGUF 文件
    let gguf_files = filter_gguf_files(&all_files);
    println!("3. GGUF 文件数: {}\n", gguf_files.len());

    // 4. 显示前 3 个 GGUF 文件的下载信息
    println!("4. 示例下载 URL (前3个):\n");
    for (i, file) in gguf_files.iter().take(3).enumerate() {
        let path = &file[3];
        let size_gb = file[2].parse::<f64>().unwrap_or(0.0) / 1024.0 / 1024.0 / 1024.0;
        let url = build_download_url(model_id, path).await?;

        println!("   [{}] {}", i + 1, path);
        println!("       大小: {:.2} GB", size_gb);
        println!("       存储: {}/{}/{}", data_dir, model_id, path);
        println!("       URL: {}\n", url);
    }

    println!("提示: 使用 download_model_file(model_id, path) 可开始下载");

    Ok(())
}
