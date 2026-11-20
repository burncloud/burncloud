use burncloud_download::{
    DownloadManager, DownloadTaskHandle, DownloadTaskSnapshot, DownloadTaskStatus,
};

#[test]
fn requested_manager_entry_points_are_exposed() {
    let _constructor = DownloadManager::new;
    let _download = DownloadManager::download;
    let _start_download = DownloadManager::start_download;
    let _get_task = DownloadManager::get_task;
    let _cancel_task = DownloadManager::cancel_task;
    let _wait_for_task = DownloadManager::wait_for_task;
    let _handle = std::mem::size_of::<DownloadTaskHandle>();
    let _snapshot = std::mem::size_of::<DownloadTaskSnapshot>();
    let _status = DownloadTaskStatus::Queued;
}
