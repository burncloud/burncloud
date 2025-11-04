use burncloud_download::DownloadManager;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("启动下载管理器...");

    // 创建管理器，自动恢复未完成的下载
    let manager = DownloadManager::new().await?;
    println!("下载管理器已启动，未完成任务已自动恢复");

    // 示例：添加下载任务
    let url = "https://mirrors.tuna.tsinghua.edu.cn/ubuntu-releases/20.04.6/ubuntu-20.04.6-live-server-amd64.iso";
    let gid = manager.add_download(url, None).await?;
    println!("添加下载任务: {}", gid);

    // 监控任务直到完成
    loop {
        let status = manager.get_status(&gid).await?;
        println!("状态: {}, 进度: {}/{}, 速度: {}",
            status.status,
            status.completed_length,
            status.total_length,
            status.download_speed
        );

        if status.status == "complete" || status.status == "error" {
            println!("下载{}: {}",
                if status.status == "complete" { "完成" } else { "失败" },
                gid
            );
            break;
        }

        sleep(Duration::from_secs(3)).await;
    }

    Ok(())
}