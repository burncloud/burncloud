use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::{Aria2Error, Aria2Result};
use crate::process::{find_running_aria2, start_aria2_rpc};
use crate::rpc::Aria2RpcClient;
use crate::types::{Aria2Config, Aria2Instance, ChildTerminator, ProcessTerminator};
use tokio::sync::Mutex as AsyncMutex;

// ============================================================================
// 简单守护进程
// ============================================================================

/// 启动器返回的 future：一个已经启动的服务实例。
pub(crate) type InstanceFuture = Pin<Box<dyn Future<Output = Aria2Result<Aria2Instance>> + Send>>;

/// 启动**一个** aria2 服务实例。
///
/// 做成可注入的缝，是为了让重启监控能被**桩进程**监督（#904）：本增量要断言的行为是监控的判定 ——
/// 「服务真的没了才补一个，且只补一个」 —— 而它必须在不启动真实 aria2、不联网、不调用进程清扫的
/// 前提下可观测。
pub(crate) type Launcher = Arc<dyn Fn(&Aria2Config) -> InstanceFuture + Send + Sync>;

pub struct Aria2Daemon {
    instance: Arc<Mutex<Option<Aria2Instance>>>,
    attached_port: Arc<Mutex<Option<u16>>>,
    config: Aria2Config,
    is_running: Arc<AtomicBool>,
    add_uri_lock: Arc<AsyncMutex<()>>,
    launcher: Launcher,
    terminator: Arc<dyn ProcessTerminator>,
}

impl Aria2Daemon {
    // 输入：aria2 运行配置。
    // 功能：创建未启动的 aria2 统一管理对象。
    // 错误：无，构造过程不执行外部操作。
    pub fn new(config: Aria2Config) -> Self {
        Self {
            instance: Arc::new(Mutex::new(None)),
            attached_port: Arc::new(Mutex::new(None)),
            config,
            is_running: Arc::new(AtomicBool::new(false)),
            add_uri_lock: Arc::new(AsyncMutex::new(())),
            launcher: Arc::new(|config: &Aria2Config| -> InstanceFuture {
                let config = config.clone();
                Box::pin(async move { start_aria2_rpc(&config).await })
            }),
            terminator: Arc::new(ChildTerminator),
        }
    }

    // 输入：可变的守护进程管理对象。
    // 功能：启动 aria2 服务，或接管同一端口上仍在运行的服务。
    // 错误：重复启动、服务启动或 RPC 探测失败时返回对应 Aria2Error。
    pub async fn start(&mut self) -> Aria2Result<()> {
        if self.is_running.load(Ordering::SeqCst) {
            return Err(Aria2Error::DaemonError("守护进程已在运行".to_string()));
        }

        if let Some(port) = find_running_aria2(&self.config.secret).await {
            *self
                .attached_port
                .lock()
                .unwrap_or_else(|error| error.into_inner()) = Some(port);
            self.is_running.store(true, Ordering::SeqCst);
            println!("已接管运行中的 aria2 RPC 服务，端口: {port}");
            return Ok(());
        }

        let instance = (self.launcher)(&self.config).await?;
        println!("aria2 RPC 服务已启动在端口: {}", instance.port);
        *self
            .instance
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(instance);
        self.is_running.store(true, Ordering::SeqCst);

        // 启动监控任务
        let instance = Arc::clone(&self.instance);
        let is_running = Arc::clone(&self.is_running);
        let config = self.config.clone();
        let launcher = Arc::clone(&self.launcher);

        // 异步监测进程状态并在退出后尝试重启
        tokio::spawn(async move {
            while is_running.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(1000)).await;
                if !is_running.load(Ordering::SeqCst) {
                    break;
                }

                // 在互斥锁保护下检查当前进程状态
                let need_restart = {
                    let mut lock = instance.lock().unwrap_or_else(|e| e.into_inner());
                    match lock.as_mut() {
                        Some(inst) => !inst.is_running(), // 检查服务是否还在运行
                        None => true,
                    }
                };

                // 服务确实退出时创建并替换新的运行实例
                if need_restart {
                    println!("检测到aria2已退出，重启中...");
                    if let Ok(new_instance) = (launcher)(&config).await {
                        let new_port = new_instance.port;
                        *instance.lock().unwrap_or_else(|e| e.into_inner()) = Some(new_instance);
                        println!("aria2重启成功，端口: {}", new_port);
                    }
                }
            }
        });

        Ok(())
    }

    // 输入：可变的守护进程管理对象。
    // 功能：停止当前管理的 aria2 子进程并清理本地连接状态。
    // 错误：进程终止错误被忽略，不向调用方返回错误。
    pub async fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);

        if let Some(ref mut instance) = self
            .instance
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .as_mut()
        {
            drop(self.terminator.terminate(instance));
        }

        *self
            .instance
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
        *self
            .attached_port
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = None;
        println!("aria2 守护进程已停止");
    }

    // 输入：守护进程管理对象。
    // 功能：根据当前实例或接管的端口创建 RPC 客户端。
    // 错误：守护进程尚未启动时返回 None。
    pub fn get_rpc_client(&self) -> Option<Aria2RpcClient> {
        let instance = self
            .instance
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let port = instance.as_ref().map(|value| value.port).or_else(|| {
            self.attached_port
                .lock()
                .unwrap_or_else(|error| error.into_inner())
                .to_owned()
        })?;
        Some(Aria2RpcClient::with_add_uri_lock(
            port,
            self.config.secret.clone(),
            Arc::clone(&self.add_uri_lock),
        ))
    }

    // 输入：守护进程管理对象。
    // 功能：读取守护进程是否可供 RPC 使用。
    // 错误：无。
    // 说明：这是**监管状态**（已启动且未停止），不是对服务的探测：服务本身是否还在，由
    // `Aria2Instance::is_running` 回答，监控循环用的也是后者。
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        reason = "test-only: an unwrap/expect failure is the intended failure signal"
    )]

    use super::*;
    use std::process::{Child, Command, Stdio};
    use std::sync::atomic::AtomicUsize;
    use std::time::{Duration, Instant};

    /// 一个长期存活的 aria2 服务替身：一个只会等待的真实进程。
    ///
    /// 它不是 aria2，不联网，也没有任何按镜像名的终止 —— #904 的计划允许、且仅允许这样的桩。
    fn spawn_stub_service() -> Child {
        #[cfg(unix)]
        let spawned = Command::new("sh")
            .args(["-c", "sleep 30"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        #[cfg(windows)]
        let spawned = Command::new("cmd")
            .args(["/C", "ping -n 31 127.0.0.1 >NUL"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        spawned.expect("桩服务进程能够启动")
    }

    /// 只计启动次数、用桩进程冒充服务的启动器。
    fn stub_launcher(launches: Arc<AtomicUsize>) -> Launcher {
        Arc::new(move |config: &Aria2Config| -> InstanceFuture {
            launches.fetch_add(1, Ordering::SeqCst);
            let config = config.clone();
            Box::pin(async move {
                Ok(Aria2Instance {
                    process: spawn_stub_service(),
                    port: config.port,
                    config,
                })
            })
        })
    }

    /// 记录每一次终止请求，然后执行真实终止。
    struct RecordingTerminator {
        targets: Arc<Mutex<Vec<u32>>>,
        inner: ChildTerminator,
    }

    impl ProcessTerminator for RecordingTerminator {
        fn terminate(&self, instance: &mut Aria2Instance) -> Aria2Result<()> {
            self.targets
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(instance.process.id());
            self.inner.terminate(instance)
        }
    }

    /// 用桩启动器与给定终止器组装的守护进程：与 `new` 同形，只是两处缝换成可观测实现。
    fn stub_daemon(
        launches: Arc<AtomicUsize>,
        terminator: Arc<dyn ProcessTerminator>,
    ) -> Aria2Daemon {
        Aria2Daemon {
            instance: Arc::new(Mutex::new(None)),
            attached_port: Arc::new(Mutex::new(None)),
            config: Aria2Config::default(),
            is_running: Arc::new(AtomicBool::new(false)),
            add_uri_lock: Arc::new(AsyncMutex::new(())),
            launcher: stub_launcher(launches),
            terminator,
        }
    }

    /// 当前被记录的实例的 pid。
    fn recorded_pid(daemon: &Aria2Daemon) -> u32 {
        let lock = daemon.instance.lock().unwrap_or_else(|e| e.into_inner());
        lock.as_ref().expect("一个被记录的实例").process.id()
    }

    /// 绕过监控直接终止被记录的服务，模拟一次崩溃。
    fn kill_recorded_service(daemon: &Aria2Daemon) {
        let mut lock = daemon.instance.lock().unwrap_or_else(|e| e.into_inner());
        let instance = lock.as_mut().expect("一个被记录的实例");
        instance.kill().expect("桩服务被终止");
    }

    /// 等到 `condition` 成立，或超时返回最后一次判定。
    async fn wait_until(mut condition: impl FnMut() -> bool, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if condition() {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        condition()
    }

    /// `pid` 是否还存在于进程表。
    #[cfg(unix)]
    fn process_exists(pid: u32) -> bool {
        // `kill -0` 是标准的「是否存在」检查，不投递任何信号。
        let check = format!("kill -0 {pid}");
        Command::new("sh")
            .args(["-c", check.as_str()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    /// `pid` 是否还存在于进程表（只查询，不终止）。
    #[cfg(windows)]
    fn process_exists(pid: u32) -> bool {
        let filter = format!("PID eq {pid}");
        let output = Command::new("tasklist")
            .args(["/FI", filter.as_str(), "/NH"])
            .output();
        match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).contains(&pid.to_string()),
            Err(_) => false,
        }
    }

    /// `pid` 是否确实消失，而不只是「终止调用返回了 Ok」。
    fn process_is_gone(pid: u32) -> bool {
        for _ in 0..100 {
            if !process_exists(pid) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// 去掉整行注释后的源码：守卫只看**代码**行，文档必须能写出被移除之物的名字。
    fn code_without_comments(source: &str) -> String {
        let mut code = String::new();
        for line in source.lines() {
            if !line.trim_start().starts_with("//") {
                code.push_str(line);
                code.push('\n');
            }
        }
        code
    }

    /// **服务活着时，监控绝不另起一个实例（#904 AC2a）。**
    #[tokio::test]
    async fn the_monitor_does_not_start_a_second_instance_while_the_service_is_alive() {
        let launches = Arc::new(AtomicUsize::new(0));
        let mut daemon = stub_daemon(Arc::clone(&launches), Arc::new(ChildTerminator));

        daemon.start().await.expect("桩服务启动成功");
        assert!(daemon.is_running(), "守护进程报告监管状态为运行");

        // 监控每 1000ms 一个 tick，等三个 tick 足以暴露一次重复启动。
        tokio::time::sleep(Duration::from_millis(3200)).await;

        assert_eq!(
            launches.load(Ordering::SeqCst),
            1,
            "服务还活着，监控不得补起第二个实例：修复前 is_running 读的是那个已经 detach 掉的子进程，\
             于是每个 tick 都会再起一个 daemon，端口一路递增"
        );
        assert!(
            daemon.get_rpc_client().is_some(),
            "仍然活着的实例必须可寻址"
        );

        daemon.stop().await;
    }

    /// **被记录的服务真的退出后，恰好补一个（#904 AC2a 的对照）。**
    ///
    /// 这条对照让上一条有意义：它证明监控确实在跑，并且当服务真的消失时**会**补起替代实例。
    #[tokio::test]
    async fn the_monitor_starts_one_replacement_after_the_recorded_service_really_exits() {
        let launches = Arc::new(AtomicUsize::new(0));
        let mut daemon = stub_daemon(Arc::clone(&launches), Arc::new(ChildTerminator));
        daemon.start().await.expect("桩服务启动成功");

        kill_recorded_service(&daemon);

        let replaced = wait_until(
            || launches.load(Ordering::SeqCst) >= 2,
            Duration::from_secs(6),
        )
        .await;

        assert!(
            replaced,
            "真正退出的服务必须被补起，否则监控等于没在跑；当前启动次数 {}",
            launches.load(Ordering::SeqCst)
        );
        assert_eq!(
            launches.load(Ordering::SeqCst),
            2,
            "替代实例只应有一个：多出来的启动正是 #904 描述的实例累积"
        );

        daemon.stop().await;
    }

    /// 停止期间监控从睡眠中唤醒后不得再启动替代服务。
    #[tokio::test]
    async fn stopping_during_monitor_sleep_does_not_restart_the_service() {
        let launches = Arc::new(AtomicUsize::new(0));
        let mut daemon = stub_daemon(Arc::clone(&launches), Arc::new(ChildTerminator));

        daemon.start().await.expect("桩服务启动成功");
        tokio::time::sleep(Duration::from_millis(100)).await;
        daemon.stop().await;
        tokio::time::sleep(Duration::from_millis(1100)).await;

        assert_eq!(
            launches.load(Ordering::SeqCst),
            1,
            "主动停止后监控不得启动替代服务"
        );
    }

    /// **`stop()` 终止的正是被记录的服务，且事后服务确实消失（#904 AC2b/AC3）。**
    #[tokio::test]
    async fn stop_terminates_the_recorded_service_once_and_the_process_is_gone() {
        let launches = Arc::new(AtomicUsize::new(0));
        let targets = Arc::new(Mutex::new(Vec::new()));
        let terminator = Arc::new(RecordingTerminator {
            targets: Arc::clone(&targets),
            inner: ChildTerminator,
        });
        let mut daemon = stub_daemon(launches, terminator);
        daemon.start().await.expect("桩服务启动成功");

        let pid = recorded_pid(&daemon);

        daemon.stop().await;

        assert_eq!(
            *targets.lock().unwrap_or_else(|e| e.into_inner()),
            vec![pid],
            "终止请求必须精确指名本 crate 记录的那个进程，且只此一次"
        );
        assert!(!daemon.is_running(), "守护进程已停止");
        assert!(daemon.get_rpc_client().is_none(), "停止后没有可寻址的实例");
        assert!(
            process_is_gone(pid),
            "服务进程必须真的消失，而不只是终止调用返回了 Ok：pid {pid}"
        );
    }

    /// **终止路径上的任何模块都不得按镜像名清扫进程（#904 AC4）。**
    ///
    /// #898 从启动路径移除了全机清扫，本增量又把终止收敛到「被记录的实例」；两件事都在源码层面
    /// 断言，且覆盖每一个可能把该形状带回来的文件。
    #[test]
    fn no_module_on_the_termination_path_sweeps_processes_by_image_name() {
        let process_killer = concat!("task", "kill");
        let image_name_match = concat!("/", "IM");
        for (name, source) in [
            ("process.rs", include_str!("process.rs")),
            ("daemon.rs", include_str!("daemon.rs")),
            ("types.rs", include_str!("types.rs")),
        ] {
            let code = code_without_comments(source);
            assert!(
                !code.contains(process_killer),
                "{name} 不得调用进程清扫器：{process_killer}"
            );
            assert!(
                !code.contains(image_name_match),
                "{name} 不得按镜像名选中进程：{image_name_match}"
            );
        }
    }
}
