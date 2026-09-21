# manager.rs 数据结构

## 公有数据结构

### Aria2Manager

#### 完整定义
```rust
pub struct Aria2Manager {
    daemon: Option<Aria2Daemon>,
    config: Aria2Config,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | 信息 |
|---|---|---|---|
| `daemon` | `Option<Aria2Daemon>` | 私有 | 当前管理器持有的守护进程；未启动时为 `None`。 |
| `config` | `Aria2Config` | 私有 | 管理器用于下载和启动 aria2 的配置。 |

## 私有数据结构

无。
