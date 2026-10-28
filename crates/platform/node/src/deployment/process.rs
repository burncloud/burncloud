use std::collections::HashMap;

use async_trait::async_trait;
use tokio::{process::Command, sync::Mutex};

use crate::{ProcessError, ProcessHandle, ProcessManager, ProcessSpec};

/// 管理由 Node 本机启动的操作系统子进程。
///
/// 丢弃该管理器只释放持有的子进程句柄，不会隐式终止仍在运行的进程。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::LocalProcessManager;
///
/// let _manager = LocalProcessManager::default();
/// ```
#[derive(Default)]
pub struct LocalProcessManager {
    children: Mutex<HashMap<u32, tokio::process::Child>>,
}

#[async_trait]
impl ProcessManager for LocalProcessManager {
    async fn start(&self, spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        if spec.program.is_empty() {
            return Err(ProcessError::StartFailed(
                "process program must be non-empty".to_string(),
            ));
        }

        let mut child = Command::new(&spec.program)
            .args(&spec.args)
            .spawn()
            .map_err(|error| ProcessError::StartFailed(error.to_string()))?;

        let pid = match child.id() {
            Some(pid) if pid != 0 => pid,
            Some(_) => {
                return Err(ProcessError::StartFailed(
                    "operating system returned a zero process ID".to_string(),
                ));
            }
            None => {
                child
                    .wait()
                    .await
                    .map_err(|error| ProcessError::StartFailed(error.to_string()))?;
                return Err(ProcessError::StartFailed(
                    "operating system did not return a process ID".to_string(),
                ));
            }
        };

        self.children.lock().await.insert(pid, child);
        Ok(ProcessHandle { pid })
    }

    async fn stop(&self, handle: ProcessHandle) -> Result<(), ProcessError> {
        if handle.pid == 0 {
            return Err(ProcessError::StopFailed(
                "process PID must be non-zero".to_string(),
            ));
        }

        let mut children = self.children.lock().await;
        let status = match children.get_mut(&handle.pid) {
            Some(child) => child
                .try_wait()
                .map_err(|error| ProcessError::StopFailed(error.to_string()))?,
            None => return Ok(()),
        };

        if status.is_some() {
            children.remove(&handle.pid);
            return Ok(());
        }

        // 持有管理表锁直到子进程终止，避免并发存活查询提前报告其已退出。
        match children.get_mut(&handle.pid) {
            Some(child) => child
                .kill()
                .await
                .map_err(|error| ProcessError::StopFailed(error.to_string()))?,
            None => return Ok(()),
        }
        children.remove(&handle.pid);
        Ok(())
    }

    async fn is_running(&self, handle: ProcessHandle) -> Result<bool, ProcessError> {
        if handle.pid == 0 {
            return Err(ProcessError::IsRunningFailed(
                "process PID must be non-zero".to_string(),
            ));
        }

        let mut children = self.children.lock().await;
        let status = match children.get_mut(&handle.pid) {
            Some(child) => child
                .try_wait()
                .map_err(|error| ProcessError::IsRunningFailed(error.to_string()))?,
            None => return Ok(false),
        };

        if status.is_some() {
            children.remove(&handle.pid);
            Ok(false)
        } else {
            Ok(true)
        }
    }
}
