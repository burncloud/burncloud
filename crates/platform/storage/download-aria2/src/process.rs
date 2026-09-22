use std::process::{Command, Stdio};
use std::time::Duration;

use reqwest::Client;
#[allow(clippy::disallowed_types)]
// Value used for aria2 JSON-RPC protocol construction/parsing
use serde_json::Value;

use crate::constants::{DEFAULT_PORT, MAX_PORT_RANGE};
use crate::error::{Aria2Error, Aria2Result};
use crate::types::{Aria2Config, Aria2Instance};

// ============================================================================
// 端口管理
// ============================================================================

/// 检查端口是否可用
// 输入：待检测的本地 TCP 端口号。
// 功能：尝试绑定本地回环地址以判断端口是否空闲。
// 错误：绑定失败时返回 false，不传播错误信息。
pub fn check_port_available(port: u16) -> bool {
    // 尝试绑定指定端口
    std::net::TcpListener::bind(("127.0.0.1", port)).is_ok()
}

/// 查找可用端口
// 输入：无。
// 功能：在默认端口及其后续范围内查找第一个可用端口。
// 错误：范围内没有可用端口时返回 PortError。
pub fn find_available_port() -> Aria2Result<u16> {
    // 逐个检测候选端口
    for port in DEFAULT_PORT..=(DEFAULT_PORT + MAX_PORT_RANGE) {
        if check_port_available(port) {
            return Ok(port);
        }
    }
    Err(Aria2Error::PortError("未找到可用端口".to_string()))
}

/// 终止所有aria2c.exe进程
// 输入：无。
// 功能：调用 taskkill 强制结束系统中的 aria2c.exe 进程。
// 错误：命令执行结果被忽略，不向调用方返回错误。
pub fn kill_existing_aria2() {
    // 执行 Windows 进程终止命令
    let _ = Command::new("taskkill")
        .args(["/F", "/IM", "aria2c.exe"])
        .output();
}

/// 启动 aria2 RPC 服务
// 输入：aria2 的运行配置。
// 功能：清理旧进程、启动新的 aria2 子进程并等待 RPC 服务就绪。
// 错误：端口选择、进程启动或 RPC 就绪等待失败时返回对应 Aria2Error。
pub async fn start_aria2_rpc(config: &Aria2Config) -> Aria2Result<Aria2Instance> {
    // 先终止现有的aria2c.exe进程
    kill_existing_aria2();

    // 选择 RPC 监听端口
    let port = find_available_port()?;

    // 根据配置构建 aria2 进程命令
    let mut cmd = Command::new(&config.aria2_path);
    cmd.args([
        "--enable-rpc",
        "--rpc-listen-all",
        &format!("--rpc-listen-port={}", port),
        &format!("--dir={}", config.download_dir.display()),
        &format!("--max-connection-per-server={}", config.max_connections),
        &format!("--split={}", config.max_connections),
        &format!("--min-split-size={}", config.split_size),
        "--continue=true",
        "--max-tries=0",
        "--retry-wait=3",
        "--daemon=true",
    ]);

    // 在配置了密钥时添加 RPC 认证参数
    if let Some(secret) = &config.secret {
        cmd.arg(format!("--rpc-secret={}", secret));
    }

    // 启动 aria2 子进程并接管标准输出与错误输出
    let child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;

    // 封装已启动进程的运行实例
    let instance = Aria2Instance {
        process: child,
        port,
        config: config.clone(),
    };

    // 等待 RPC 服务启动
    wait_for_rpc_ready(port, &config.secret).await?;

    Ok(instance)
}

// 输入：RPC 端口和可选认证密钥。
// 功能：轮询 aria2.getVersion，直到 RPC 服务可以正常响应。
// 错误：连续轮询超时时返回 RpcError。
async fn wait_for_rpc_ready(port: u16, secret: &Option<String>) -> Aria2Result<()> {
    // 创建健康检查客户端并构造 RPC 地址
    let client = Client::new();
    let url = format!("http://localhost:{}/jsonrpc", port);

    // 在限定次数内发送版本查询请求
    for _ in 0..30 {
        // 按需加入 RPC 认证令牌
        let mut params = vec![];
        if let Some(s) = secret {
            params.push(Value::String(format!("token:{}", s)));
        }

        // 构建 aria2 JSON-RPC 健康检查请求
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": "test",
            "method": "aria2.getVersion",
            "params": params
        });

        // 发送请求并检查服务状态
        if let Ok(response) = client.post(&url).json(&request).send().await {
            if response.status().is_success() {
                return Ok(());
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    // 所有轮询均失败后返回启动超时错误
    Err(Aria2Error::RpcError("RPC 服务启动超时".to_string()))
}
