use thiserror::Error;

#[derive(Error, Debug)]
pub enum DownloadError {
    #[error("数据库错误: {0}")]
    Database(#[from] burncloud_database::DatabaseError),
    #[error("Aria2错误: {0}")]
    Aria2(#[from] burncloud_download_aria2::Aria2Error),
}

pub type Result<T> = std::result::Result<T, DownloadError>;
