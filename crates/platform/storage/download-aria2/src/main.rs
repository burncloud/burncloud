use burncloud_download_aria2::*;
use std::time::Duration;

#[tokio::main]
// 输入：无。
// 功能：启动 aria2 管理器并执行基础操作与下载流程测试。
// 错误：管理器、RPC 调用或测试操作失败时返回 Aria2Error。
async fn main() -> Aria2Result<()> {
    // 输出测试启动信息并初始化管理器
    println!("🚀 启动 BurnCloud Aria2 测试...");

    // 使用快速启动
    let mut manager = quick_start().await?;
    println!("✅ Aria2 管理器启动成功");

    // 获取 RPC 客户端
    // 获取客户端后执行 RPC 相关测试
    if let Some(client) = manager.create_rpc_client() {
        println!("📡 获取到 RPC 客户端");

        // 测试基本功能
        test_basic_operations(&client).await?;

        // 测试下载功能
        test_download(&client).await?;
    }

    // 等待一会儿让操作完成
    // 等待异步测试操作完成
    tokio::time::sleep(Duration::from_secs(2)).await;

    // 关闭管理器
    manager.shutdown().await?;
    println!("🛑 测试完成，管理器已关闭");

    Ok(())
}

// 输入：已连接的 aria2 RPC 客户端。
// 功能：查询全局统计信息和活跃任务，验证基础 RPC 功能。
// 错误：函数当前忽略查询失败，始终返回 Ok。
async fn test_basic_operations(client: &Aria2RpcClient) -> Aria2Result<()> {
    // 输出基础操作测试的开始信息
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

    // 完成基础操作测试
    Ok(())
}

// 输入：已连接的 aria2 RPC 客户端。
// 功能：创建一个下载任务并查询其初始状态。
// 错误：添加下载任务失败时仅输出错误，函数当前始终返回 Ok。
async fn test_download(client: &Aria2RpcClient) -> Aria2Result<()> {
    // 输出下载功能测试的开始信息
    println!("📥 测试下载功能...");

    // 添加一个小文件下载测试
    let test_url = "https://mirrors.tuna.tsinghua.edu.cn/ubuntu-releases/20.04.6/ubuntu-20.04.6-live-server-amd64.iso";

    // 构造下载任务配置
    let options = DownloadOptions {
        dir: Some("./downloads".to_string()),
        out: None,
        split: None,
        max_connection_per_server: None,
        continue_download: None,
        allow_overwrite: Some(true),     // 开启覆盖式下载
        auto_file_renaming: Some(false), // 关闭文件自动重命名
    };
    // 提交下载任务并根据结果输出状态
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

    // 完成下载功能测试
    Ok(())
}
