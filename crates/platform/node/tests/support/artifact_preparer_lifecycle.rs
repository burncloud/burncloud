use super::ArtifactPreparer;

#[tokio::test]
async fn new_preparer_starts_idle_with_no_manager_or_references() {
    let preparer = ArtifactPreparer::new();
    let lifecycle = preparer.lifecycle.lock().await;

    assert_eq!(lifecycle.ref_count, 0);
    assert!(lifecycle.download_manager.is_none());
}
