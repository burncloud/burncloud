use burncloud_service_models::{filter_gguf_files, get_model_files};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 测试包含 GGUF 文件的模型
    let model_id = "Qwen/Qwen2.5-7B-Instruct-GGUF";

    println!("正在获取模型文件: {}\n", model_id);

    let all_files = get_model_files(model_id).await?;
    println!("总文件数: {}\n", all_files.len());

    let gguf_files = filter_gguf_files(&all_files);
    println!("GGUF 文件数: {}\n", gguf_files.len());

    println!("GGUF 文件列表:\n");
    for (i, file) in gguf_files.iter().enumerate() {
        let size_gb = file[2].parse::<f64>().unwrap_or(0.0) / 1024.0 / 1024.0 / 1024.0;
        println!("{}. {}", i + 1, file[3]);
        println!("   大小: {:.2} GB", size_gb);
        println!("   OID:  {}\n", file[1]);
    }

    Ok(())
}
