//! 只读模型下载目录；不启动 aria2，不向接口层泄露存储表类型。

use crate::{DownloadError, DownloadTaskStatus};
use burncloud_database::Database;
use burncloud_database_sys::DownloadDB;
use std::sync::Arc;

/// 一条完整变体身份的最新任务凭据；只有验证成功才返回路径和摘要。
#[derive(Debug)]
pub struct CatalogDownload {
    /// 真实仓库的文件名（含子目录）。
    pub variant_id: String,
    /// 下载请求的原始工件地址。
    pub artifact_uri: String,
    /// 下载请求固定的 SHA-256。
    pub checksum: String,
    /// 已知的文件长度；离线重建目录时使用。
    pub total_bytes: u64,
    /// 文件与校验记录共同决定的本地状态。
    pub status: DownloadTaskStatus,
    /// 已校验文件的本机路径。
    pub local_path: Option<String>,
    /// 已校验文件的实际摘要。
    pub actual_sha256: Option<String>,
}

/// 读取指定仓库修订的本地下载快照，按任务 ID 降序返回。
///
/// # Examples
/// ```no_run
/// # async fn example(db: std::sync::Arc<burncloud_database::Database>) -> Result<(), burncloud_download::DownloadError> {
/// let records = burncloud_download::catalog_downloads(db, "org/model", "modelscope", "master").await?;
/// # Ok(()) }
/// ```
/// # Notes
/// 即使仓库和修订相同，调用方仍须同时匹配变体、URI 和期望摘要。
pub async fn catalog_downloads(
    database: Arc<Database>,
    model: &str,
    source: &str,
    revision: &str,
) -> Result<Vec<CatalogDownload>, DownloadError> {
    let store = DownloadDB::new_shared_db(database).await?;
    let records = store.model_downloads(model, source, revision).await?;
    let mut downloads = Vec::new();
    for row in records {
        let (Some(variant_id), Some(artifact_uri), Some(checksum)) =
            (row.variant_id, row.artifact_uri, row.checksum)
        else {
            continue;
        };
        let mut status = crate::operations::status_from_record(&row.status)
            .unwrap_or(DownloadTaskStatus::Failed);
        let mut local_path = None;
        let mut actual_sha256 = None;
        if status == DownloadTaskStatus::Succeeded {
            let digest_valid = checksum
                .strip_prefix("sha256:")
                .is_some_and(|d| d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit()))
                && row.actual_sha256.as_ref() == Some(&checksum);
            let is_file = match row.final_path.as_deref() {
                Some(path) if digest_valid => {
                    match crate::utils::existing_database_file(path).await {
                        Ok(path) => path.is_some(),
                        Err(error) => {
                            tracing::warn!(%error, "已登记的模型文件不可读");
                            false
                        }
                    }
                }
                _ => false,
            };
            if is_file {
                local_path = row.final_path;
                actual_sha256 = row.actual_sha256;
            } else {
                status = DownloadTaskStatus::Failed;
            }
        }
        downloads.push(CatalogDownload {
            variant_id,
            artifact_uri,
            checksum,
            total_bytes: row.total_length.unwrap_or_default().max(0) as u64,
            status,
            local_path,
            actual_sha256,
        });
    }
    Ok(downloads)
}
