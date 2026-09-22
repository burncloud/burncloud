/// 从 URL 中提取文件名
pub(crate) fn extract_filename_from_url(url: &str) -> Option<String> {
    // 移除查询参数
    let url_without_query = url.split('?').next()?;

    // 提取路径的最后一部分
    let filename = url_without_query.split('/').next_back()?;

    // 如果文件名为空，返回 None
    if filename.is_empty() {
        None
    } else {
        Some(filename.to_string())
    }
}
