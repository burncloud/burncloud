use burncloud_service_models::{filter_gguf_files, get_model_files};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = "Qwen/Qwen2.5-7B-Instruct-GGUF";

    let files = get_model_files(model_id).await?;
    let gguf_files = filter_gguf_files(&files);

    println!("模型: {}", model_id);
    println!("总文件: {} | GGUF文件: {}\n", files.len(), gguf_files.len());

    for file in gguf_files {
        println!("{:?}", file);
    }

    Ok(())
}
