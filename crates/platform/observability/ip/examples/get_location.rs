use burncloud_service_ip::get_location;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 第一次调用：从网络查询并缓存
    println!("第一次查询位置...");
    let location = get_location().await?;
    println!("位置: {}", location);

    // 第二次调用：直接从缓存读取
    println!("\n第二次查询位置（从缓存）...");
    let location = get_location().await?;
    println!("位置: {}", location);

    Ok(())
}
