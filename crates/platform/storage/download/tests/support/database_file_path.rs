#![allow(
    clippy::panic_in_result_fn,
    reason = "测试用 Result 传播文件操作错误，并用断言验证行为"
)]

use std::path::PathBuf;

use super::existing_database_file;
use tempfile::tempdir;

#[tokio::test]
async fn returns_the_database_path_when_it_is_an_existing_file(
) -> Result<(), Box<dyn std::error::Error>> {
    let directory = tempdir()?;
    let file_path = directory.path().join("model.gguf");
    tokio::fs::write(&file_path, b"model").await?;
    let database_path = file_path.to_string_lossy().into_owned();

    let result = existing_database_file(&database_path).await?;

    assert_eq!(result, Some(PathBuf::from(database_path)));
    Ok(())
}

#[tokio::test]
async fn treats_a_missing_database_file_as_a_cache_miss() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = tempdir()?;
    let missing_path = directory.path().join("removed.gguf");
    let database_path = missing_path.to_string_lossy().into_owned();

    assert_eq!(existing_database_file(&database_path).await?, None);
    Ok(())
}

#[tokio::test]
async fn does_not_return_a_directory_as_a_download_file() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = tempdir()?;
    let database_path = directory.path().to_string_lossy().into_owned();

    assert_eq!(existing_database_file(&database_path).await?, None);
    Ok(())
}
