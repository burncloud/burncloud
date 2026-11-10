use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

pub(super) struct IsolatedDatabase {
    _environment_lock: MutexGuard<'static, ()>,
    previous_url: Option<OsString>,
    path: PathBuf,
}

static DATABASE_ENVIRONMENT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

pub(super) fn isolated_database(
    name: &str,
) -> Result<IsolatedDatabase, Box<dyn std::error::Error>> {
    // 测试通过进程环境变量选择数据库，必须串行修改和恢复，避免并发测试互相覆盖连接地址。
    let environment_lock = DATABASE_ENVIRONMENT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| std::io::Error::other("测试数据库环境变量锁已中毒"))?;
    let executable = std::env::current_exe()?;
    let executable_parent = executable.parent().ok_or("测试可执行文件应有父目录")?;
    let directory = executable_parent.join("burncloud-node-runtime-test-databases");
    std::fs::create_dir_all(&directory)?;

    let path = directory.join(format!("{name}-{}.sqlite", std::process::id()));
    match std::fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }

    let previous_url = std::env::var_os("BURNCLOUD_DATABASE_URL");
    let normalized = path.to_string_lossy().replace('\\', "/");
    std::env::set_var(
        "BURNCLOUD_DATABASE_URL",
        format!("sqlite:///{normalized}?mode=rwc"),
    );

    Ok(IsolatedDatabase {
        _environment_lock: environment_lock,
        previous_url,
        path,
    })
}

impl IsolatedDatabase {
    pub(super) async fn cleanup(&self) -> Result<(), std::io::Error> {
        let paths = [
            self.path.clone(),
            PathBuf::from(format!("{}-wal", self.path.display())),
            PathBuf::from(format!("{}-shm", self.path.display())),
        ];

        for path in paths {
            let mut last_error = None;
            for _ in 0..50 {
                match tokio::fs::remove_file(&path).await {
                    Ok(()) => {
                        last_error = None;
                        break;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        last_error = None;
                        break;
                    }
                    Err(error) => {
                        last_error = Some(error);
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
            if let Some(error) = last_error {
                return Err(error);
            }
        }

        Ok(())
    }
}

impl Drop for IsolatedDatabase {
    fn drop(&mut self) {
        match self.previous_url.take() {
            Some(value) => std::env::set_var("BURNCLOUD_DATABASE_URL", value),
            None => std::env::remove_var("BURNCLOUD_DATABASE_URL"),
        }

        // Drop 无法返回清理错误，因此只报告异常，避免把测试资源问题静默吞掉。
        for path in [
            self.path.clone(),
            PathBuf::from(format!("{}-wal", self.path.display())),
            PathBuf::from(format!("{}-shm", self.path.display())),
        ] {
            if let Err(error) = std::fs::remove_file(&path) {
                if error.kind() != std::io::ErrorKind::NotFound {
                    eprintln!("清理测试数据库失败：{}：{error}", path.display());
                }
            }
        }
    }
}
