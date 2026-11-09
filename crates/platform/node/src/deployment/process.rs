// 导入标准库的 HashMap：用于在管理器内部按 PID 索引子进程句柄
use std::collections::HashMap;

// 导入 async_trait 宏：Rust 稳定版 trait 中不能直接写 async fn，
// 该宏会把 async fn 改写成返回 Pin<Box<dyn Future>> 的形式
use async_trait::async_trait;

// 从 tokio 导入：
// - process::Command：异步子进程启动器（类似 std::process::Command，但是异步的）
// - sync::Mutex：异步互斥锁（因为持锁期间可能 .await，不能用 std 的同步 Mutex）
use tokio::{process::Command, sync::Mutex};

// 从本 crate 导入进程管理相关的契约类型：
// - ProcessError：进程管理错误（StartFailed / StopFailed / IsRunningFailed 等变体）
// - ProcessHandle：进程句柄（内部是一个 pid 字段）
// - ProcessManager：进程管理 trait（本文件要实现它）
// - ProcessSpec：进程启动规格（program + args）
use crate::{ProcessError, ProcessHandle, ProcessManager, ProcessSpec};

/// 管理由 Node 本机启动的操作系统子进程。
///
/// 默认构造在丢弃管理器时只释放句柄；[`LocalProcessManager::with_kill_on_drop`]
/// 可选择在管理器销毁时终止仍在运行的子进程。
///
/// # Examples
///
/// ```
/// use burncloud_node_runtime::deployment::LocalProcessManager;
///
/// let _manager = LocalProcessManager::default();
/// ```
//
// 文档要点：
//   1. 该管理器只负责“本机”启动的 OS 子进程；
//   2. 关键语义：Drop 管理器时只释放句柄，不会 kill 子进程
//      —— 子进程会继续在后台运行。这是一个刻意设计（避免意外杀进程），
//      但也意味着调用方要负责显式 stop。
//   3. doctest 验证 Default 可构造，防止文档与代码漂移。
//
// 派生 Default：允许 LocalProcessManager::default() 构造（HashMap 默认空、Mutex 默认可建）。
#[derive(Default)]
pub struct LocalProcessManager {
    // 内部状态：PID -> tokio 子进程句柄 的映射表，用异步 Mutex 保护。
    // 用 HashMap 是因为需要按 PID 随机查找/删除；
    // 用 tokio::sync::Mutex 是因为持锁期间会 .await（如 kill().await），
    // 不能用 std::sync::Mutex（那会阻塞整个异步运行时线程）。
    children: Mutex<HashMap<u32, tokio::process::Child>>,
    kill_on_drop: bool,
}

impl LocalProcessManager {
    /// 创建一个在管理器销毁时终止子进程的管理器。
    ///
    /// 用于服务拥有完整进程生命周期的场景；默认构造仍保留原有的显式停止语义。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_node_runtime::deployment::LocalProcessManager;
    ///
    /// let _manager = LocalProcessManager::with_kill_on_drop();
    /// ```
    pub fn with_kill_on_drop() -> Self {
        Self {
            kill_on_drop: true,
            ..Self::default()
        }
    }
}

// 用 async_trait 宏为 LocalProcessManager 实现 ProcessManager trait
#[async_trait]
impl ProcessManager for LocalProcessManager {
    // 启动一个子进程，返回其句柄
    async fn start(&self, spec: ProcessSpec) -> Result<ProcessHandle, ProcessError> {
        // 前置校验 1：程序名不能为空，否则无法启动
        if spec.program.is_empty() {
            // 返回 StartFailed 错误（带原因字符串）
            return Err(ProcessError::StartFailed(
                "process program must be non-empty".to_string(),
            ));
        }

        // 用 tokio 的 Command 构造并启动子进程：
        //   - Command::new(&spec.program) 指定可执行程序
        //   - .args(&spec.args) 传入参数
        //   - .spawn() 异步启动
        // 启动失败（如程序不存在、无权限）映射为 ProcessError::StartFailed
        let mut command = Command::new(&spec.program);
        command.args(&spec.args).kill_on_drop(self.kill_on_drop);
        let mut child = command
            .spawn()
            .map_err(|error| ProcessError::StartFailed(error.to_string()))?;

        // 获取子进程 PID，并处理三种异常情况
        let pid = match child.id() {
            // 情况 A：拿到非零 PID -> 正常
            Some(pid) if pid != 0 => pid,
            // 情况 B：拿到 0 号 PID -> 视为 OS 异常，拒绝接受
            Some(_) => {
                // 返回 StartFailed 错误（零 PID 是无效的）
                return Err(ProcessError::StartFailed(
                    "operating system returned a zero process ID".to_string(),
                ));
            }
            // 情况 C：拿不到 PID（进程已立刻退出，如 exec 失败）
            None => {
                // 先 wait 回收子进程，避免僵尸进程泄漏
                child
                    .wait()
                    .await
                    .map_err(|error| ProcessError::StartFailed(error.to_string()))?;
                // 再返回 StartFailed 错误（OS 未给出 PID）
                return Err(ProcessError::StartFailed(
                    "operating system did not return a process ID".to_string(),
                ));
            }
        };

        // 把 PID 与子进程句柄登记到管理表（异步加锁）
        self.children.lock().await.insert(pid, child);
        // 返回对外句柄（只暴露 PID）
        Ok(ProcessHandle { pid })
    }

    // 停止指定句柄对应的子进程
    async fn stop(&self, handle: ProcessHandle) -> Result<(), ProcessError> {
        // 前置校验：PID 必须非零（零 PID 是无效句柄）
        if handle.pid == 0 {
            // 返回 StopFailed 错误
            return Err(ProcessError::StopFailed(
                "process PID must be non-zero".to_string(),
            ));
        }

        // 获取管理表的异步锁（整个 stop 过程持锁）
        let mut children = self.children.lock().await;

        // 先探测子进程当前状态（不阻塞、不 kill）
        let status = match children.get_mut(&handle.pid) {
            // 找到句柄：调用 try_wait 非阻塞查询是否已退出
            Some(child) => child
                .try_wait()
                .map_err(|error| ProcessError::StopFailed(error.to_string()))?,
            // 表里没有该 PID：视为已停止（幂等），直接返回 Ok
            None => return Ok(()),
        };

        // 若子进程已退出（status 为 Some）：
        if status.is_some() {
            // 从表中移除句柄（清理）
            children.remove(&handle.pid);
            // 返回成功（无需 kill）
            return Ok(());
        }

        // 注释说明：持有管理表锁直到子进程终止，
        // 避免并发 is_running 查询提前报告其已退出。
        // （即：kill 完成前，别人查 is_running 会先阻塞在这把锁上，
        //   从而不会看到“已 kill 但表里还留着”的中间态。）

        // 再次 get_mut（此时已知存在）并执行 kill
        match children.get_mut(&handle.pid) {
            // 调用异步 kill 终止子进程
            Some(child) => child
                .kill()
                .await
                .map_err(|error| ProcessError::StopFailed(error.to_string()))?,
            // 理论上不会走到这里（前面已确认存在），保险起见直接返回 Ok
            None => return Ok(()),
        }
        // kill 成功后从表中移除句柄
        children.remove(&handle.pid);
        // 返回成功
        Ok(())
    }

    // 查询指定句柄对应的子进程是否仍在运行
    async fn is_running(&self, handle: ProcessHandle) -> Result<bool, ProcessError> {
        // 前置校验：PID 必须非零
        if handle.pid == 0 {
            // 返回 IsRunningFailed 错误
            return Err(ProcessError::IsRunningFailed(
                "process PID must be non-zero".to_string(),
            ));
        }

        // 获取管理表的异步锁
        let mut children = self.children.lock().await;

        // 非阻塞查询子进程状态
        let status = match children.get_mut(&handle.pid) {
            // 找到句柄：try_wait 查询是否已退出
            Some(child) => child
                .try_wait()
                .map_err(|error| ProcessError::IsRunningFailed(error.to_string()))?,
            // 表里没有该 PID：说明未知/已清理，视为不在运行
            None => return Ok(false),
        };

        // 根据 try_wait 结果判断
        if status.is_some() {
            // 已退出：从表中移除句柄（清理），返回 false
            children.remove(&handle.pid);
            Ok(false)
        } else {
            // 未退出：返回 true
            Ok(true)
        }
    }
}
