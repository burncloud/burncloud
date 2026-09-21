# rpc.rs 数据结构

## 公有数据结构

### Aria2RpcClient

#### 完整定义
```rust
pub struct Aria2RpcClient {
    client: Client,
    base_url: String,
    secret: Option<String>,
    request_id: Arc<AtomicU64>,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | 信息 |
|---|---|---|---|
| `client` | `Client` | 私有 | 用于发送 aria2 JSON-RPC HTTP 请求的客户端。 |
| `base_url` | `String` | 私有 | JSON-RPC 服务地址，格式为 `http://localhost:<port>/jsonrpc`。 |
| `secret` | `Option<String>` | 私有 | 可选 RPC 认证密钥。 |
| `request_id` | `Arc<AtomicU64>` | 私有 | 线程安全的 JSON-RPC 请求编号生成器。 |

## 私有数据结构

无。
