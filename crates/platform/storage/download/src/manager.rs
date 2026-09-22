use burncloud_database_sys::DownloadDB;
use burncloud_download_aria2::Aria2Manager;
use std::sync::Arc;

pub struct DownloadManager {
    pub(crate) aria2: Arc<Aria2Manager>,
    pub(crate) db: Arc<DownloadDB>,
}
