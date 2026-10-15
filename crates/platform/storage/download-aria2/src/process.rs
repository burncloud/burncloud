use std::process::{Command, Stdio};
use std::time::Duration;

use reqwest::Client;
#[allow(
    clippy::disallowed_types,
    reason = "aria2 JSON-RPC parameters are intentionally represented as dynamic serde_json::Value values"
)]
use serde_json::Value;

use crate::constants::{DEFAULT_PORT, MAX_PORT_RANGE};
use crate::error::{Aria2Error, Aria2Result};
use crate::rpc::Aria2RpcClient;
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
    // The sweep is deliberately best-effort in the current design. Explicitly drop the Result so a failed
    // `taskkill` remains non-fatal without hiding a #[must_use] value behind `let _ = ...`.
    drop(
        Command::new("taskkill")
            .args(["/F", "/IM", "aria2c.exe"])
            .output(),
    );
}

pub(crate) async fn find_running_aria2(secret: &Option<String>) -> Option<u16> {
    for port in DEFAULT_PORT..=DEFAULT_PORT + MAX_PORT_RANGE {
        if check_port_available(port) {
            continue;
        }
        let client = Aria2RpcClient::new(port, secret.clone());
        let responds = tokio::time::timeout(Duration::from_secs(1), client.tell_active())
            .await
            .is_ok_and(|result| result.is_ok());
        if responds {
            return Some(port);
        }
    }
    None
}

/// 启动 aria2 RPC 服务
// 输入：aria2 的运行配置。
// 功能：启动新的 aria2 子进程并等待 RPC 服务就绪。
// 错误：端口选择、进程启动或 RPC 就绪等待失败时返回对应 Aria2Error。
pub async fn start_aria2_rpc(config: &Aria2Config) -> Aria2Result<Aria2Instance> {
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

#[cfg(test)]
mod tests {
    use super::{check_port_available, find_available_port};
    use crate::constants::{DEFAULT_PORT, MAX_PORT_RANGE};

    /// A port that something is listening on is reported unavailable, and the same port is available once the
    /// listener is gone.
    ///
    /// Both directions matter: a function that always returned `true` would let `find_available_port` hand out a
    /// port already in use, and one that always returned `false` would make it fail on an idle machine.
    #[test]
    fn a_bound_port_is_unavailable_and_becomes_available_when_released() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).expect("an ephemeral port");
        let port = listener.local_addr().expect("a local address").port();

        assert!(
            !check_port_available(port),
            "port {port} is bound by this test, so it must report as unavailable"
        );

        drop(listener);

        // Reported as available again. A short retry, because the OS releases the socket asynchronously on some
        // platforms and asserting immediately can be a race rather than a fact.
        let mut available = false;
        for _ in 0..50 {
            if check_port_available(port) {
                available = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        println!("port {port} available after release: {available}");
        assert!(
            available,
            "the port must be available once the listener is dropped"
        );
    }

    /// `find_available_port` returns a free port **inside the configured range**, and the port it returns can
    /// actually be bound.
    ///
    /// The second half is the part worth asserting: a function that returned a port without checking, or checked
    /// a different one, would still return a number in range.
    #[test]
    fn the_port_found_is_in_range_and_free() {
        let port = find_available_port().expect("an idle machine has a free port in the range");
        println!("find_available_port -> {port}");

        assert!(
            (DEFAULT_PORT..=DEFAULT_PORT + MAX_PORT_RANGE).contains(&port),
            "the port must be within {DEFAULT_PORT}..={}, got {port}",
            DEFAULT_PORT + MAX_PORT_RANGE
        );

        // Binding it proves nothing else holds it, which is what "available" is supposed to mean. If this fails
        // the check and the search disagreed.
        let listener = std::net::TcpListener::bind(("127.0.0.1", port));
        assert!(
            listener.is_ok(),
            "find_available_port returned {port} but it cannot be bound: {:?}",
            listener.err()
        );
    }

    /// The search prefers the **first** free port in the range, so the default is used when it is free.
    ///
    /// Recorded as a property of the search rather than a requirement: the loop starts at `DEFAULT_PORT` and
    /// returns the first candidate that binds. A test that only checked "some free port" would accept a search
    /// that scanned backwards, which would move the daemon off its documented default on every launch.
    #[test]
    fn the_search_starts_at_the_default_port() {
        // Only meaningful when the default port is free, which it is on a machine running no aria2.
        if !check_port_available(DEFAULT_PORT) {
            println!(
                "port {DEFAULT_PORT} is in use, so the starting point cannot be observed here"
            );
            return;
        }

        let port = find_available_port().expect("a free port");
        println!("default {DEFAULT_PORT} is free; find_available_port -> {port}");
        assert_eq!(
            port, DEFAULT_PORT,
            "with the default free, the search must return it rather than a later candidate"
        );
    }

    /// `start_aria2_rpc` 不再调用全局进程清理函数。
    ///
    /// `kill_existing_aria2` runs `taskkill /F /IM aria2c.exe`, which terminates **every** aria2 process on the
    /// machine, and `start_aria2_rpc` calls it unconditionally before starting its own. The plan lists "只终止
    /// 自己启动的进程"; this is the opposite -- it terminates processes this program did not start, so a user's
    /// own downloads are killed when the application launches.
    ///
    /// There is no assertion to make without killing something: calling the function in a test would terminate
    /// real aria2 processes on the machine running the tests, which is exactly the behaviour being reported. So
    /// the test asserts the *shape of the code* instead -- that the function is reached from the start path --
    /// by reading the source, which is a weaker check than executing it and the only one available.
    ///
    /// The proper fix is to track the child process and terminate only that one, which is what `Aria2Daemon::stop`
    /// already does through `self.instance`. The initial sweep is the part that cannot distinguish "a process I
    /// left behind" from "a process the user started".
    #[test]
    fn starting_a_new_rpc_service_does_not_sweep_other_aria2_processes() {
        // Read the source of this module, so the assertion is about the shipped code rather than about a copy of
        // its behaviour.
        let source = include_str!("process.rs");
        let Some(start) = source.find("pub async fn start_aria2_rpc") else {
            panic!("找不到 start_aria2_rpc 定义");
        };
        let body = &source[start..];
        let body = body.split("\n}\n\n// 输入：").next().unwrap_or(body);
        assert!(!body.contains("kill_existing_aria2"));
    }
}
