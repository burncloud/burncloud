use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::{Aria2Error, Aria2Result};
use crate::process::start_aria2_rpc;
use crate::rpc::Aria2RpcClient;
use crate::types::{Aria2Config, Aria2Instance};

// ============================================================================
// 简单守护进程
// ============================================================================

pub struct Aria2Daemon {
    instance: Arc<Mutex<Option<Aria2Instance>>>,
    config: Aria2Config,
    is_running: Arc<AtomicBool>,
}

impl Aria2Daemon {
    // 输入：aria2 运行配置。
    // 功能：创建未启动的 aria2 守护进程管理对象。
    // 错误：无，构造过程不执行外部操作。
    pub fn new(config: Aria2Config) -> Self {
        // 初始化进程槽位、配置和运行标记
        Self {
            instance: Arc::new(Mutex::new(None)),
            config,
            is_running: Arc::new(AtomicBool::new(false)),
        }
    }

    // 输入：可变的守护进程管理对象。
    // 功能：启动 aria2 服务并创建自动重启监控任务。
    // 错误：重复启动、服务启动或重启失败时返回对应 Aria2Error。
    pub async fn start(&mut self) -> Aria2Result<()> {
        // 防止重复启动守护进程
        if self.is_running.load(Ordering::SeqCst) {
            return Err(Aria2Error::DaemonError("守护进程已在运行".to_string()));
        }

        // 启动 aria2 RPC 服务并保存运行实例
        let instance = start_aria2_rpc(&self.config).await?;
        println!("aria2 RPC 服务已启动在端口: {}", instance.port);

        *self.instance.lock().unwrap_or_else(|e| e.into_inner()) = Some(instance);
        self.is_running.store(true, Ordering::SeqCst);

        // 启动监控任务
        let instance = Arc::clone(&self.instance);
        let is_running = Arc::clone(&self.is_running);
        let config = self.config.clone();

        // 异步监测进程状态并在退出后尝试重启
        tokio::spawn(async move {
            while is_running.load(Ordering::SeqCst) {
                tokio::time::sleep(Duration::from_millis(1000)).await;

                // 在互斥锁保护下检查当前进程状态
                let need_restart = {
                    let mut lock = instance.lock().unwrap_or_else(|e| e.into_inner());
                    match lock.as_mut() {
                        Some(inst) => !inst.is_running(), // 检查进程是否还在运行
                        None => true,
                    }
                };

                // 进程退出时创建并替换新的运行实例
                if need_restart {
                    println!("检测到aria2已退出，重启中...");
                    if let Ok(new_instance) = start_aria2_rpc(&config).await {
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
    // 功能：停止监控任务并终止当前 aria2 子进程。
    // 错误：进程终止错误被忽略，不向调用方返回错误。
    pub async fn stop(&mut self) {
        // 更新运行标记以结束监控循环
        self.is_running.store(false, Ordering::SeqCst);

        // 终止当前保存的 aria2 进程
        if let Some(ref mut instance) = self
            .instance
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .as_mut()
        {
            let _ = instance.kill();
        }

        // 清除已停止的运行实例
        *self.instance.lock().unwrap_or_else(|e| e.into_inner()) = None;
        println!("aria2 守护进程已停止");
    }

    // 输入：守护进程管理对象。
    // 功能：根据当前实例端口创建一个新的 RPC 客户端。
    // 错误：无，守护进程尚未启动时返回 None。
    pub fn get_rpc_client(&self) -> Option<Aria2RpcClient> {
        // 读取当前实例并映射为 RPC 客户端
        let lock = self.instance.lock().unwrap_or_else(|e| e.into_inner());
        lock.as_ref()
            .map(|instance| Aria2RpcClient::new(instance.port, self.config.secret.clone()))
    }

    // 输入：守护进程管理对象。
    // 功能：读取守护进程的运行状态标记。
    // 错误：无。
    pub fn is_running(&self) -> bool {
        // 返回原子运行标记的当前值
        self.is_running.load(Ordering::SeqCst)
    }
}
