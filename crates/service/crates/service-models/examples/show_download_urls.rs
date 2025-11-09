use burncloud_service_models::{get_data_dir, build_download_url};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("数据目录: {}\n", get_data_dir().await?);

    let model_id = "Qwen/Qwen2.5-7B-Instruct-GGUF";
    let files = vec![
        "qwen2.5-7b-instruct-q2_k.gguf",
        "qwen2.5-7b-instruct-q4_0-00001-of-00002.gguf",
        "qwen2.5-7b-instruct-q8_0-00001-of-00003.gguf",
    ];

    println!("下载 URL 示例:\n");
    for file in files {
        let url = build_download_url(model_id, file).await?;
        println!("{}", file);
        println!("  -> {}\n", url);
    }

    Ok(())
}
