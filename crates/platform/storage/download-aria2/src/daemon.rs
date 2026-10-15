use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::error::{Aria2Error, Aria2Result};
use crate::process::{find_running_aria2, start_aria2_rpc};
use crate::rpc::Aria2RpcClient;
use crate::types::{Aria2Config, Aria2Instance};
use tokio::sync::Mutex as AsyncMutex;

// ============================================================================
// 简单守护进程
// ============================================================================

pub struct Aria2Daemon {
    instance: Arc<Mutex<Option<Aria2Instance>>>,
    attached_port: Arc<Mutex<Option<u16>>>,
    config: Aria2Config,
    is_running: Arc<AtomicBool>,
    add_uri_lock: Arc<AsyncMutex<()>>,
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

        let instance = start_aria2_rpc(&self.config).await?;
        println!("aria2 RPC 服务已启动在端口: {}", instance.port);
        *self
            .instance
            .lock()
            .unwrap_or_else(|error| error.into_inner()) = Some(instance);
        self.is_running.store(true, Ordering::SeqCst);
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
            drop(instance.kill());
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
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}
