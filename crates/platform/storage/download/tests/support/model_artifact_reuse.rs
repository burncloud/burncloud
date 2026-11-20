#[tokio::test]
#[expect(
    clippy::panic_in_result_fn,
    reason = "测试通过 Result 传播准备错误，断言用于检查持久化契约"
)]
async fn persisted_progress_and_verified_file_are_reused_without_aria2(
) -> Result<(), Box<dyn std::error::Error>> {
    let temp = tempfile::tempdir()?;
    let directory = temp.path().canonicalize()?;
    let database = burncloud_database::create_database_with_url(&burncloud_database::sqlite_url(
        &temp
            .path()
            .join("test.db")
            .to_string_lossy()
            .replace('\\', "/"),
        cfg!(windows),
        true,
    ))
    .await?;
    let store = burncloud_database_sys::DownloadDB::new_with_db(database).await?;
    let manager = super::DownloadManager {
        aria2: std::sync::Arc::new(burncloud_download_aria2::Aria2Manager::new()),
        db: std::sync::Arc::new(store),
        monitor_states: Default::default(),
        start_lock: Default::default(),
        task_lock: Default::default(),
        cancellation: Default::default(),
        monitor: None,
    };
    // 只查询临时目录中的持久化记录，不启动网络传输。
    let url = "https://huggingface.co/org/model/resolve/main/model.gguf";
    let dir = directory.to_string_lossy();
    let id = manager.db.insert_task("gid", url, &dir, "model").await?;
    let checksum =
        "sha256:84d89877f0d4041efb6bf91a16f0248f2fd573e6af05c19f96bedb9f882f7882".to_owned();
    manager
        .set_task_identity(
            id,
            "org/model",
            "huggingface",
            "main",
            "model.gguf",
            url,
            &checksum,
        )
        .await?;
    manager
        .db
        .update_task_progress(id, "gid", "active", (10, 5, 2))
        .await?;
    let snapshot = manager.get_task(id).await?.ok_or("快照不存在")?;
    assert_eq!(
        (
            snapshot.completed_bytes,
            snapshot.total_bytes,
            snapshot.download_speed
        ),
        (5, Some(10), 2)
    );
    let path = directory.join("model");
    tokio::fs::write(&path, b"0123456789").await?;
    manager
        .db
        .mark_completed(id, "gid", &path.to_string_lossy(), 10, 10)
        .await?;
    manager.mark_task_verified(id, &checksum).await?;
    let record = manager
        .find_reusable_record(url, &dir, "model")
        .await?
        .ok_or("已校验文件应可复用")?;
    let reused = super::handle_from_record(&record)?;
    assert_eq!(reused.task_id, id);
    assert_eq!(reused.status, super::DownloadTaskStatus::Succeeded);
    assert_eq!(manager.wait_for_task(id).await?, path);
    assert!(manager.cancel_task(id).await.is_err());
    Ok(())
}
