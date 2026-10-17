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

/// 启动失败时终止本次启动的 aria2 子进程。
// 输入：本次启动的子进程 PID、终止该进程的动作、诊断信息的接收方。
// 功能：执行终止动作；终止失败时把「哪个进程可能还在运行」交给报告方，而不是静默丢弃（#898）。
// 错误：不向调用方返回错误 —— 调用方已持有启动失败的真实原因，替换它会掩盖原因本身。
fn terminate_after_failed_start(
    pid: u32,
    terminate: impl FnOnce() -> Aria2Result<()>,
    report: impl FnOnce(String),
) {
    if let Err(error) = terminate() {
        let message = format!("aria2 子进程 {pid} 在启动失败后未能终止，可能仍在运行：{error}");
        report(message);
    }
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
// 说明：启动前**不再**清扫机器上的其他 aria2 进程（#898）。端口是本次启动自己挑的空闲端口，
// 因此别的 aria2 占着默认端口只意味着换一个端口，既不需要也无权先把它终止掉。
pub async fn start_aria2_rpc(config: &Aria2Config) -> Aria2Result<Aria2Instance> {
    // 选择 RPC 监听端口
    let port = find_available_port()?;

    // 根据配置构建 aria2 进程命令。
    //
    // **不传 `--daemon=true`**（#904）：该标志让 aria2 立刻 fork 出守护进程、直接子进程随即退出。
    // `Aria2Instance` 记录的正是那个直接子进程，于是启动成功后 `is_running()` 恒为 `false` ⇒ 重启监控
    // 每个 tick 都再起一个实例（端口递增、无界累积），而 `stop()` 只能终止一个早已退出的句柄。旧实现用
    // 按镜像名的全机清扫掩盖了这个后果，#898 删除 sweep 后才暴露。让服务**就是**直接子进程，从结构上消除
    // 「服务 ≠ 子进程」，也就不必先确定某个平台是否真的 detach（#904 的 BLOCK-1）。
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
    ]);

    // 在配置了密钥时添加 RPC 认证参数
    if let Some(secret) = &config.secret {
        cmd.arg(format!("--rpc-secret={}", secret));
    }

    // 启动 aria2 子进程，并丢弃它的输出。
    //
    // 这里**不能**用 `Stdio::piped()`：管道只有被读取才会排空，而本 crate 从不读取它，于是 aria2 写满
    // 管道缓冲（约 64 KiB）后会阻塞在 write 上、整个服务停住 —— 过去直接子进程立即退出、由分离出去的
    // 守护进程继承写端，这一点才没有暴露。服务现在就是直接子进程，必须让它一直写得下去；而这些输出本来
    // 也从未被读取，丢弃不损失任何既有信息。
    let child = cmd
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| Aria2Error::ProcessError(e.to_string()))?;

    // 封装已启动进程的运行实例
    let mut instance = Aria2Instance {
        process: child,
        port,
        config: config.clone(),
    };

    // 等待 RPC 服务启动。失败时终止本次启动的子进程：它是本程序唯一有权终止的进程
    // （`Aria2Daemon::stop` 走的是同一个 `Aria2Instance::kill`），终止失败也要报告出来，
    // 而不是留下一个无人负责、又不被记录的 aria2。
    if let Err(error) = wait_for_rpc_ready(port, &config.secret).await {
        let pid = instance.process.id();
        // 既然不再读取子进程的输出，它退出时的状态就是唯一还留在手上的诊断信息：启动失败最常见的原因
        // （可执行文件缺失、参数被拒）会让 aria2 立刻退出，而「RPC 就绪超时」本身过于笼统。这里只把状态
        // 说出来，不替换真正的失败原因。
        if let Ok(Some(status)) = instance.process.try_wait() {
            eprintln!("aria2 子进程 {pid} 在 RPC 就绪前已退出，退出状态：{status}");
        }
        terminate_after_failed_start(pid, || instance.kill(), |message| eprintln!("{message}"));
        return Err(error);
    }

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
    use super::{check_port_available, find_available_port, terminate_after_failed_start};
    use crate::constants::{DEFAULT_PORT, MAX_PORT_RANGE};
    use crate::error::Aria2Error;

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
    /// The previous implementation ran the image-name form of the process killer (`taskkill /F /IM aria2c.exe`)
    /// before every start, so it terminated **every** aria2 process on the machine -- a user's own downloads, or
    /// another component's instance. What replaced it is not a narrower sweep: `find_available_port` already picks
    /// a free port, so a leftover process on the default port does not conflict with the new one, and the only
    /// process this crate may terminate is the child it started itself. The assertion is therefore about absence.
    ///
    /// Only **code** lines are scanned, because the documentation above has to be able to name what was removed
    /// in order to explain why it is gone. The needles are assembled with `concat!` so that this test's own text
    /// cannot satisfy the positive form of either check.
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
