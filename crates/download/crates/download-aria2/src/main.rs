use burncloud_download_aria2::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Aria2Result<()> {
    println!("🚀 启动 BurnCloud Aria2 测试...");

    // 使用快速启动
    let mut manager = quick_start().await?;
    println!("✅ Aria2 管理器启动成功");

    // 获取 RPC 客户端
    if let Some(client) = manager.create_rpc_client() {
        println!("📡 获取到 RPC 客户端");

        // 测试基本功能
        test_basic_operations(&client).await?;

        // 测试下载功能
        test_download(&client).await?;
    }

    // 等待一会儿让操作完成
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 关闭管理器
    manager.shutdown().await?;
    println!("🛑 测试完成，管理器已关闭");

    Ok(())
}

async fn test_basic_operations(client: &Aria2RpcClient) -> Aria2Result<()> {
    println!("🔍 测试基本操作...");

    // 获取全局统计
    if let Ok(stat) = client.get_global_stat().await {
        println!("  - 活跃下载: {}", stat.num_active);
        println!("  - 等待下载: {}", stat.num_waiting);
        println!("  - 下载速度: {}", stat.download_speed);
    }

    // 获取活跃任务
    if let Ok(active) = client.tell_active().await {
        println!("  - 当前活跃任务数: {}", active.len());
    }

    Ok(())
}

async fn test_download(client: &Aria2RpcClient) -> Aria2Result<()> {
    println!("📥 测试下载功能...");

    // 添加一个小文件下载测试
    let test_url = "https://mirrors.tuna.tsinghua.edu.cn/ubuntu-releases/20.04.6/ubuntu-20.04.6-live-server-amd64.iso";

    let options = DownloadOptions {
        dir: Some("./downloads".to_string()),
        out: None,
        split: None,
        max_connection_per_server: None,
        continue_download: None,
    };
    match client
        .add_uri(vec![test_url.to_string()], Some(options))
        .await
    {
        Ok(gid) => {
            println!("  - 添加下载任务成功，GID: {}", gid);

            // 等待一会儿
            tokio::time::sleep(Duration::from_secs(1)).await;

            // 检查下载状态
            if let Ok(status) = client.tell_status(&gid).await {
                println!("  - 任务状态: {}", status.status);
                println!("  - 总大小: {}", status.total_length);
                println!("  - 已完成: {}", status.completed_length);
            }
        }
        Err(e) => println!("  - 添加下载任务失败: {}", e),
    }

    Ok(())
}
