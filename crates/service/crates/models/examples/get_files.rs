use burncloud_service_models::get_model_files;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model_id = "deepseek-ai/DeepSeek-OCR";

    println!("正在获取模型文件列表: {}\n", model_id);

    let files = get_model_files(model_id).await?;

    println!("共找到 {} 个文件:\n", files.len());

    for (i, file) in files.iter().enumerate() {
        println!(
            "{}. {} | {} bytes | {}",
            i + 1,
            file[3], // path
            file[2], // size
            file[0], // type
        );
    }

    Ok(())
}
