# daemon.rs 数据结构

## 公有数据结构

### Aria2Daemon

#### 完整定义
```rust
pub struct Aria2Daemon {
    instance: Arc<Mutex<Option<Aria2Instance>>>,
    config: Aria2Config,
    is_running: Arc<AtomicBool>,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | 信息 |
|---|---|---|---|
| `instance` | `Arc<Mutex<Option<Aria2Instance>>>` | 私有 | 线程安全地保存当前 aria2 运行实例；未启动或已停止时为 `None`。 |
| `config` | `Aria2Config` | 私有 | aria2 启动和重启使用的配置。 |
| `is_running` | `Arc<AtomicBool>` | 私有 | 线程安全的守护进程运行状态标记。 |

## 私有数据结构

无。
