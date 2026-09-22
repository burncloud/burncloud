use crate::daemon::Aria2Daemon;
use crate::downloader::download_aria2;
use crate::error::{Aria2Error, Aria2Result};
use crate::rpc::Aria2RpcClient;
use crate::types::Aria2Config;

// ============================================================================
// 统一管理器 - 主要入口点
// ============================================================================

pub struct Aria2Manager {
    daemon: Option<Aria2Daemon>,
    config: Aria2Config,
}

impl Aria2Manager {
    // 输入：无。
    // 功能：使用默认配置创建未启动的 aria2 统一管理器。
    // 错误：无，构造过程不执行外部操作。
    pub fn new() -> Self {
        // 初始化默认配置与空守护进程槽位
        Self {
            daemon: None,
            config: Aria2Config::default(),
        }
    }

    // 输入：调用方提供的 aria2 配置。
    // 功能：使用指定配置创建未启动的 aria2 统一管理器。
    // 错误：无，构造过程不执行外部操作。
    pub fn with_config(config: Aria2Config) -> Self {
        // 保存指定配置并初始化空守护进程槽位
        Self {
            daemon: None,
            config,
        }
    }

    /// 下载并设置 aria2
    // 输入：可变的统一管理器。
    // 功能：下载 aria2 二进制文件并更新管理器中的可执行文件路径。
    // 错误：下载或文件准备失败时返回 DownloadError。
    pub async fn download_and_setup(&mut self) -> Aria2Result<()> {
        // 下载 aria2 并记录得到的可执行文件路径
        println!("正在下载 aria2...");
        let aria2_path = download_aria2().await?;
        println!("aria2 已下载到: {:?}", aria2_path);

        // 更新后续守护进程启动使用的二进制路径
        self.config.aria2_path = aria2_path;
        Ok(())
    }

    /// 启动守护进程
    // 输入：可变的统一管理器。
    // 功能：创建并启动一个 aria2 守护进程。
    // 错误：守护进程已存在或启动失败时返回 DaemonError 或底层错误。
    pub async fn start_daemon(&mut self) -> Aria2Result<()> {
        // 确保管理器中不存在已有守护进程
        if self.daemon.is_some() {
            return Err(Aria2Error::DaemonError("守护进程已存在".to_string()));
        }

        // 创建、启动并保存新的守护进程
        let mut daemon = Aria2Daemon::new(self.config.clone());
        daemon.start().await?;
        self.daemon = Some(daemon);

        println!("aria2 守护进程启动成功！");
        Ok(())
    }

    /// 获取 RPC 客户端
    // 输入：统一管理器。
    // 功能：预留用于借用方式获取 RPC 客户端的接口。
    // 错误：无，当前实现始终返回 None。
    pub fn get_rpc_client(&self) -> Option<&Aria2RpcClient> {
        // 由于借用检查器限制，这里简化实现
        None
    }

    /// 创建新的 RPC 客户端
    // 输入：统一管理器。
    // 功能：委托守护进程按当前端口创建独立的 RPC 客户端。
    // 错误：无，守护进程不存在或未启动时返回 None。
    pub fn create_rpc_client(&self) -> Option<Aria2RpcClient> {
        // 从已启动的守护进程中创建客户端
        self.daemon.as_ref().and_then(|d| d.get_rpc_client())
    }

    /// 关闭管理器
    // 输入：可变的统一管理器。
    // 功能：停止守护进程、清理其引用并完成管理器关闭。
    // 错误：守护进程停止内部错误被忽略，函数当前始终返回 Ok。
    pub async fn shutdown(&mut self) -> Aria2Result<()> {
        // 停止已存在的守护进程
        if let Some(ref mut daemon) = self.daemon {
            daemon.stop().await;
        }
        // 清除守护进程引用
        self.daemon = None;
        println!("Aria2Manager 已关闭");
        Ok(())
    }

    /// 检查是否运行中
    // 输入：统一管理器。
    // 功能：检查内部守护进程是否处于运行状态。
    // 错误：无，管理器未持有守护进程时返回 false。
    pub fn is_running(&self) -> bool {
        // 委托当前守护进程读取运行状态
        self.daemon.as_ref().is_some_and(|d| d.is_running())
    }
}

impl Default for Aria2Manager {
    // 输入：无。
    // 功能：创建默认配置的 aria2 统一管理器。
    // 错误：无，构造过程不执行外部操作。
    fn default() -> Self {
        // 复用标准构造函数
        Self::new()
    }
}

// ============================================================================
// 便利函数
// ============================================================================

/// 快速启动 aria2 管理器
// 输入：无。
// 功能：下载 aria2、启动守护进程并返回已就绪的管理器。
// 错误：下载或守护进程启动失败时返回对应 Aria2Error。
pub async fn quick_start() -> Aria2Result<Aria2Manager> {
    // 创建管理器并完成下载配置
    let mut manager = Aria2Manager::new();
    manager.download_and_setup().await?;
    // 启动守护进程并返回管理器
    manager.start_daemon().await?;
    Ok(manager)
}
