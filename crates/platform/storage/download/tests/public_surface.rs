use burncloud_download::DownloadManager;

#[test]
fn requested_manager_entry_points_are_exposed() {
    let _constructor = DownloadManager::new;
    let _download = DownloadManager::download;
}
