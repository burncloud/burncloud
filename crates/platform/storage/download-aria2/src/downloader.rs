use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::Client;

use crate::constants::{get_burncloud_dir, ARIA2_BACKUP_URL, ARIA2_MAIN_URL};
use crate::error::{Aria2Error, Aria2Result};

// ============================================================================
// Aria2 下载功能
// ============================================================================

/// 下载 aria2 二进制文件
// 输入：无。
// 功能：下载 aria2 压缩包、解压 aria2c.exe 并返回其路径。
// 错误：客户端创建、文件下载、目录创建或解压失败时返回 DownloadError。
pub async fn download_aria2() -> Aria2Result<PathBuf> {
    // 创建带超时设置的 HTTP 客户端
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    // 准备本地存储目录和目标文件路径
    let target_dir = get_burncloud_dir();
    std::fs::create_dir_all(&target_dir)
        .map_err(|e| Aria2Error::DownloadError(format!("创建目录失败: {}", e)))?;

    let zip_path = target_dir.join("aria2.zip");
    let exe_path = target_dir.join("aria2c.exe");

    // 如果 exe 已存在，直接返回
    // 验证解压结果并返回二进制路径
    if exe_path.exists() {
        return Ok(exe_path);
    }

    // 尝试主链接下载
    match download_file(&client, ARIA2_MAIN_URL, &zip_path).await {
        Ok(_) => println!("从主链接下载成功"),
        Err(_) => {
            println!("主链接下载失败，尝试备用链接...");
            download_file(&client, ARIA2_BACKUP_URL, &zip_path)
                .await
                .map_err(|e| Aria2Error::DownloadError(format!("所有下载链接均失败: {}", e)))?;
            println!("从备用链接下载成功");
        }
    }

    // 解压 ZIP 文件
    extract_aria2(&zip_path, &target_dir)?;

    // 删除 ZIP 文件
    let _ = std::fs::remove_file(&zip_path);

    if exe_path.exists() {
        Ok(exe_path)
    } else {
        Err(Aria2Error::DownloadError(
            "解压后未找到 aria2c.exe".to_string(),
        ))
    }
}

// 输入：HTTP 客户端、下载地址和目标文件路径。
// 功能：请求远程资源并将响应内容写入指定文件。
// 错误：请求、HTTP 状态、读取响应或写文件失败时返回 DownloadError。
async fn download_file(client: &Client, url: &str, path: &Path) -> Aria2Result<()> {
    // 发送下载请求
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    // 校验 HTTP 响应状态
    if !response.status().is_success() {
        return Err(Aria2Error::DownloadError(format!(
            "HTTP错误: {}",
            response.status()
        )));
    }

    // 读取响应正文并写入目标文件
    let bytes = response
        .bytes()
        .await
        .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    std::fs::write(path, &bytes).map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    Ok(())
}

// 输入：ZIP 压缩包路径和 aria2 二进制目标目录。
// 功能：从压缩包中查找 aria2c.exe 并将其解压到目标目录。
// 错误：ZIP 打开、解析、条目读取或文件写入失败时返回 DownloadError。
fn extract_aria2(zip_path: &Path, target_dir: &Path) -> Aria2Result<()> {
    // 输入：ZIP 压缩包路径和 aria2 二进制目标目录。
    // 功能：从压缩包中查找 aria2c.exe 并将其解压到目标目录。
    // 错误：ZIP 打开、解析、条目读取或文件写入失败时返回 DownloadError。
    // 打开 ZIP 压缩包并创建归档读取器
    let file =
        std::fs::File::open(zip_path).map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

    // 遍历归档条目并提取 aria2c.exe
    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;

        if file.name().ends_with("aria2c.exe") {
            let mut out_file = std::fs::File::create(target_dir.join("aria2c.exe"))
                .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;
            std::io::copy(&mut file, &mut out_file)
                .map_err(|e| Aria2Error::DownloadError(e.to_string()))?;
            return Ok(());
        }
    }

    // 未找到所需的可执行文件
    Err(Aria2Error::DownloadError(
        "ZIP文件中未找到 aria2c.exe".to_string(),
    ))
}
