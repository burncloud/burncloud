# types.rs 数据结构

## 公有数据结构

### Aria2Config

#### 完整定义
```rust
#[derive(Debug, Clone)]
pub struct Aria2Config {
    pub port: u16,
    pub secret: Option<String>,
    pub download_dir: PathBuf,
    pub max_connections: u8,
    pub split_size: String,
    pub aria2_path: PathBuf,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | 信息 |
|---|---|---|---|
| `port` | `u16` | `pub` | aria2 RPC 服务监听端口。 |
| `secret` | `Option<String>` | `pub` | 可选的 RPC 认证密钥；`None` 表示未配置。 |
| `download_dir` | `PathBuf` | `pub` | 下载文件保存目录。 |
| `max_connections` | `u8` | `pub` | 每个服务器的最大连接数，同时用于构造 aria2 的分片数参数。 |
| `split_size` | `String` | `pub` | aria2 分片最小大小配置，例如 `"1M"`。 |
| `aria2_path` | `PathBuf` | `pub` | aria2 可执行文件路径。 |

### DownloadOptions

#### 完整定义
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub split: Option<u8>,
    #[serde(
        rename = "max-connection-per-server",
        skip_serializing_if = "Option::is_none"
    )]
    pub max_connection_per_server: Option<u8>,
    #[serde(rename = "continue", skip_serializing_if = "Option::is_none")]
    pub continue_download: Option<bool>,
    #[serde(rename = "allow-overwrite", skip_serializing_if = "Option::is_none")]
    pub allow_overwrite: Option<bool>,
    #[serde(rename = "auto-file-renaming", skip_serializing_if = "Option::is_none")]
    pub auto_file_renaming: Option<bool>,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | Serde 配置 | 信息 |
|---|---|---|---|---|
| `dir` | `Option<String>` | `pub` | `None` 时跳过序列化 | 下载目录。 |
| `out` | `Option<String>` | `pub` | `None` 时跳过序列化 | 输出文件名。 |
| `split` | `Option<u8>` | `pub` | `None` 时跳过序列化 | 分片数量。 |
| `max_connection_per_server` | `Option<u8>` | `pub` | 序列化名称为 `max-connection-per-server`，`None` 时跳过 | 每个服务器的最大连接数。 |
| `continue_download` | `Option<bool>` | `pub` | 序列化名称为 `continue`，`None` 时跳过 | 是否续传。 |
| `allow_overwrite` | `Option<bool>` | `pub` | 序列化名称为 `allow-overwrite`，`None` 时跳过 | 是否允许覆盖已存在的文件。 |
| `auto_file_renaming` | `Option<bool>` | `pub` | 序列化名称为 `auto-file-renaming`，`None` 时跳过 | 是否允许文件自动重命名。 |

### DownloadStatus

#### 完整定义
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct DownloadStatus {
    pub gid: String,
    pub status: String,
    #[serde(rename = "totalLength")]
    pub total_length: String,
    #[serde(rename = "completedLength")]
    pub completed_length: String,
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | Serde 配置 | 信息 |
|---|---|---|---|---|
| `gid` | `String` | `pub` | 无 | 下载任务标识。 |
| `status` | `String` | `pub` | 无 | 下载状态。 |
| `total_length` | `String` | `pub` | 反序列化字段名为 `totalLength` | 文件总大小。 |
| `completed_length` | `String` | `pub` | 反序列化字段名为 `completedLength` | 已完成大小。 |
| `download_speed` | `String` | `pub` | 反序列化字段名为 `downloadSpeed` | 下载速度。 |

### GlobalStat

#### 完整定义
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct GlobalStat {
    #[serde(rename = "downloadSpeed")]
    pub download_speed: String,
    #[serde(rename = "numActive")]
    pub num_active: String,
    #[serde(rename = "numWaiting")]
    pub num_waiting: String,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | Serde 配置 | 信息 |
|---|---|---|---|---|
| `download_speed` | `String` | `pub` | 反序列化字段名为 `downloadSpeed` | 全局下载速度。 |
| `num_active` | `String` | `pub` | 反序列化字段名为 `numActive` | 活跃任务数量。 |
| `num_waiting` | `String` | `pub` | 反序列化字段名为 `numWaiting` | 等待任务数量。 |

### FileInfo

#### 完整定义
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub uris: Vec<UriInfo>,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | 信息 |
|---|---|---|---|
| `path` | `String` | `pub` | 文件路径。 |
| `uris` | `Vec<UriInfo>` | `pub` | 与文件关联的 URI 信息列表。 |

### UriInfo

#### 完整定义
```rust
#[derive(Debug, Clone, Deserialize)]
pub struct UriInfo {
    pub uri: String,
    pub status: String,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | 信息 |
|---|---|---|---|
| `uri` | `String` | `pub` | 资源 URI。 |
| `status` | `String` | `pub` | URI 下载状态。 |

### Aria2Instance

#### 完整定义
```rust
pub struct Aria2Instance {
    pub process: Child,
    pub port: u16,
    pub config: Aria2Config,
}
```

#### 内部参数
| 参数 | 类型 | 可见性 | 信息 |
|---|---|---|---|
| `process` | `Child` | `pub` | aria2 子进程句柄。 |
| `port` | `u16` | `pub` | 当前 aria2 RPC 服务端口。 |
| `config` | `Aria2Config` | `pub` | 启动该实例时使用的配置副本。 |

## 私有数据结构

无。
