#![allow(clippy::panic_in_result_fn, reason = "测试断言用于报告生命周期失败")]

use super::ArtifactPreparer;

#[tokio::test]
async fn new_preparer_starts_idle_with_no_manager_or_references() {
    let preparer = ArtifactPreparer::new();
    let lifecycle = preparer.lifecycle.lock().await;

    assert_eq!(lifecycle.ref_count, 0);
    assert!(lifecycle.download_manager.is_none());
}

#[cfg_attr(
    not(windows),
    ignore = "真实 aria2 生命周期测试需要 Windows 可执行文件"
)]
#[tokio::test]
async fn manager_references_are_reused_and_released_in_order(
) -> Result<(), Box<dyn std::error::Error>> {
    let _database = super::test_database::isolated_database("lifecycle")?;
    let preparer = ArtifactPreparer::new();
    let first = preparer.acquire_manager().await?;
    let first_address = std::sync::Arc::as_ptr(&first);

    {
        let lifecycle = preparer.lifecycle.lock().await;
        assert_eq!(lifecycle.ref_count, 1);
        assert!(lifecycle.download_manager.is_some());
    }

    let second = preparer.acquire_manager().await?;
    assert_eq!(std::sync::Arc::as_ptr(&second), first_address);
    {
        let lifecycle = preparer.lifecycle.lock().await;
        assert_eq!(lifecycle.ref_count, 2);
    }

    drop(first);
    preparer.release_manager().await;
    {
        let lifecycle = preparer.lifecycle.lock().await;
        assert_eq!(lifecycle.ref_count, 1);
        assert!(lifecycle.download_manager.is_some());
    }

    drop(second);
    preparer.release_manager().await;
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let lifecycle = preparer.lifecycle.lock().await;
    assert_eq!(lifecycle.ref_count, 0);
    assert!(lifecycle.download_manager.is_none());
    _database.cleanup().await?;
    Ok(())
}
