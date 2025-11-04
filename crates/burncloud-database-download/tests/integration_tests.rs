#[cfg(test)]
mod tests {
    use burncloud_database_download::DownloadDB;

    #[tokio::test]
    async fn test_download_operations() {
        let db = DownloadDB::new().await.unwrap();

        // 添加下载任务
        let gid = "test_gid_123";
        let uris = vec!["http://example.com/file1.zip".to_string()];
        db.add(gid, uris, Some("/test/download"), Some("file1.zip")).await.unwrap();

        // 获取任务
        let download = db.get(gid).await.unwrap().unwrap();
        assert_eq!(download.gid, gid);
        assert_eq!(download.status, "waiting");

        // 更新状态
        db.update_status(gid, "active").await.unwrap();

        // 更新进度
        db.update_progress(gid, 1024, 512).await.unwrap();

        // 验证更新
        let updated = db.get(gid).await.unwrap().unwrap();
        assert_eq!(updated.status, "active");
        assert_eq!(updated.completed_length, 1024);
        assert_eq!(updated.download_speed, 512);

        // 列出所有任务
        let all = db.list(None).await.unwrap();
        assert!(!all.is_empty());

        // 按状态列出任务
        let active = db.list(Some("active")).await.unwrap();
        assert_eq!(active.len(), 1);

        // 删除任务
        db.delete(gid).await.unwrap();
        let deleted = db.get(gid).await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_duplicate_uris_and_download_dir() {
        let db = DownloadDB::new().await.unwrap();

        // 添加第一个任务
        let gid1 = "test_gid_1";
        let uris = vec!["http://example.com/file.zip".to_string()];
        let download_dir = Some("/test/download");
        db.add(gid1, uris.clone(), download_dir, Some("file.zip")).await.unwrap();

        // 尝试添加相同uris和download_dir的任务，应该返回OK但不插入
        let gid2 = "test_gid_2";
        db.add(gid2, uris, download_dir, Some("file.zip")).await.unwrap();

        // 验证只有第一个任务存在
        let task1 = db.get(gid1).await.unwrap();
        let task2 = db.get(gid2).await.unwrap();

        assert!(task1.is_some());
        assert!(task2.is_none()); // 第二个任务不应该存在

        // 清理
        db.delete(gid1).await.unwrap();
    }
}