# error.rs 数据结构

## 公有数据结构

### Aria2Error

#### 完整定义
```rust
#[derive(Debug)]
pub enum Aria2Error {
    DownloadError(String),
    PortError(String),
    RpcError(String),
    DaemonError(String),
    ProcessError(String),
    ConfigError(String),
}
```

#### 内部参数
| 变体 | 关联数据 | 信息 |
|---|---|---|
| `DownloadError` | `String` | 下载、文件准备或解压相关错误信息。 |
| `PortError` | `String` | 端口查找或端口相关错误信息。 |
| `RpcError` | `String` | RPC 请求、响应或服务端错误信息。 |
| `DaemonError` | `String` | 守护进程状态或操作错误信息。 |
| `ProcessError` | `String` | 子进程启动、终止或等待错误信息。 |
| `ConfigError` | `String` | 配置相关错误信息。 |

## 私有数据结构

无。
