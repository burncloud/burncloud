# manager.rs 数据结构

## 公有数据结构

### DownloadManager

#### 完整定义
```rust
pub struct DownloadManager {
    pub(crate) aria2: Arc<Aria2Manager>,
    pub(crate) db: Arc<DownloadDB>,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | 信息 |
|---|---|---|---|
| `aria2` | `Arc<Aria2Manager>` | `pub(crate)` | 线程安全引用计数包装的 aria2 管理器，用于创建 RPC 客户端和执行下载任务操作。 |
| `db` | `Arc<DownloadDB>` | `pub(crate)` | 线程安全引用计数包装的下载数据库，用于保存、更新和删除下载记录。 |

## 私有数据结构

无。
