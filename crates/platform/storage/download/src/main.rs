use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

use burncloud_database_sys::{DownloadDB, SysDownload};
use burncloud_download::{DownloadError, DownloadManager};
use burncloud_download_aria2::kill_existing_aria2;
use tokio::task::JoinHandle;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const MODELS_DIR: &str = "models";
const LOG_PATH: &str = "download-test-run.log";
const T6_URL: &str =
    "https://modelscope.cn/models/google-bert/bert-base-chinese/resolve/master/not-found-file.bin";
const T7_HOMEPAGE_URL: &str = "https://hf-mirror.com/openai-community/gpt2-medium";

#[derive(Clone, Copy)]
struct ModelCase {
    name: &'static str,
    url: &'static str,
    repository: &'static str,
}

const M1: ModelCase = ModelCase {
    name: "M1",
    url: "https://modelscope.cn/models/google-bert/bert-base-chinese/resolve/master/vocab.txt",
    repository: "bert-base-chinese",
};
const M2: ModelCase = ModelCase {
    name: "M2",
    url: "https://modelscope.cn/models/AI-ModelScope/Mistral-7B-v0.2-hf/resolve/master/tokenizer.json",
    repository: "Mistral-7B-v0.2-hf",
};
const M3: ModelCase = ModelCase {
    name: "M3",
    url: "https://modelscope.cn/models/BAAI/bge-small-zh-v1.5/resolve/master/tokenizer.json",
    repository: "bge-small-zh-v1.5",
};
const M4: ModelCase = ModelCase {
    name: "M4",
    url: "https://modelscope.cn/models/AI-ModelScope/roberta-base/resolve/master/vocab.json",
    repository: "roberta-base",
};
const M5: ModelCase = ModelCase {
    name: "M5",
    url: "https://hf-mirror.com/google-bert/bert-large-uncased/resolve/main/pytorch_model.bin",
    repository: "bert-large-uncased",
};
const M6: ModelCase = ModelCase {
    name: "M6",
    url: "https://hf-mirror.com/TinyLlama/TinyLlama-1.1B-Chat-v1.0/resolve/main/model.safetensors",
    repository: "TinyLlama-1.1B-Chat-v1.0",
};
const M7: ModelCase = ModelCase {
    name: "M7",
    url: "https://hf-mirror.com/EleutherAI/pythia-1b/resolve/main/pytorch_model.bin",
    repository: "pythia-1b",
};
const M8: ModelCase = ModelCase {
    name: "M8",
    url: "https://hf-mirror.com/openai-community/gpt2-medium/resolve/main/pytorch_model.bin",
    repository: "gpt2-medium",
};

#[tokio::main]
async fn main() -> ExitCode {
    let executable = match std::env::current_exe() {
        Ok(path) => path,
        Err(error) => {
            println!("读取可执行文件路径 FAIL: {error}");
            return ExitCode::from(1);
        }
    };
    let executable_dir = match executable.parent() {
        Some(path) => path.to_path_buf(),
        None => {
            println!("读取可执行文件父目录 FAIL");
            return ExitCode::from(1);
        }
    };
    let mut log = vec![
        "burncloud-download 真实运行测试".to_string(),
        "工作目录：E:\\burnCloud".to_string(),
        "流程：76-download模块测试流程.md，T1-T11 顺序执行".to_string(),
        format!("可执行文件：{}", executable.display()),
        format!("可执行文件目录：{}", executable_dir.display()),
    ];
    let mut failed = false;

    match prepare_clean_state().await {
        Ok(()) => {
            log.push("前置清理：已清理已知测试记录、models 文件和残留 aria2 进程".to_string())
        }
        Err(error) => {
            let line = format!("前置清理 FAIL: {error}");
            println!("{line}");
            log.push(line);
            failed = true;
        }
    }

    if !failed {
        failed |= !run_case("T1 基本下载", &mut log, test_t1()).await;
        failed |= !run_case("T2 同键幂等", &mut log, test_t2()).await;
        failed |= !run_case("T3 已完成复用", &mut log, test_t3()).await;
        failed |= !run_case("T4 文件删除后重下", &mut log, test_t4()).await;
        failed |= !run_case("T5a 关闭后重启·接管", &mut log, test_t5a()).await;
        failed |= !run_case("T5b 关闭后重启·重建", &mut log, test_t5b()).await;
        failed |= !run_case("T6 失败路径", &mut log, test_t6()).await;
        failed |= !run_case("T7 输入拒绝", &mut log, test_t7()).await;
        failed |= !run_case("T8 并发多任务", &mut log, test_t8()).await;
        failed |= !run_case("T9 大文件下载", &mut log, test_t9()).await;
        failed |= !run_case("T10 models 位置正确", &mut log, test_t10()).await;
        failed |= !run_case("T11 models 未嵌套创建", &mut log, test_t11()).await;
    }

    if let Err(error) = clear_models().await {
        let line = format!("收尾清理 models FAIL: {error}");
        println!("{line}");
        log.push(line);
        failed = true;
    }
    kill_existing_aria2();
    log.push("收尾清理：已请求终止 aria2 进程".to_string());

    let summary = if failed {
        "真实运行测试结论：FAIL"
    } else {
        "真实运行测试结论：T1-T11 全部 PASS"
    };
    println!("{summary}");
    log.push(summary.to_string());

    if let Err(error) = tokio::fs::write(LOG_PATH, log.join("\n") + "\n").await {
        println!("写入运行日志 FAIL: {error}");
        failed = true;
    } else {
        println!("运行日志：{LOG_PATH}");
    }

    if failed {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    }
}

async fn run_case<F>(name: &str, log: &mut Vec<String>, test: F) -> bool
where
    F: std::future::Future<Output = TestResult<()>>,
{
    match test.await {
        Ok(()) => {
            let line = format!("{name}: PASS");
            println!("{line}");
            log.push(line);
            true
        }
        Err(error) => {
            let line = format!("{name}: FAIL: {error}");
            println!("{line}");
            log.push(line);
            false
        }
    }
}

async fn prepare_clean_state() -> TestResult {
    clear_models().await?;
    kill_existing_aria2();

    let database = DownloadDB::new().await?;
    let known_urls = [
        M1.url, M2.url, M3.url, M4.url, M5.url, M6.url, M7.url, M8.url, T6_URL,
    ];
    for record in database.list(None).await? {
        let is_known = record
            .url
            .as_deref()
            .is_some_and(|url| known_urls.contains(&url))
            || known_urls.iter().any(|url| record.uris.contains(url));
        if is_known {
            database.delete(&record.gid).await?;
        }
    }
    Ok(())
}

async fn clear_models() -> TestResult {
    let models = models_dir().await?;
    let mut entries = tokio::fs::read_dir(&models).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        let metadata = tokio::fs::symlink_metadata(&path).await?;
        if metadata.is_dir() {
            tokio::fs::remove_dir_all(path).await?;
        } else {
            tokio::fs::remove_file(path).await?;
        }
    }
    Ok(())
}

async fn models_dir() -> TestResult<PathBuf> {
    let executable = std::env::current_exe()?;
    let executable_dir = executable
        .parent()
        .ok_or_else(|| error("可执行文件没有父目录"))?;
    let models = executable_dir.join(MODELS_DIR);
    tokio::fs::create_dir_all(&models).await?;
    Ok(normalize_path(&tokio::fs::canonicalize(models).await?))
}

fn normalize_path(path: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let path = path.to_string_lossy();
        let path = path.as_ref();
        let path = path
            .strip_prefix("//?/")
            .or_else(|| path.strip_prefix("\\\\?\\"))
            .unwrap_or(path);
        PathBuf::from(path)
    }
    #[cfg(not(windows))]
    {
        path.to_path_buf()
    }
}

async fn test_t1() -> TestResult {
    clear_models().await?;
    let manager = DownloadManager::new().await?;
    let path = manager.download(M1.url, MODELS_DIR).await?;
    assert_completed(M1, &path).await?;
    println!(
        "T1 关键值：path={} repository={}",
        path.display(),
        M1.repository
    );
    Ok(())
}

async fn test_t2() -> TestResult {
    clear_models().await?;
    let manager = Arc::new(DownloadManager::new().await?);
    let first = spawn_download(Arc::clone(&manager), M1);
    let second = spawn_download(Arc::clone(&manager), M1);
    let database = DownloadDB::new().await?;
    let record = wait_for_incomplete(&database, M1).await?;
    let matching = database
        .list(None)
        .await?
        .into_iter()
        .filter(|value| {
            value.url.as_deref() == Some(M1.url)
                && matches!(
                    value.status.as_str(),
                    "pending" | "active" | "waiting" | "paused"
                )
        })
        .count();
    require(
        matching == 1,
        format!("同键进行中记录数应为 1，实际为 {matching}"),
    )?;

    let first_path = join_download(first).await?;
    let second_path = join_download(second).await?;
    require(
        first_path == second_path,
        format!(
            "并发调用返回路径不一致：{} / {}",
            first_path.display(),
            second_path.display()
        ),
    )?;
    assert_completed(M1, &first_path).await?;
    println!(
        "T2 关键值：gid={} in_progress_records={matching}",
        record.gid
    );
    drop(manager);
    Ok(())
}

async fn test_t3() -> TestResult {
    clear_models().await?;
    let manager = DownloadManager::new().await?;
    let first_path = manager.download(M1.url, MODELS_DIR).await?;
    let database = DownloadDB::new().await?;
    let before = matching_records(&database, M1).await?.len();
    let second_path = manager.download(M1.url, MODELS_DIR).await?;
    let after = matching_records(&database, M1).await?.len();
    require(first_path == second_path, "已完成复用返回路径不一致")?;
    require(
        before == after,
        format!("复用不应新增记录：{before} -> {after}"),
    )?;
    assert_completed(M1, &second_path).await?;
    println!(
        "T3 关键值：reused_path={} record_count={after}",
        second_path.display()
    );
    Ok(())
}

async fn test_t4() -> TestResult {
    clear_models().await?;
    let manager = DownloadManager::new().await?;
    let old_path = manager.download(M1.url, MODELS_DIR).await?;
    let database = DownloadDB::new().await?;
    let old_gid = newest_record(&database, M1).await?.gid;
    tokio::fs::remove_file(&old_path).await?;
    require(
        !tokio::fs::try_exists(&old_path).await?,
        "T4 删除文件未生效",
    )?;

    let new_path = manager.download(M1.url, MODELS_DIR).await?;
    let new_gid = newest_record(&database, M1).await?.gid;
    require(
        tokio::fs::try_exists(&new_path).await?,
        "T4 重下后文件不存在",
    )?;
    require(
        new_gid != old_gid,
        format!("T4 未创建新的 aria2 任务：旧 GID={old_gid}，新 GID={new_gid}"),
    )?;
    assert_completed(M1, &new_path).await?;
    println!(
        "T4 关键值：deleted_path={} old_gid={} new_gid={}",
        old_path.display(),
        old_gid,
        new_gid
    );
    Ok(())
}

async fn test_t5a() -> TestResult {
    clear_models().await?;
    let manager = Arc::new(DownloadManager::new().await?);
    let task = spawn_download(Arc::clone(&manager), M5);
    let database = DownloadDB::new().await?;
    let old_record = wait_for_incomplete(&database, M5).await?;
    task.abort();
    join_aborted(task).await?;
    drop(manager);
    tokio::time::sleep(Duration::from_millis(500)).await;

    let restored_manager = DownloadManager::new().await?;
    let restored_record = newest_record(&database, M5).await?;
    require(
        restored_record.gid == old_record.gid,
        format!(
            "T5a 应接管旧 GID：旧={}，恢复后={}",
            old_record.gid, restored_record.gid
        ),
    )?;
    let path = restored_manager.download(M5.url, MODELS_DIR).await?;
    assert_completed(M5, &path).await?;
    println!(
        "T5a 关键值：old_gid={} restored_gid={}",
        old_record.gid, restored_record.gid
    );
    Ok(())
}

async fn test_t5b() -> TestResult {
    clear_models().await?;
    let manager = Arc::new(DownloadManager::new().await?);
    let task = spawn_download(Arc::clone(&manager), M5);
    let database = DownloadDB::new().await?;
    let old_record = wait_for_incomplete(&database, M5).await?;
    task.abort();
    join_aborted(task).await?;
    drop(manager);
    kill_existing_aria2();
    tokio::time::sleep(Duration::from_millis(500)).await;

    let restored_manager = DownloadManager::new().await?;
    let restored_record = newest_record(&database, M5).await?;
    require(
        restored_record.gid != old_record.gid,
        format!(
            "T5b 应重建新 GID：旧={}，恢复后={}",
            old_record.gid, restored_record.gid
        ),
    )?;
    let path = restored_manager.download(M5.url, MODELS_DIR).await?;
    assert_completed(M5, &path).await?;
    println!(
        "T5b 关键值：old_gid={} rebuilt_gid={}",
        old_record.gid, restored_record.gid
    );
    Ok(())
}

async fn test_t6() -> TestResult {
    clear_models().await?;
    let manager = DownloadManager::new().await?;
    let result = manager.download(T6_URL, MODELS_DIR).await;
    require(result.is_err(), "404 下载不应返回路径")?;
    let database = DownloadDB::new().await?;
    let record = database
        .list(None)
        .await?
        .into_iter()
        .filter(|value| value.url.as_deref() == Some(T6_URL))
        .max_by_key(|value| value.id);
    let record = record.ok_or_else(|| error("T6 未找到失败记录"))?;
    require(
        record.status == "failed",
        format!("T6 状态应为 failed，实际为 {}", record.status),
    )?;
    require(
        record.final_path.is_none(),
        "T6 失败记录不应返回 final_path",
    )?;
    println!(
        "T6 关键值：error={}",
        record.error.as_deref().unwrap_or("<none>")
    );
    Ok(())
}

async fn test_t7() -> TestResult {
    clear_models().await?;
    let manager = DownloadManager::new().await?;
    let invalid_inputs = [
        (
            "ftp scheme",
            "ftp://8.8.8.8/org/repo/resolve/main/file",
            MODELS_DIR,
            "InvalidUrl",
        ),
        (
            "loopback",
            "http://127.0.0.1/org/repo/resolve/main/file",
            MODELS_DIR,
            "InvalidUrl",
        ),
        (
            "private",
            "https://10.0.0.1/org/repo/resolve/main/file",
            MODELS_DIR,
            "InvalidUrl",
        ),
        ("directory escape", M1.url, "..", "InvalidDirectory"),
        (
            "repository homepage",
            T7_HOMEPAGE_URL,
            MODELS_DIR,
            "InvalidFilename",
        ),
    ];
    let database = DownloadDB::new().await?;
    for (name, url, directory, expected) in invalid_inputs {
        let before_records = database.list(None).await?;
        let before_ids = before_records
            .iter()
            .map(|value| value.id)
            .collect::<Vec<_>>();
        let result = manager.download(url, directory).await;
        let after_records = database.list(None).await?;
        let created = after_records
            .iter()
            .any(|value| !before_ids.contains(&value.id) && value.url.as_deref() == Some(url));
        require(
            result_matches(&result, expected),
            format!("{name} 返回错误类型不符"),
        )?;
        require(!created, format!("{name} 不应产生数据库记录"))?;
        require(
            before_records.len() == after_records.len(),
            format!(
                "{name} 改变了记录数：{} -> {}",
                before_records.len(),
                after_records.len()
            ),
        )?;
    }
    println!("T7 关键值：5 类输入均拒绝且无新增记录");
    Ok(())
}

async fn test_t8() -> TestResult {
    clear_models().await?;
    let manager = Arc::new(DownloadManager::new().await?);
    let mut tasks = Vec::new();
    for model in [M1, M2, M3, M4, M5, M6, M7, M8] {
        tasks.push(spawn_download(Arc::clone(&manager), model));
    }

    let mut paths = Vec::new();
    for (model, task) in [M1, M2, M3, M4, M5, M6, M7, M8].into_iter().zip(tasks) {
        let path = join_download(task).await?;
        assert_completed(model, &path).await?;
        paths.push(path);
    }
    let unique_count = paths.iter().collect::<std::collections::HashSet<_>>().len();
    require(
        unique_count == 8,
        format!("T8 路径应互不相同，实际为 {unique_count}"),
    )?;
    println!("T8 关键值：completed=8 unique_paths={unique_count}");
    Ok(())
}

async fn test_t9() -> TestResult {
    clear_models().await?;
    let manager = DownloadManager::new().await?;
    for model in [M5, M6, M7, M8] {
        let path = manager.download(model.url, MODELS_DIR).await?;
        assert_completed(model, &path).await?;
        println!("T9 关键值：{} path={}", model.name, path.display());
    }
    Ok(())
}

async fn test_t10() -> TestResult {
    let models = models_dir().await?;
    let executable_dir = models
        .parent()
        .ok_or_else(|| error("models 目录没有可执行文件父目录"))?;
    let project_root = tokio::fs::canonicalize(std::env::current_dir()?).await?;
    let project_models = project_root.join(MODELS_DIR);

    require(
        models == executable_dir.join(MODELS_DIR),
        format!("models 目录位置错误：{}", models.display()),
    )?;
    require(
        !tokio::fs::try_exists(&project_models).await?,
        format!("项目根不应存在 models：{}", project_models.display()),
    )?;
    let mut entries = tokio::fs::read_dir(&models).await?;
    while let Some(entry) = entries.next_entry().await? {
        let metadata = tokio::fs::symlink_metadata(entry.path()).await?;
        require(
            !metadata.is_dir(),
            format!("下载文件不应直接落在嵌套目录：{}", entry.path().display()),
        )?;
    }
    println!(
        "T10 关键值：exe_dir={} models_dir={}",
        executable_dir.display(),
        models.display()
    );
    Ok(())
}

async fn test_t11() -> TestResult {
    clear_models().await?;
    let manager = DownloadManager::new().await?;
    let nested_path = manager.download(M1.url, "models/sub").await?;
    let direct_path = manager.download(M2.url, MODELS_DIR).await?;
    let models = models_dir().await?;
    let executable_dir = models
        .parent()
        .ok_or_else(|| error("models 目录没有可执行文件父目录"))?;
    let models_dirs = find_models_directories(executable_dir).await?;
    let nested_models = models.join(MODELS_DIR);

    require(
        nested_path.starts_with(models.join("sub")),
        format!(
            "models/sub 未规范化到 models/sub：{}",
            nested_path.display()
        ),
    )?;
    require(
        direct_path.starts_with(&models) && !direct_path.starts_with(&nested_models),
        format!("models 未规范化到 models 根目录：{}", direct_path.display()),
    )?;
    require(
        models_dirs.len() == 1 && models_dirs[0] == models,
        format!("models 目录数量或位置错误：{models_dirs:?}"),
    )?;
    require(
        !tokio::fs::try_exists(nested_models).await?,
        "不应创建 models/models 嵌套目录",
    )?;
    println!(
        "T11 关键值：models={} nested={} models_dir_count={}",
        models.display(),
        nested_path.display(),
        models_dirs.len()
    );
    Ok(())
}

async fn find_models_directories(root: &Path) -> TestResult<Vec<PathBuf>> {
    let mut pending = vec![root.to_path_buf()];
    let mut models = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = tokio::fs::read_dir(&directory).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let metadata = tokio::fs::symlink_metadata(&path).await?;
            if metadata.is_dir() {
                if path.file_name().and_then(|value| value.to_str()) == Some(MODELS_DIR) {
                    models.push(normalize_path(&tokio::fs::canonicalize(&path).await?));
                }
                pending.push(path);
            }
        }
    }
    Ok(models)
}

fn spawn_download(
    manager: Arc<DownloadManager>,
    model: ModelCase,
) -> JoinHandle<Result<PathBuf, DownloadError>> {
    tokio::spawn(async move { manager.download(model.url, MODELS_DIR).await })
}

async fn join_download(task: JoinHandle<Result<PathBuf, DownloadError>>) -> TestResult<PathBuf> {
    match task.await {
        Ok(Ok(path)) => Ok(path),
        Ok(Err(error)) => Err(Box::new(error)),
        Err(error) => Err(Box::new(error)),
    }
}

async fn join_aborted(task: JoinHandle<Result<PathBuf, DownloadError>>) -> TestResult {
    match task.await {
        Err(error) if error.is_cancelled() => Ok(()),
        Ok(_) => Err(error("被取消的下载任务意外返回")),
        Err(error) => Err(Box::new(error)),
    }
}

async fn wait_for_incomplete(database: &DownloadDB, model: ModelCase) -> TestResult<SysDownload> {
    for _ in 0..100 {
        if let Some(record) = database
            .list(None)
            .await?
            .into_iter()
            .filter(|value| value.url.as_deref() == Some(model.url))
            .find(|value| {
                matches!(
                    value.status.as_str(),
                    "pending" | "active" | "waiting" | "paused"
                )
            })
        {
            return Ok(record);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err(error(format!("{} 未在 10 秒内进入进行中状态", model.name)))
}

async fn matching_records(database: &DownloadDB, model: ModelCase) -> TestResult<Vec<SysDownload>> {
    Ok(database
        .list(None)
        .await?
        .into_iter()
        .filter(|value| value.url.as_deref() == Some(model.url))
        .collect())
}

async fn newest_record(database: &DownloadDB, model: ModelCase) -> TestResult<SysDownload> {
    matching_records(database, model)
        .await?
        .into_iter()
        .max_by_key(|value| value.id)
        .ok_or_else(|| error(format!("{} 未找到数据库记录", model.name)))
}

async fn assert_completed(model: ModelCase, path: &Path) -> TestResult {
    let models = models_dir().await?;
    require(
        path.is_absolute(),
        format!("{} 返回路径不是绝对路径", model.name),
    )?;
    require(
        path.starts_with(&models),
        format!("{} 返回路径超出 models：{}", model.name, path.display()),
    )?;
    require(
        path.file_name().and_then(|value| value.to_str()) == Some(model.repository),
        format!(
            "{} 落盘名错误：期望 {}，实际 {}",
            model.name,
            model.repository,
            path.display()
        ),
    )?;
    let metadata = tokio::fs::metadata(path).await?;
    require(
        metadata.is_file(),
        format!("{} 返回路径不是文件", model.name),
    )?;

    let database = DownloadDB::new().await?;
    let record = newest_record(&database, model).await?;
    require(
        record.status == "completed",
        format!("{} 数据库状态不是 completed：{}", model.name, record.status),
    )?;
    let final_path = record
        .final_path
        .as_deref()
        .ok_or_else(|| error(format!("{} 缺少 final_path", model.name)))?;
    require(
        Path::new(final_path) == path,
        format!("{} 返回路径与数据库 final_path 不一致", model.name),
    )?;
    let size = i64::try_from(metadata.len())?;
    require(
        record.total_length == Some(size) && record.completed_length == Some(size),
        format!(
            "{} 文件大小与数据库响应大小不一致：file={} total={:?} completed={:?}",
            model.name, size, record.total_length, record.completed_length
        ),
    )?;
    Ok(())
}

fn result_matches(result: &Result<PathBuf, DownloadError>, expected: &str) -> bool {
    matches!(
        (result, expected),
        (Err(DownloadError::InvalidUrl(_)), "InvalidUrl")
            | (Err(DownloadError::InvalidDirectory(_)), "InvalidDirectory")
            | (Err(DownloadError::InvalidFilename), "InvalidFilename")
    )
}

fn require(condition: bool, message: impl Into<String>) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(error(message))
    }
}

fn error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    Box::new(std::io::Error::other(message.into()))
}
